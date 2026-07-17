//! Agent WebSocket 隧道 — 单一连接完成认证、心跳、资源连接、数据转发。
//!
//! 协议：控制消息用 JSON 文本帧，数据转发用二进制帧（前 4 字节 channelId）。

use std::collections::HashMap;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::AppState;

// ═══════════════════════════════════════
// Agent → Hub 消息
// ═══════════════════════════════════════

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AgentMsg {
    #[serde(rename = "auth")]
    Auth {
        payload: AuthPayload,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat {
        payload: HeartbeatPayload,
    },
    #[serde(rename = "connected")]
    Connected {
        payload: ConnectedPayload,
    },
    #[serde(rename = "connect_error")]
    ConnectError {
        payload: ConnectErrorPayload,
    },
    #[serde(rename = "closed")]
    Closed {
        payload: ChannelIdPayload,
    },
}

#[derive(Debug, Deserialize)]
struct AuthPayload {
    agent_id: String,
    #[allow(dead_code)]
    token: String,
}

#[derive(Debug, Deserialize)]
struct HeartbeatPayload {
    #[serde(default)]
    version: String,
    #[serde(default)]
    #[allow(dead_code)]
    os: String,
    #[serde(default)]
    #[allow(dead_code)]
    arch: String,
    #[serde(default)]
    #[allow(dead_code)]
    hostname: String,
}

#[derive(Debug, Deserialize)]
struct ConnectedPayload {
    request_id: String,
    channel_id: String,
}

#[derive(Debug, Deserialize)]
struct ConnectErrorPayload {
    request_id: String,
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChannelIdPayload {
    channel_id: String,
}

// ═══════════════════════════════════════
// Hub → Agent 消息
// ═══════════════════════════════════════

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum HubMsg {
    #[serde(rename = "auth_ok")]
    AuthOk {
        payload: AuthOkPayload,
    },
    #[serde(rename = "auth_fail")]
    AuthFail {
        payload: AuthFailPayload,
    },
    #[serde(rename = "heartbeat_ack")]
    HeartbeatAck,
    #[serde(rename = "connect")]
    Connect {
        payload: ConnectRequest,
    },
    #[serde(rename = "close")]
    Close {
        payload: ChannelIdPayload,
    },
}

#[derive(Debug, Serialize)]
struct AuthOkPayload {
    agent_id: String,
}

#[derive(Debug, Serialize)]
struct AuthFailPayload {
    reason: String,
}

#[derive(Debug, Serialize)]
struct ConnectRequest {
    request_id: String,
    resource_id: String,
    protocol: String,
    config: serde_json::Value,
}

// ═══════════════════════════════════════
// 运行时状态
// ═══════════════════════════════════════

/// Agent WebSocket 连接
#[derive(Clone)]
pub struct AgentConnection {
    pub agent_id: String,
    pub sender: mpsc::Sender<AgentEvent>,
}

/// 发给 Agent WebSocket 写入任务的事件
pub enum AgentEvent {
    Text(String),
    Bytes(Vec<u8>),
    Close,
}

/// Agent connect 请求的响应
pub struct ConnectResponse {
    pub channel_id: Option<String>,
    pub error: Option<String>,
}

/// 运行时 Agent 连接池和 channel 映射
pub struct AgentTunnelState {
    /// agent_id → AgentConnection
    pub connections: RwLock<HashMap<String, AgentConnection>>,
    /// channel_id → agent_id
    pub channels: RwLock<HashMap<String, String>>,
    /// request_id → response sender（用于 connect 请求-响应匹配）
    pub pending_requests: RwLock<HashMap<String, oneshot::Sender<ConnectResponse>>>,
    /// channel_id → tunnel data sender（用于二进制帧路由到正确的 tunnel）
    pub tunnel_data: RwLock<HashMap<String, mpsc::Sender<Vec<u8>>>>,
}

impl AgentTunnelState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for AgentTunnelState {
    fn default() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            channels: RwLock::new(HashMap::new()),
            pending_requests: RwLock::new(HashMap::new()),
            tunnel_data: RwLock::new(HashMap::new()),
        }
    }
}

// ═══════════════════════════════════════
// WebSocket 入口
// ═══════════════════════════════════════

#[derive(Deserialize)]
pub struct AgentQuery {
    #[allow(dead_code)]
    pub token: String,
}

/// GET /ws/agent?token=<agent_token>
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<AgentQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_agent_socket(socket, query.token, state))
}

