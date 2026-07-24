//! Tunnel WebSocket — 浏览器 ↔ Hub ↔ Agent ↔ 内网资源。
//!
//! 浏览器通过 /ws/tunnel 连接到 Hub，Hub 将数据通过 Agent 隧道转发到内网资源。
//! 对前端透明——和直连 SSH 的体验完全一致。

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use crate::agent_ws::{AgentEvent, ConnectResponse};
use crate::AppState;

/// 前端 → Hub 的连接请求（第一条消息）
#[derive(Debug, Deserialize)]
struct TunnelConnectRequest {
    protocol: String,
    host: String,
    port: u16,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    private_key: Option<String>,
    #[serde(default)]
    database: Option<String>,
}

/// Hub → 前端的消息
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum TunnelMsg {
    #[serde(rename = "tunnel.connected")]
    Connected,
    #[serde(rename = "tunnel.data")]
    Data { data: String },
    #[serde(rename = "tunnel.error")]
    Error { message: String },
    #[serde(rename = "tunnel.disconnected")]
    Disconnected { reason: String },
}

/// GET /ws/tunnel?agent_id=<id>&resource_id=<id>
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<TunnelQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_tunnel(socket, state, params))
}

#[derive(Debug, Deserialize)]
pub struct TunnelQuery {
    pub agent_id: String,
    pub resource_id: String,
}

