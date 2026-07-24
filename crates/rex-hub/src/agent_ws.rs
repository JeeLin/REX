//! Agent WebSocket 隧道 — 单一连接完成认证、心跳、资源连接、数据转发。
//!
//! 协议：控制消息用 JSON 文本帧，数据转发用二进制帧（前 4 字节 channelId）。

use std::collections::HashMap;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
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
    Auth { payload: AuthPayload },
    #[serde(rename = "heartbeat")]
    Heartbeat { payload: HeartbeatPayload },
    #[serde(rename = "connected")]
    Connected { payload: ConnectedPayload },
    #[serde(rename = "connect_error")]
    ConnectError { payload: ConnectErrorPayload },
    #[serde(rename = "closed")]
    Closed { payload: ChannelIdPayload },
}

#[derive(Debug, Deserialize)]
struct AuthPayload {
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
    AuthOk { payload: AuthOkPayload },
    #[serde(rename = "auth_fail")]
    AuthFail { payload: AuthFailPayload },
    #[serde(rename = "heartbeat_ack")]
    HeartbeatAck,
    #[serde(rename = "connect")]
    Connect { payload: ConnectRequest },
    #[serde(rename = "close")]
    Close { payload: ChannelIdPayload },
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
#[derive(Debug)]
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

/// GET /ws/agent — Agent 通过 WebSocket auth 消息认证
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_agent_socket(socket, state))
}