/// Agent WebSocket 主处理循环
async fn handle_agent_socket(ws: WebSocket, token: String, state: AppState) {
    // 拆分 WebSocket
    let (mut ws_sink, mut ws_stream) = ws.split();

    // 1. 等待 auth 消息
    let auth_msg = match recv_agent_msg(&mut ws_stream).await {
        Some(AgentMsg::Auth { payload }) => payload,
        _ => {
            let fail = serde_json::to_string(&HubMsg::AuthFail {
                payload: AuthFailPayload {
                    reason: "expected auth message".into(),
                },
            })
            .unwrap();
            let _ = ws_sink.send(Message::Text(fail)).await;
            return;
        }
    };

    // 2. 验证 token
    let db = state.db.clone();
    let agent_id = auth_msg.agent_id.clone();
    let token_clone = token.clone();
    let verified = tokio::task::spawn_blocking(move || db.verify_agent_token(&agent_id, &token_clone))
        .await;
    let verified_id = match verified {
        Ok(Ok(Some(id))) => id,
        _ => {
            let fail = serde_json::to_string(&HubMsg::AuthFail {
                payload: AuthFailPayload {
                    reason: "invalid token or agent not found".into(),
                },
            })
            .unwrap();
            let _ = ws_sink.send(Message::Text(fail)).await;
            return;
        }
    };

    // 3. 认证成功 — 标记 online
    let db = state.db.clone();
    let aid = verified_id.clone();
    let _ = tokio::task::spawn_blocking(move || db.update_agent_heartbeat(&aid, "", ""))
        .await;

    let ok_msg = serde_json::to_string(&HubMsg::AuthOk {
        payload: AuthOkPayload {
            agent_id: verified_id.clone(),
        },
    })
    .unwrap();
    if ws_sink.send(Message::Text(ok_msg)).await.is_err() {
        return;
    }

    tracing::info!(agent_id = %verified_id, "agent WebSocket connected");

    // 4. 创建事件通道
    let (evt_tx, mut evt_rx) = mpsc::channel::<AgentEvent>(256);

    // 注册到连接池
    {
        let mut conns = state.agent_tunnel.connections.write().await;
        conns.insert(
            verified_id.clone(),
            AgentConnection {
                agent_id: verified_id.clone(),
                sender: evt_tx.clone(),
            },
        );
    }

    // 5. 启动写入任务（evt_rx → WebSocket）
    let ws_write_task = tokio::spawn(async move {
        while let Some(evt) = evt_rx.recv().await {
            let msg = match evt {
                AgentEvent::Text(t) => Message::Text(t),
                AgentEvent::Bytes(b) => Message::Binary(b),
                AgentEvent::Close => break,
            };
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 6. 读取循环
    let agent_id = verified_id;
    let state_clone = state.clone();
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(agent_msg) = serde_json::from_str::<AgentMsg>(&text) {
                    handle_agent_msg(
                        agent_msg,
                        &agent_id,
                        &state_clone,
                    )
                    .await;
                }
            }
            Ok(Message::Binary(data)) => {
                // 二进制帧：前 4 字节 channelId，其余为数据
                if data.len() >= 4 {
                    let ch_id = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                    let ch_id_str = ch_id.to_string();
                    let payload = data[4..].to_vec();
                    // 转发到对应的 tunnel
                    let tunnel_data = state_clone.agent_tunnel.tunnel_data.read().await;
                    if let Some(tx) = tunnel_data.get(&ch_id_str) {
                        let _ = tx.send(payload).await;
                    }
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    // 7. 清理
    tracing::info!(agent_id = %agent_id, "agent WebSocket disconnected");

    // 标记 offline
    let db = state.db.clone();
    let aid = agent_id.clone();
    let _ = tokio::task::spawn_blocking(move || db.set_agent_offline(&aid)).await;

    // 从连接池移除
    {
        let mut conns = state.agent_tunnel.connections.write().await;
        conns.remove(&agent_id);
    }

    // 取消写入任务
    ws_write_task.abort();
}

/// 处理 Agent 发来的控制消息
async fn handle_agent_msg(msg: AgentMsg, agent_id: &str, state: &AppState) {
    match msg {
        AgentMsg::Heartbeat { payload } => {
            let db = state.db.clone();
            let aid = agent_id.to_string();
            let ver = payload.version;
            let _ = tokio::task::spawn_blocking(move || {
                db.update_agent_heartbeat(&aid, &ver, "")
            })
            .await;

            // 回复 ack
            if let Some(conn) = state.agent_tunnel.connections.read().await.get(agent_id) {
                let ack = serde_json::to_string(&HubMsg::HeartbeatAck).unwrap();
                let _ = conn.sender.send(AgentEvent::Text(ack)).await;
            }
        }
        AgentMsg::Connected {
            payload:
                ConnectedPayload {
                    request_id,
                    channel_id,
                },
        } => {
            // 注册 channel → agent 映射
            let mut channels = state.agent_tunnel.channels.write().await;
            channels.insert(channel_id.clone(), agent_id.to_string());

            // 通知等待此 requestId 的 tunnel
            let mut pending = state.agent_tunnel.pending_requests.write().await;
            if let Some(tx) = pending.remove(&request_id) {
                let _ = tx.send(ConnectResponse {
                    channel_id: Some(channel_id.clone()),
                    error: None,
                });
            }

            tracing::info!(
                agent_id,
                request_id,
                channel_id,
                "agent resource connected"
            );
        }
        AgentMsg::ConnectError {
            payload:
                ConnectErrorPayload {
                    request_id,
                    error,
                },
        } => {
            // 通知等待此 requestId 的 tunnel
            let mut pending = state.agent_tunnel.pending_requests.write().await;
            if let Some(tx) = pending.remove(&request_id) {
                let _ = tx.send(ConnectResponse {
                    channel_id: None,
                    error: Some(error.clone()),
                });
            }

            tracing::warn!(agent_id, request_id, error, "agent resource connect failed");
        }
        AgentMsg::Closed {
            payload: ChannelIdPayload { channel_id },
        } => {
            let mut channels = state.agent_tunnel.channels.write().await;
            channels.remove(&channel_id);
            tracing::info!(agent_id, channel_id, "agent channel closed");
        }
        AgentMsg::Auth { .. } => {
            // 已在握手阶段处理，忽略后续 auth 消息
        }
    }
}

/// 从 WebSocket 流读取下一条文本消息并解析为 AgentMsg
async fn recv_agent_msg(stream: &mut (impl StreamExt<Item = Result<Message, axum::Error>> + Unpin)) -> Option<AgentMsg> {
    while let Some(msg) = stream.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(parsed) = serde_json::from_str::<AgentMsg>(&text) {
                return Some(parsed);
            }
        }
    }
    None
}
