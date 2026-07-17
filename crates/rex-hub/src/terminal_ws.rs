//! WebSocket 终端桥接 — 浏览器 ↔ Hub ↔ SSH 服务器 / Agent。
//!
//! 统一入口：/ws/terminal?token=jwt&resourceId=xxx
//! Hub 从 DB 读取资源连接信息，自动判断直连或 Agent 隧道。
//! 前端完全不感知底层连接方式。

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use rex_ssh::{SshConfig, SshSession, TerminalEvent};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::agent_ws::{AgentEvent, ConnectResponse};
use crate::AppState;

/// 前端 → 后端的消息（连接建立后的控制消息）
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMsg {
    #[serde(rename = "terminal.data")]
    Data { data: String },
    #[serde(rename = "terminal.resize")]
    Resize { cols: u32, rows: u32 },
    #[serde(rename = "terminal.disconnect")]
    Disconnect,
}

/// 后端 → 前端的消息
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMsg {
    #[serde(rename = "terminal.connected")]
    Connected { payload: ConnectedPayload },
    #[serde(rename = "terminal.data")]
    Data { payload: DataPayload },
    #[serde(rename = "terminal.disconnected")]
    Disconnected { payload: DisconnectedPayload },
    #[serde(rename = "terminal.error")]
    Error { payload: ErrorPayload },
}

#[derive(Debug, Serialize)]
struct ConnectedPayload {
    #[serde(rename = "sessionId")]
    session_id: String,
}

#[derive(Debug, Serialize)]
struct DataPayload {
    data: String,
}

#[derive(Debug, Serialize)]
struct DisconnectedPayload {
    reason: String,
}

#[derive(Debug, Serialize)]
struct ErrorPayload {
    message: String,
}

/// URL 查询参数
#[derive(Deserialize)]
pub struct TerminalQuery {
    #[serde(rename = "resourceId")]
    pub resource_id: String,
}

/// 资源连接信息
struct ResourceConnInfo {
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    private_key: Option<String>,
    use_agent: bool,
    agent_id: Option<String>,
}

/// GET /ws/terminal?token=jwt&resourceId=xxx
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<TerminalQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, query.resource_id))
}

/// WebSocket 连接处理主循环
async fn handle_socket(mut ws: WebSocket, state: AppState, resource_id: String) {
    let session_id = format!("sess_{}", &uuid::Uuid::new_v4().to_string()[..8]);

    // 从 DB 读取资源连接信息（含解密密码）
    let conn_info = match load_resource_conn(&state, &resource_id).await {
        Ok(info) => info,
        Err(e) => {
            let _ = send_ws_error(&mut ws, &e).await;
            return;
        }
    };

    if conn_info.use_agent {
        handle_agent_terminal(ws, &state, &conn_info, &resource_id, &session_id).await;
    } else {
        handle_direct_terminal(ws, &conn_info, &session_id).await;
    }
}