/// Agent WebSocket 主处理循环
async fn handle_agent_socket(ws: WebSocket, state: AppState) {
    // 拆分 WebSocket
    let (mut ws_sink, mut ws_stream) = ws.split();

    // 1. 等待 auth 消息
    let auth_msg = match recv_agent_msg(&mut ws_stream).await {
        Some(AgentMsg::Auth { payload }) => payload,
        _ => {
            if let Ok(fail) = serde_json::to_string(&HubMsg::AuthFail {
                payload: AuthFailPayload {
                    reason: "expected auth message".into(),
                },
            }) {
                let _ = ws_sink.send(Message::Text(fail.into())).await;
            }
            return;
        }
    };

    // 2. 通过 token 查找 Agent
    let db = state.db.clone();
    let token_for_lookup = auth_msg.token.clone();
    let verified =
        tokio::task::spawn_blocking(move || db.find_agent_by_token(&token_for_lookup)).await;
    let verified_id = match verified {
        Ok(Ok(Some(id))) => id,
        _ => {
            if let Ok(fail) = serde_json::to_string(&HubMsg::AuthFail {
                payload: AuthFailPayload {
                    reason: "invalid token or agent not found".into(),
                },
            }) {
                let _ = ws_sink.send(Message::Text(fail.into())).await;
            }
            return;
        }
    };

    // 3. 认证成功 — 标记 online
    let db = state.db.clone();
    let aid = verified_id.clone();
    let _ = tokio::task::spawn_blocking(move || db.update_agent_heartbeat(&aid, "", "")).await;

    let ok_msg = match serde_json::to_string(&HubMsg::AuthOk {
        payload: AuthOkPayload {
            agent_id: verified_id.clone(),
        },
    }) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(agent_id = %verified_id, error = %e, "failed to serialize auth_ok");
            return;
        }
    };
    if ws_sink.send(Message::Text(ok_msg.into())).await.is_err() {
        return;
    }

    tracing::info!(action = "AGENT_ONLINE", agent_id = %verified_id, "agent WebSocket connected");

    // 审计日志：Agent 上线
    {
        let audit_db = state.db.clone();
        let aid = verified_id.clone();
        let _ = tokio::task::spawn_blocking(move || {
            audit_db.write_audit_log(&crate::models::NewAuditEntry {
                action: "AGENT_ONLINE".into(),
                target: Some(aid.clone()),
                agent_id: Some(aid),
                result: "success".into(),
                ..Default::default()
            })
        })
        .await;
    }

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
                AgentEvent::Text(t) => Message::Text(t.into()),
                AgentEvent::Bytes(b) => Message::Binary(b.into()),
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
    let mut total_bytes_forwarded: u64 = 0;
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(agent_msg) = serde_json::from_str::<AgentMsg>(&text) {
                    handle_agent_msg(agent_msg, &agent_id, &state_clone).await;
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
                        let payload_len = payload.len() as u64;
                        let _ = tx.send(payload).await;
                        total_bytes_forwarded += payload_len;
                    }
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    // 7. 清理
    tracing::info!(action = "AGENT_OFFLINE", agent_id = %agent_id, bytes_forwarded = total_bytes_forwarded, "agent WebSocket disconnected");

    // 审计日志：Agent 离线
    {
        let audit_db = state.db.clone();
        let aid = agent_id.clone();
        let _ = tokio::task::spawn_blocking(move || {
            audit_db.write_audit_log(&crate::models::NewAuditEntry {
                action: "AGENT_OFFLINE".into(),
                target: Some(aid.clone()),
                agent_id: Some(aid),
                result: "success".into(),
                ..Default::default()
            })
        })
        .await;
    }

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
            let ver = payload.version.clone();
            let _ = tokio::task::spawn_blocking(move || db.update_agent_heartbeat(&aid, &ver, ""))
                .await;

            // 回复 ack
            if let Some(conn) = state.agent_tunnel.connections.read().await.get(agent_id) {
                if let Ok(ack) = serde_json::to_string(&HubMsg::HeartbeatAck) {
                    let _ = conn.sender.send(AgentEvent::Text(ack)).await;
                }

                // 版本对比 — Hub 版本 ≠ Agent 版本时推送更新
                let hub_version = env!("CARGO_PKG_VERSION");
                if !payload.version.is_empty() && payload.version != hub_version {
                    tracing::info!(
                        action = "AGENT_VERSION_MISMATCH",
                        agent_id = %agent_id,
                        agent_version = %payload.version,
                        hub_version = hub_version,
                        "version mismatch detected, pushing update"
                    );
                    // 构造 Agent 下载 URL（Hub 提供二进制）
                    // 验证 os/arch 值，防止恶意数据
                    let valid_os = ["linux", "windows", "macos"];
                    let valid_arch = ["amd64", "arm64"];
                    let os = if valid_os.contains(&payload.os.as_str()) {
                        payload.os.clone()
                    } else {
                        "linux".into()
                    };
                    let arch = if valid_arch.contains(&payload.arch.as_str()) {
                        payload.arch.clone()
                    } else {
                        "amd64".into()
                    };
                    let download_url = format!("/api/agents/download?os={os}&arch={arch}");

                    let update_cmd = rex_common::update::UpdateCommand {
                        version: hub_version.to_string(),
                        download_url,
                        fallback_url: String::new(),
                        sha256: String::new(),
                    };
                    let msg = serde_json::to_string(&serde_json::json!({
                        "type": "update",
                        "payload": update_cmd
                    }))
                    .unwrap();
                    let _ = conn.sender.send(AgentEvent::Text(msg)).await;
                }
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

            tracing::info!(action = "AGENT_RESOURCE_CONNECTED", agent_id, request_id, channel_id, "agent resource connected");
        }
        AgentMsg::ConnectError {
            payload: ConnectErrorPayload { request_id, error },
        } => {
            // 通知等待此 requestId 的 tunnel
            let mut pending = state.agent_tunnel.pending_requests.write().await;
            if let Some(tx) = pending.remove(&request_id) {
                let _ = tx.send(ConnectResponse {
                    channel_id: None,
                    error: Some(error.clone()),
                });
            }

            tracing::warn!(action = "AGENT_RESOURCE_CONNECT_FAILED", agent_id, request_id, error, "agent resource connect failed");
        }
        AgentMsg::Closed {
            payload: ChannelIdPayload { channel_id },
        } => {
            let mut channels = state.agent_tunnel.channels.write().await;
            channels.remove(&channel_id);
            tracing::info!(action = "AGENT_CHANNEL_CLOSED", agent_id, channel_id, "agent channel closed");
        }
        AgentMsg::Auth { .. } => {
            // 已在握手阶段处理，忽略后续 auth 消息
        }
    }
}