async fn handle_tunnel(mut ws: WebSocket, state: AppState, params: TunnelQuery) {
    let start = std::time::Instant::now();
    let total_bytes_frontend_to_agent = Arc::new(AtomicU64::new(0));
    let total_bytes_agent_to_frontend = Arc::new(AtomicU64::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));
    // 1. 等待前端发送连接请求（第一条消息）
    let connect_req = match recv_connect_msg(&mut ws).await {
        Some(req) => req,
        None => {
            let _ = send_error(&mut ws, "expected connect message").await;
            return;
        }
    };

    tracing::info!(
        agent_id = %params.agent_id,
        protocol = %connect_req.protocol,
        host = %connect_req.host,
        "tunnel connect requested"
    );

    // 2. 查找 Agent 连接
    let agent_conn = {
        let conns = state.agent_tunnel.connections.read().await;
        conns.get(&params.agent_id).cloned()
    };

    let agent_conn = match agent_conn {
        Some(c) => c,
        None => {
            let _ = send_error(&mut ws, "agent not connected").await;
            return;
        }
    };

    // 3. 注册 pending request 并发送 connect 到 Agent
    let request_id = format!("req_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

    {
        let mut pending = state.agent_tunnel.pending_requests.write().await;
        pending.insert(request_id.clone(), resp_tx);
    }

    let connect_msg = serde_json::json!({
        "type": "connect",
        "payload": {
            "request_id": request_id,
            "resource_id": params.resource_id,
            "protocol": connect_req.protocol,
            "config": {
                "host": connect_req.host,
                "port": connect_req.port,
                "username": connect_req.username,
                "password": connect_req.password,
                "privateKey": connect_req.private_key,
                "database": connect_req.database,
            }
        }
    });

    if agent_conn
        .sender
        .send(AgentEvent::Text(connect_msg.to_string()))
        .await
        .is_err()
    {
        let _ = send_error(&mut ws, "failed to send connect to agent").await;
        return;
    }

    // 4. 等待 Agent 响应（5 秒超时）
    let connect_result = tokio::time::timeout(std::time::Duration::from_secs(5), resp_rx).await;

    let channel_id = match connect_result {
        Ok(Ok(ConnectResponse {
            channel_id: Some(id),
            ..
        })) => id,
        Ok(Ok(ConnectResponse { error: Some(e), .. })) => {
            let _ = send_error(&mut ws, &e).await;
            return;
        }
        Ok(Ok(ConnectResponse { .. })) => {
            let _ = send_error(&mut ws, "agent returned empty response").await;
            return;
        }
        Ok(Err(_)) => {
            let _ = send_error(&mut ws, "agent response channel closed").await;
            return;
        }
        Err(_) => {
            // 超时 — 清理 pending request
            let mut pending = state.agent_tunnel.pending_requests.write().await;
            pending.remove(&request_id);
            let _ = send_error(&mut ws, "agent response timeout").await;
            return;
        }
    };

    tracing::info!(channel_id = %channel_id, "tunnel established");

    // 5. 通知前端连接成功
    let _ = ws
        .send(Message::Text(
            serde_json::to_string(&TunnelMsg::Connected).unwrap().into(),
        ))
        .await;

    // 6. 注册 tunnel data channel（用于接收 Agent 二进制数据）
    let (data_tx, mut data_rx) = mpsc::channel::<Vec<u8>>(512);
    {
        let mut tunnel_data = state.agent_tunnel.tunnel_data.write().await;
        tunnel_data.insert(channel_id.clone(), data_tx);
    }

    // 7. 拆分前端 WebSocket
    let (mut ws_sink, mut ws_stream) = ws.split();

    // 8. 前端 → Agent（文本帧转二进制帧）
    let f2a_bytes = Arc::clone(&total_bytes_frontend_to_agent);
    let f2a_errors = Arc::clone(&error_count);
    let agent_conn_for_send = agent_conn.clone();
    let ch_id = channel_id.clone();
    let frontend_to_agent = tokio::spawn(async move {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // 前端发来的文本数据（terminal.data 等），转为二进制帧
                    let data = text.as_bytes().to_vec();
                    let ch_id_bytes = ch_id.parse::<u32>().unwrap_or(0).to_be_bytes();
                    let mut frame = Vec::with_capacity(4 + data.len());
                    frame.extend_from_slice(&ch_id_bytes);
                    frame.extend_from_slice(&data);
                    if agent_conn_for_send
                        .sender
                        .send(AgentEvent::Bytes(frame))
                        .await
                        .is_err()
                    {
                        f2a_errors.fetch_add(1, Ordering::Relaxed);
                        break;
                    }
                    f2a_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
                }
                Ok(Message::Binary(data)) => {
                    let ch_id_bytes = ch_id.parse::<u32>().unwrap_or(0).to_be_bytes();
                    let mut frame = Vec::with_capacity(4 + data.len());
                    frame.extend_from_slice(&ch_id_bytes);
                    frame.extend_from_slice(&data);
                    if agent_conn_for_send
                        .sender
                        .send(AgentEvent::Bytes(frame))
                        .await
                        .is_err()
                    {
                        f2a_errors.fetch_add(1, Ordering::Relaxed);
                        break;
                    }
                    f2a_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    });

    // 9. Agent → 前端（二进制数据转文本帧）
    let a2f_bytes = Arc::clone(&total_bytes_agent_to_frontend);
    let a2f_errors = Arc::clone(&error_count);
    let agent_to_frontend = tokio::spawn(async move {
        while let Some(data) = data_rx.recv().await {
            let msg = Message::Text(String::from_utf8_lossy(&data).to_string().into());
            if ws_sink.send(msg).await.is_err() {
                a2f_errors.fetch_add(1, Ordering::Relaxed);
                break;
            }
            a2f_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
        }
    });

    // 10. 等待任一方向结束
    tokio::select! {
        _ = frontend_to_agent => {},
        _ = agent_to_frontend => {},
    }

    // 11. 清理
    {
        let mut tunnel_data = state.agent_tunnel.tunnel_data.write().await;
        tunnel_data.remove(&channel_id);
    }
    {
        let mut channels = state.agent_tunnel.channels.write().await;
        channels.remove(&channel_id);
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let bytes_forwarded =
        total_bytes_frontend_to_agent.load(Ordering::Relaxed)
            + total_bytes_agent_to_frontend.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);
    tracing::info!(
        channel_id = %channel_id,
        duration_ms,
        bytes_forwarded,
        error_count = errors,
        "tunnel closed"
    );
}

/// 从 WebSocket 读取连接请求
async fn recv_connect_msg(ws: &mut WebSocket) -> Option<TunnelConnectRequest> {
    while let Some(msg) = ws.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(req) = serde_json::from_str::<TunnelConnectRequest>(&text) {
                return Some(req);
            }
        }
    }
    None
}

/// 发送错误消息到前端
async fn send_error(ws: &mut WebSocket, msg: &str) -> Result<(), axum::Error> {
    let err = serde_json::to_string(&TunnelMsg::Error {
        message: msg.into(),
    })
    .unwrap();
    ws.send(Message::Text(err.into())).await
}
