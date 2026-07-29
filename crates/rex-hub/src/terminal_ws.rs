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
    /// 客户端心跳（每30秒发送），后端忽略即可维持连接活跃
    #[serde(rename = "ping")]
    Ping,
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
    keepalive_interval: Option<u32>,
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
            tracing::warn!(action = "SSH_RESOURCE_LOAD", resource_id = %resource_id, error = %e, "SSH resource load failed");
            let _ = send_ws_error(&mut ws, &e).await;
            return;
        }
    };

    tracing::info!(
        action = "SSH_CONNECT",
        resource_id = %resource_id,
        host = %conn_info.host,
        port = conn_info.port,
        username = %conn_info.username,
        use_agent = conn_info.use_agent,
        "SSH connection initiated"
    );

    if conn_info.use_agent {
        handle_agent_terminal(ws, &state, &conn_info, &resource_id, &session_id).await;
    } else {
        handle_direct_terminal(ws, &conn_info, &session_id).await;
    }

    tracing::info!(
        action = "SSH_DISCONNECT",
        resource_id = %resource_id,
        "SSH session ended"
    );
}

/// 从 DB 读取资源连接信息
/// host/port/username 来自 Resource 顶层字段；password/privateKey 来自 config_json（加密存储）
async fn load_resource_conn(
    state: &AppState,
    resource_id: &str,
) -> Result<ResourceConnInfo, String> {
    let db = state.db.clone();
    let rid = resource_id.to_string();
    let crypto = state.crypto.clone();

    tokio::task::spawn_blocking(move || {
        // 获取资源
        tracing::debug!(action = "SSH_RESOURCE_LOAD", resource_id = %rid, "loading resource connection info");
        let resource = db
            .get_resource(&rid)
            .map_err(|e| {
                tracing::error!(action = "SSH_RESOURCE_LOAD", resource_id = %rid, error = %e, "database query failed");
                format!("db error: {e}")
            })?
            .ok_or_else(|| {
                tracing::warn!(action = "SSH_RESOURCE_LOAD", resource_id = %rid, "resource not found in database");
                "resource not found".to_string()
            })?;

        tracing::debug!(
            action = "SSH_RESOURCE_LOAD",
            resource_id = %rid,
            name = %resource.name,
            protocol = %resource.protocol,
            host = %resource.host,
            port = ?resource.port,
            username = %resource.username,
            has_config_json = %(!resource.config_json.is_empty() && resource.config_json != "{}"),
            "resource loaded"
        );

        // 从 Resource 顶层字段获取连接信息
        let host = resource.host.clone();
        if host.is_empty() {
            return Err(format!("resource {rid}: host is empty, please fill in host in resource settings"));
        }
        let port = resource.port.unwrap_or(22);
        let username = if resource.username.is_empty() {
            "root".to_string()
        } else {
            resource.username.clone()
        };

        // 从 config_json 解密敏感字段（password、privateKey）
        let (password, private_key) = if !resource.config_json.is_empty() && resource.config_json != "{}" {
            let config_str = crypto
                .decrypt(&resource.config_json)
                .map_err(|e| {
                    tracing::error!(action = "SSH_CONFIG_DECRYPT", resource_id = %rid, error = %e, "config_json decryption failed");
                    format!("decrypt failed: {e}")
                })?;

            let config: serde_json::Value = serde_json::from_str(&config_str).map_err(|e| {
                tracing::error!(action = "SSH_CONFIG_PARSE", resource_id = %rid, error = %e, "config_json parse failed");
                format!("invalid config json: {e}")
            })?;

            let pw = config
                .get("password")
                .and_then(|v| v.as_str())
                .map(String::from);
            let pk = config
                .get("privateKey")
                .and_then(|v| v.as_str())
                .map(String::from);

            tracing::debug!(
                action = "SSH_CONFIG_LOADED",
                resource_id = %rid,
                has_password = pw.is_some(),
                has_private_key = pk.is_some(),
                "sensitive config loaded"
            );

            (pw, pk)
        } else {
            tracing::debug!(action = "SSH_CONFIG_PARSE", resource_id = %rid, "no config_json — using defaults");
            (None, None)
        };

        let auth_method = if private_key.is_some() {
            "key"
        } else if password.is_some() {
            "password"
        } else {
            "none"
        };
        tracing::info!(
            action = "SSH_CONFIG_LOADED",
            resource_id = %rid,
            host = %host,
            port = port,
            username = %username,
            auth_method = auth_method,
            "SSH connection parameters resolved"
        );

        // 获取环境信息
        let env = db
            .get_environment(&resource.environment_id)
            .map_err(|e| {
                tracing::error!(action = "SSH_ENV_LOAD", resource_id = %rid, env_id = %resource.environment_id, error = %e, "failed to load environment");
                format!("db error: {e}")
            })?
            .ok_or_else(|| {
                tracing::warn!(action = "SSH_ENV_NOT_FOUND", resource_id = %rid, env_id = %resource.environment_id, "environment not found");
                "environment not found".to_string()
            })?;

        let use_agent = env.connection_mode == "agent";
        tracing::debug!(
            action = "SSH_ENV_LOADED",
            resource_id = %rid,
            env_id = %resource.environment_id,
            connection_mode = %env.connection_mode,
            use_agent = use_agent,
            "environment loaded"
        );

        let agent_id = if use_agent {
            let agents = db
                .list_agents_by_env(&resource.environment_id)
                .unwrap_or_default();
            let online = agents.iter().find(|a| a.status == "online");
            tracing::debug!(
                action = "SSH_AGENT_LOOKUP",
                resource_id = %rid,
                total_agents = agents.len(),
                online_agent = online.map(|a| a.id.as_str()).unwrap_or("none"),
                "agent lookup"
            );
            online.map(|a| a.id.clone())
        } else {
            None
        };

        if use_agent && agent_id.is_none() {
            tracing::warn!(
                action = "SSH_NO_AGENT",
                resource_id = %rid,
                env_id = %resource.environment_id,
                "no online agent available — agent connection will fail"
            );
        }

        Ok(ResourceConnInfo {
            host,
            port,
            username,
            password,
            private_key,
            use_agent,
            agent_id,
            keepalive_interval: None,
        })
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

// ═══════════════════════════════════════
// 直连模式
// ═══════════════════════════════════════

async fn handle_direct_terminal(mut ws: WebSocket, conn: &ResourceConnInfo, session_id: &str) {
    tracing::info!(
        action = "SSH_DIRECT_CONNECT",
        session_id = %session_id,
        host = %conn.host,
        port = conn.port,
        username = %conn.username,
        has_password = conn.password.is_some(),
        has_private_key = conn.private_key.is_some(),
        "SSH direct connection attempting"
    );

    let config = SshConfig {
        host: conn.host.clone(),
        port: conn.port,
        username: conn.username.clone(),
        password: conn.password.clone(),
        private_key: conn.private_key.clone(),
        keepalive_interval: conn.keepalive_interval,
    };

    let session = match SshSession::connect(config).await {
        Ok(s) => {
            tracing::info!(action = "SSH_DIRECT_CONNECTED", session_id = %session_id, host = %conn.host, "SSH direct connection established");
            s
        }
        Err(e) => {
            tracing::error!(
                action = "SSH_DIRECT_FAILED",
                session_id = %session_id,
                host = %conn.host,
                port = conn.port,
                username = %conn.username,
                error = %e,
                "SSH direct connection failed"
            );
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
            .unwrap()
            .into(),
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
                        Some(ClientMsg::Ping) => {
                            // 心跳收到，无需响应（前端只是发 ping 维持连接活跃）
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
                Message::Text(data.into())
            } else {
                let wrapped = ServerMsg::Data {
                    payload: DataPayload { data },
                };
                Message::Text(serde_json::to_string(&wrapped).unwrap().into())
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

    tracing::debug!(
        action = "SSH_SESSION_END",
        session_id,
        "terminal session ended"
    );
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
    tracing::info!(
        action = "SSH_AGENT_CONNECT",
        session_id = %session_id,
        resource_id = %resource_id,
        host = %conn.host,
        port = conn.port,
        username = %conn.username,
        "SSH agent connection attempting"
    );

    let agent_id = match conn.agent_id.as_ref() {
        Some(id) => id.clone(),
        None => {
            tracing::error!(
                action = "SSH_AGENT_NO_ONLINE",
                session_id = %session_id,
                resource_id = %resource_id,
                "no online agent available for this environment"
            );
            let _ = send_ws_error(&mut ws, "no online agent for this environment").await;
            return;
        }
    };

    tracing::debug!(action = "SSH_AGENT_SELECTED", session_id = %session_id, agent_id = %agent_id, "agent selected");

    let agent_conn = {
        let conns = state.agent_tunnel.connections.read().await;
        conns.get(&agent_id).cloned()
    };

    let agent_conn = match agent_conn {
        Some(c) => c,
        None => {
            tracing::error!(
                action = "SSH_AGENT_NOT_FOUND",
                session_id = %session_id,
                agent_id = %agent_id,
                "agent WebSocket connection not found — agent may have disconnected"
            );
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

    tracing::debug!(
        action = "SSH_AGENT_SEND_FAILED",
        session_id = %session_id,
        agent_id = %agent_id,
        request_id = %request_id,
        host = %conn.host,
        port = conn.port,
        "sending connect request to agent"
    );

    if agent_conn
        .sender
        .send(AgentEvent::Text(connect_msg.to_string()))
        .await
        .is_err()
    {
        tracing::error!(
            action = "SSH_AGENT_SEND_FAILED",
            session_id = %session_id,
            agent_id = %agent_id,
            request_id = %request_id,
            "failed to send connect request to agent — channel may be closed"
        );
        let _ = send_ws_error(&mut ws, "failed to contact agent").await;
        return;
    }

    // 等待 Agent 响应
    let channel_id = match tokio::time::timeout(std::time::Duration::from_secs(10), resp_rx).await {
        Ok(Ok(ConnectResponse {
            channel_id: Some(id),
            ..
        })) => {
            tracing::info!(
                action = "SSH_AGENT_CONNECTED",
                session_id = %session_id,
                agent_id = %agent_id,
                channel_id = %id,
                "agent SSH connection established"
            );
            id
        }
        Ok(Ok(ConnectResponse { error: Some(e), .. })) => {
            tracing::error!(
                action = "SSH_AGENT_ERROR",
                session_id = %session_id,
                agent_id = %agent_id,
                request_id = %request_id,
                error = %e,
                "agent reported connection error"
            );
            let _ = send_ws_error(&mut ws, &e).await;
            return;
        }
        Ok(Ok(_)) => {
            tracing::error!(
                action = "SSH_AGENT_ERROR",
                session_id = %session_id,
                agent_id = %agent_id,
                request_id = %request_id,
                "agent returned unexpected response (no channel_id, no error)"
            );
            let _ = send_ws_error(&mut ws, "agent returned unexpected response").await;
            return;
        }
        Ok(Err(_)) => {
            tracing::error!(
                action = "SSH_AGENT_ERROR",
                session_id = %session_id,
                agent_id = %agent_id,
                request_id = %request_id,
                "agent response channel closed unexpectedly"
            );
            let _ = send_ws_error(&mut ws, "agent connection failed (channel closed)").await;
            return;
        }
        Err(_) => {
            tracing::error!(
                action = "SSH_AGENT_TIMEOUT",
                session_id = %session_id,
                agent_id = %agent_id,
                request_id = %request_id,
                "agent connection timed out after 10s"
            );
            let _ = send_ws_error(&mut ws, "agent connection timeout").await;
            return;
        }
    };

    tracing::info!(action = "SSH_AGENT_CONNECTED", channel_id = %channel_id, session_id = %session_id, "agent terminal connected");

    // 通知前端连接成功
    let _ = ws
        .send(Message::Text(
            serde_json::to_string(&ServerMsg::Connected {
                payload: ConnectedPayload {
                    session_id: session_id.to_string(),
                },
            })
            .unwrap()
            .into(),
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
                            ClientMsg::Ping => {
                                // 心跳收到，无需响应
                            }
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
                .send(Message::Text(serde_json::to_string(&msg).unwrap().into()))
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

    tracing::debug!(
        action = "SSH_SESSION_END",
        session_id,
        "agent terminal session ended"
    );
}

async fn send_ws_error(ws: &mut WebSocket, msg: &str) -> Result<(), axum::Error> {
    ws.send(Message::Text(
        serde_json::to_string(&ServerMsg::Error {
            payload: ErrorPayload {
                message: msg.into(),
            },
        })
        .unwrap()
        .into(),
    ))
    .await
}