/// 从 WebSocket 流读取下一条文本消息并解析为 AgentMsg
async fn recv_agent_msg(
    stream: &mut (impl StreamExt<Item = Result<Message, axum::Error>> + Unpin),
) -> Option<AgentMsg> {
    while let Some(msg) = stream.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(parsed) = serde_json::from_str::<AgentMsg>(&text) {
                return Some(parsed);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_msg_auth_deserialize() {
        let json = r#"{"type":"auth","payload":{"token":"tok1"}}"#;
        let msg: AgentMsg = serde_json::from_str(json).unwrap();
        match msg {
            AgentMsg::Auth { payload } => {
                assert_eq!(payload.token, "tok1");
            }
            _ => panic!("expected Auth"),
        }
    }

    #[test]
    fn test_agent_msg_heartbeat_deserialize() {
        let json = r#"{"type":"heartbeat","payload":{"version":"0.16.0","os":"linux","arch":"x86_64","hostname":"server1"}}"#;
        let msg: AgentMsg = serde_json::from_str(json).unwrap();
        match msg {
            AgentMsg::Heartbeat { payload } => {
                assert_eq!(payload.version, "0.16.0");
                assert_eq!(payload.os, "linux");
            }
            _ => panic!("expected Heartbeat"),
        }
    }

    #[test]
    fn test_agent_msg_connected_deserialize() {
        let json = r#"{"type":"connected","payload":{"request_id":"req_1","channel_id":"ch_1"}}"#;
        let msg: AgentMsg = serde_json::from_str(json).unwrap();
        match msg {
            AgentMsg::Connected { payload } => {
                assert_eq!(payload.request_id, "req_1");
                assert_eq!(payload.channel_id, "ch_1");
            }
            _ => panic!("expected Connected"),
        }
    }

    #[test]
    fn test_hub_msg_auth_ok_serialize() {
        let msg = HubMsg::AuthOk {
            payload: AuthOkPayload {
                agent_id: "a1".into(),
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("auth_ok"));
        assert!(json.contains("a1"));
    }

    #[test]
    fn test_hub_msg_connect_serialize() {
        let msg = HubMsg::Connect {
            payload: ConnectRequest {
                request_id: "req_1".into(),
                resource_id: "res_1".into(),
                protocol: "ssh".into(),
                config: serde_json::json!({"host": "10.0.0.1", "port": 22}),
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("connect"));
        assert!(json.contains("req_1"));
    }

    #[tokio::test]
    async fn test_tunnel_state_default() {
        let state = AgentTunnelState::default();
        assert!(state.connections.read().await.is_empty());
        assert!(state.channels.read().await.is_empty());
        assert!(state.pending_requests.read().await.is_empty());
        assert!(state.tunnel_data.read().await.is_empty());
    }

    #[test]
    fn test_connect_response_pending() {
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        let mut pending = HashMap::new();
        pending.insert("req_1".to_string(), tx);

        // Simulate Agent response
        let resp = ConnectResponse {
            channel_id: Some("ch_1".to_string()),
            error: None,
        };
        let sender = pending.remove("req_1").unwrap();
        sender.send(resp).unwrap();

        let result = rx.try_recv().unwrap();
        assert_eq!(result.channel_id.unwrap(), "ch_1");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_connect_response_error() {
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        let mut pending = HashMap::new();
        pending.insert("req_2".to_string(), tx);

        let resp = ConnectResponse {
            channel_id: None,
            error: Some("connection refused".into()),
        };
        let sender = pending.remove("req_2").unwrap();
        sender.send(resp).unwrap();

        let result = rx.try_recv().unwrap();
        assert!(result.channel_id.is_none());
        assert_eq!(result.error.unwrap(), "connection refused");
    }
}