/// 从 DB 读取资源连接信息（解密 config_json）
async fn load_resource_conn(
    state: &AppState,
    resource_id: &str,
) -> Result<ResourceConnInfo, String> {
    let db = state.db.clone();
    let rid = resource_id.to_string();
    let crypto = state.crypto.clone();

    tokio::task::spawn_blocking(move || {
        // 获取资源
        let resource = db
            .get_resource(&rid)
            .map_err(|e| format!("db error: {e}"))?
            .ok_or_else(|| "resource not found".to_string())?;

        // 解密 config_json
        let config_str = if !resource.config_json.is_empty() && resource.config_json != "{}" {
            crypto
                .decrypt(&resource.config_json)
                .map_err(|e| format!("decrypt failed: {e}"))?
        } else {
            resource.config_json.clone()
        };

        let config: serde_json::Value =
            serde_json::from_str(&config_str).map_err(|e| format!("invalid config json: {e}"))?;

        let host = config
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(22) as u16;
        let username = config
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let password = config
            .get("password")
            .and_then(|v| v.as_str())
            .map(String::from);
        let private_key = config
            .get("privateKey")
            .and_then(|v| v.as_str())
            .map(String::from);

        // 获取环境信息
        let env = db
            .get_environment(&resource.environment_id)
            .map_err(|e| format!("db error: {e}"))?
            .ok_or_else(|| "environment not found".to_string())?;

        let use_agent = env.connection_mode == "agent";

        let agent_id = if use_agent {
            let agents = db
                .list_agents_by_env(&resource.environment_id)
                .unwrap_or_default();
            agents
                .into_iter()
                .find(|a| a.status == "online")
                .map(|a| a.id)
        } else {
            None
        };

        Ok(ResourceConnInfo {
            host,
            port,
            username,
            password,
            private_key,
            use_agent,
            agent_id,
        })
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

// ═══════════════════════════════════════
// 直连模式
// ═══════════════════════════════════════

async fn handle_direct_terminal(mut ws: WebSocket, conn: &ResourceConnInfo, session_id: &str) {
    let config = SshConfig {
        host: conn.host.clone(),
        port: conn.port,
        username: conn.username.clone(),
        password: conn.password.clone(),
        private_key: conn.private_key.clone(),
    };

    let session = match SshSession::connect(config).await {
        Ok(s) => s,
        Err(e) => {
            let _ = send_ws_error(&mut ws, &format!("SSH connection failed: {e}")).await;
            return;
        }
    };

    let _ = ws
        .send(Message::Text(
            serde_json::to_string(&ServerMsg::Connected {
                payload: ConnectedPayload {
                    session_id: session_id.to_string(),
                },
            })
            .unwrap(),
        ))
        .await;

    let session = Arc::new(Mutex::new(session));
    let (mut ws_sink, mut ws_stream) = ws.split();
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<ClientMsg>(64);
    let (data_tx, mut data_rx) = mpsc::channel::<String>(512);

    let ws_read_task = tokio::spawn(async move {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) {
                        let is_disconnect = matches!(client_msg, ClientMsg::Disconnect);
                        if cmd_tx.send(client_msg).await.is_err() {
                            break;
                        }
                        if is_disconnect {
                            break;
                        }
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    });

    let session_for_ssh = session.clone();
    let ssh_task = tokio::spawn(async move {
        loop {
            let mut session = session_for_ssh.lock().await;
            tokio::select! {
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(ClientMsg::Data { data }) => {
                            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&data) {
                                let _ = session.send_data(bytes::Bytes::from(bytes)).await;
                            }
                        }
                        Some(ClientMsg::Resize { cols, rows }) => {
                            let _ = session.resize(cols, rows).await;
                        }
                        Some(ClientMsg::Disconnect) | None => {
                            let _ = session.disconnect().await;
                            break;
                        }
                    }
                }
                event = session.recv() => {
                    drop(session);
                    match event {
                        Some(TerminalEvent::Data(data)) => {
                            let encoded = base64::engine::general_purpose::STANDARD.encode(data.as_bytes());
                            if data_tx.send(encoded).await.is_err() {
                                break;
                            }
                        }
                        Some(TerminalEvent::Disconnected(reason)) => {
                            let msg = ServerMsg::Disconnected {
                                payload: DisconnectedPayload { reason },
                            };
                            let _ = data_tx.send(serde_json::to_string(&msg).unwrap_or_default()).await;
                            break;
                        }
                        None => break,
                    }
                    continue;
                }
            }
            drop(session);
        }
    });

    let ws_write_task = tokio::spawn(async move {
        while let Some(data) = data_rx.recv().await {
            let msg = if data.starts_with('{') {
                Message::Text(data)
            } else {
                let wrapped = ServerMsg::Data {
                    payload: DataPayload { data },
                };
                Message::Text(serde_json::to_string(&wrapped).unwrap())
            };
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    tokio::select! {
        _ = ws_read_task => {},
        _ = ssh_task => {},
        _ = ws_write_task => {},
    }

    tracing::debug!(session_id, "terminal session ended");
}

// ═══════════════════════════════════════
// Agent 隧道模式
// ═══════════════════════════════════════

async fn handle_agent_terminal(
    mut ws: WebSocket,
    state: &AppState,
    conn: &ResourceConnInfo,
    resource_id: &str,
    session_id: &str,
) {
    let agent_id = match conn.agent_id.as_ref() {
        Some(id) => id.clone(),
        None => {
            let _ = send_ws_error(&mut ws, "no online agent for this environment").await;
            return;
        }
    };

    let agent_conn = {
        let conns = state.agent_tunnel.connections.read().await;
        conns.get(&agent_id).cloned()
    };

    let agent_conn = match agent_conn {
        Some(c) => c,
        None => {
            let _ = send_ws_error(&mut ws, "agent not connected").await;
            return;
        }
    };

    // 发送 connect 到 Agent
    let request_id = format!("req_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let (resp_tx, resp_rx) = oneshot::channel();

    {
        let mut pending = state.agent_tunnel.pending_requests.write().await;
        pending.insert(request_id.clone(), resp_tx);
    }

    let connect_msg = serde_json::json!({
        "type": "connect",
        "payload": {
            "request_id": request_id,
            "resource_id": resource_id,
            "protocol": "ssh",
            "config": {
                "host": conn.host,
                "port": conn.port,
                "username": conn.username,
                "password": conn.password,
                "privateKey": conn.private_key,
            }
        }
    });

    if agent_conn
        .sender
        .send(AgentEvent::Text(connect_msg.to_string()))
        .await
        .is_err()
    {
        let _ = send_ws_error(&mut ws, "failed to contact agent").await;
        return;
    }

    // 等待 Agent 响应
    let channel_id = match tokio::time::timeout(std::time::Duration::from_secs(10), resp_rx).await {
        Ok(Ok(ConnectResponse {
            channel_id: Some(id),
            ..
        })) => id,
        Ok(Ok(ConnectResponse { error: Some(e), .. })) => {
            let _ = send_ws_error(&mut ws, &e).await;
            return;
        }
        _ => {
            let _ = send_ws_error(&mut ws, "agent connection timeout").await;
            return;
        }
    };

    tracing::info!(channel_id = %channel_id, session_id = %session_id, "agent terminal connected");

    // 通知前端连接成功
    let _ = ws
        .send(Message::Text(
            serde_json::to_string(&ServerMsg::Connected {
                payload: ConnectedPayload {
                    session_id: session_id.to_string(),
                },
            })
            .unwrap(),
        ))
        .await;

    // 注册 tunnel data channel
    let (data_tx, mut data_rx) = mpsc::channel::<Vec<u8>>(512);
    {
        let mut tunnel_data = state.agent_tunnel.tunnel_data.write().await;
        tunnel_data.insert(channel_id.clone(), data_tx);
    }

    // 桥接：前端 ↔ Agent
    let (mut ws_sink, mut ws_stream) = ws.split();
    let ch_id_num = channel_id.parse::<u32>().unwrap_or(0);

    let agent_for_send = agent_conn.clone();
    let channel_id_clone = channel_id.clone();
    let frontend_to_agent = tokio::spawn(async move {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) {
                        match client_msg {
                            ClientMsg::Data { data } => {
                                if let Ok(decoded) =
                                    base64::engine::general_purpose::STANDARD.decode(&data)
                                {
                                    let mut frame = Vec::with_capacity(4 + decoded.len());
                                    frame.extend_from_slice(&ch_id_num.to_be_bytes());
                                    frame.extend_from_slice(&decoded);
                                    let _ =
                                        agent_for_send.sender.send(AgentEvent::Bytes(frame)).await;
                                }
                            }
                            ClientMsg::Resize { cols, rows } => {
                                let resize_msg = serde_json::json!({
                                    "type": "resize",
                                    "payload": { "channelId": channel_id_clone, "cols": cols, "rows": rows }
                                });
                                let _ = agent_for_send
                                    .sender
                                    .send(AgentEvent::Text(resize_msg.to_string()))
                                    .await;
                            }
                            ClientMsg::Disconnect => break,
                        }
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    });

    let agent_to_frontend = tokio::spawn(async move {
        while let Some(data) = data_rx.recv().await {
            let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
            let msg = ServerMsg::Data {
                payload: DataPayload { data: encoded },
            };
            if ws_sink
                .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    tokio::select! {
        _ = frontend_to_agent => {},
        _ = agent_to_frontend => {},
    }

    // 清理
    {
        let mut tunnel_data = state.agent_tunnel.tunnel_data.write().await;
        tunnel_data.remove(&channel_id);
    }
    {
        let mut channels = state.agent_tunnel.channels.write().await;
        channels.remove(&channel_id);
    }

    tracing::debug!(session_id, "agent terminal session ended");
}

async fn send_ws_error(ws: &mut WebSocket, msg: &str) -> Result<(), axum::Error> {
    ws.send(Message::Text(
        serde_json::to_string(&ServerMsg::Error {
            payload: ErrorPayload {
                message: msg.into(),
            },
        })
        .unwrap(),
    ))
    .await
}
