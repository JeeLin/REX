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
    #[serde(default)]
    name: Option<String>,
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
#[derive(Debug, Default)]
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
    /// (channel_id:seq) → session_response sender（v0.70.6 子任务 #7：协议会话请求-响应匹配）
    pub session_responses: RwLock<HashMap<String, oneshot::Sender<SessionRelay>>>,
    /// session_request 序号分配器
    pub session_seq: std::sync::atomic::AtomicU64,
    /// channel_id → 探测出的 subclass（v0.70.7：Agent 侧探测后回传，Hub 据此持久化资源 subtype）
    pub session_db_type: RwLock<HashMap<String, String>>,
}

/// 一次协议会话请求（Hub → Agent → Hub）的响应载体。
pub struct SessionRelay {
    pub data: serde_json::Value,
    pub error: Option<String>,
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
            session_responses: RwLock::new(HashMap::new()),
            session_seq: std::sync::atomic::AtomicU64::new(1),
            session_db_type: RwLock::new(HashMap::new()),
        }
    }
}

/// 在 Agent 侧发起一次协议会话（sql/redis/s3/sftp），返回隧道 channel_id。
///
/// 复用既有的 `pending_requests` 握手：Hub 下发 `connect`，Agent 回 `SessionOpened`
/// （v0.70.6 agent 侧已改为回 `session_opened` 完成握手）。
pub async fn open_agent_session(
    state: &AppState,
    agent_id: &str,
    resource_id: &str,
    protocol: &str,
    config: serde_json::Value,
) -> anyhow::Result<String> {
    let agent_conn = {
        let conns = state.agent_tunnel.connections.read().await;
        conns.get(agent_id).cloned()
    };
    let agent_conn = match agent_conn {
        Some(c) => c,
        None => anyhow::bail!("agent {agent_id} not connected"),
    };

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
            "protocol": protocol,
            "config": config,
        }
    });
    if agent_conn
        .sender
        .send(AgentEvent::Text(connect_msg.to_string()))
        .await
        .is_err()
    {
        anyhow::bail!("failed to contact agent");
    }

    let channel_id = match tokio::time::timeout(std::time::Duration::from_secs(10), resp_rx).await {
        Ok(Ok(ConnectResponse {
            channel_id: Some(id),
            ..
        })) => id,
        Ok(Ok(ConnectResponse {
            channel_id: None,
            error,
        })) => anyhow::bail!("agent connect failed: {}", error.unwrap_or_default()),
        Ok(Err(_)) => anyhow::bail!("agent connect channel dropped"),
        Err(_) => anyhow::bail!("agent connect timeout"),
    };
    Ok(channel_id)
}

/// v0.70.7：消费 Agent 在 `session_opened` 中回传的探测子类（mysql/postgresql/sqlite）。
///
/// Agent 侧探测成功后通过 `SessionOpened.subtype` 上报，Hub 据此回写资源 subtype 缓存。
/// 取走即删除（一次性语义）。
pub async fn take_session_subtype(state: &AppState, channel_id: &str) -> Option<String> {
    state
        .agent_tunnel
        .session_db_type
        .write()
        .await
        .remove(channel_id)
}

/// 经隧道向 Agent 下发一次协议子请求，并等待其 `session_response` 回传。
///
/// 二进制帧结构：`[4B u32 channelId BE][json(SessionRequest 内层结构)]`。
/// Agent 侧按 channel_id 多路到对应会话处理循环。
pub async fn agent_session_request(
    state: &AppState,
    channel_id: &str,
    kind: &str,
    payload: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let agent_id = {
        let ch = state.agent_tunnel.channels.read().await;
        ch.get(channel_id).cloned()
    };
    let agent_id = match agent_id {
        Some(a) => a,
        None => anyhow::bail!("agent tunnel channel {channel_id} not found"),
    };
    let sender = {
        let conns = state.agent_tunnel.connections.read().await;
        conns.get(&agent_id).map(|c| c.sender.clone())
    };
    let sender = match sender {
        Some(s) => s,
        None => anyhow::bail!("agent {agent_id} not connected"),
    };

    let seq = state
        .agent_tunnel
        .session_seq
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let key = format!("{channel_id}:{seq}");
    let (tx, rx) = oneshot::channel::<SessionRelay>();
    {
        let mut m = state.agent_tunnel.session_responses.write().await;
        m.insert(key.clone(), tx);
    }

    let req = rex_common::agent_proto::SessionRequest {
        channel_id: channel_id.to_string(),
        kind: kind.to_string(),
        seq,
        payload,
    };
    let bytes = serde_json::to_vec(&req)?;
    let cid: u32 = channel_id
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid numeric channel_id {channel_id}"))?;
    let mut frame = cid.to_be_bytes().to_vec();
    frame.extend_from_slice(&bytes);

    if sender.send(AgentEvent::Bytes(frame)).await.is_err() {
        let mut m = state.agent_tunnel.session_responses.write().await;
        m.remove(&key);
        anyhow::bail!("failed to send to agent");
    }

    match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(Ok(r)) => {
            if let Some(e) = r.error {
                anyhow::bail!(e);
            }
            Ok(r.data)
        }
        Ok(Err(_)) => anyhow::bail!("session relay dropped"),
        Err(_) => {
            let mut m = state.agent_tunnel.session_responses.write().await;
            m.remove(&key);
            anyhow::bail!("session request timeout")
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

    // 2. 通过注册令牌查找环境，自动注册 Agent
    let db = state.db.clone();
    let token_for_lookup = auth_msg.token.clone();
    let env_result = tokio::task::spawn_blocking(move || {
        db.find_environment_by_registration_token(&token_for_lookup)
    })
    .await;
    let env = match env_result {
        Ok(Ok(Some(env))) => env,
        _ => {
            if let Ok(fail) = serde_json::to_string(&HubMsg::AuthFail {
                payload: AuthFailPayload {
                    reason: "invalid registration token".into(),
                },
            }) {
                let _ = ws_sink.send(Message::Text(fail.into())).await;
            }
            return;
        }
    };

    // 3. 查找或创建 Agent 记录
    let db = state.db.clone();
    let env_id = env.id.clone();
    let agent_id = tokio::task::spawn_blocking(move || db.find_agent_by_env_id(&env_id)).await;
    let verified_id = match agent_id {
        Ok(Ok(Some(id))) => id,
        Ok(Ok(None)) => {
            // 自动创建 Agent 记录
            let db = state.db.clone();
            let env_id = env.id.clone();
            let agent_name = auth_msg.name.clone().unwrap_or_else(|| "agent".into());
            match tokio::task::spawn_blocking(move || {
                db.create_agent(&env_id, &agent_name, "", "", "", "", "")
            })
            .await
            {
                Ok(Ok(agent)) => agent.id,
                Ok(Err(e)) => {
                    tracing::error!(error = %e, "failed to create agent");
                    if let Ok(fail) = serde_json::to_string(&HubMsg::AuthFail {
                        payload: AuthFailPayload {
                            reason: "failed to register agent".into(),
                        },
                    }) {
                        let _ = ws_sink.send(Message::Text(fail.into())).await;
                    }
                    return;
                }
                Err(e) => {
                    tracing::error!(error = %e, "failed to create agent");
                    if let Ok(fail) = serde_json::to_string(&HubMsg::AuthFail {
                        payload: AuthFailPayload {
                            reason: "internal error".into(),
                        },
                    }) {
                        let _ = ws_sink.send(Message::Text(fail.into())).await;
                    }
                    return;
                }
            }
        }
        _ => {
            if let Ok(fail) = serde_json::to_string(&HubMsg::AuthFail {
                payload: AuthFailPayload {
                    reason: "agent lookup failed".into(),
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
                } else if let Ok(session_msg) =
                    serde_json::from_str::<rex_common::agent_proto::AgentSessionMsg>(&text)
                {
                    // v0.70.6 子任务 #7：协议会话消息（session_opened / session_response /
                    // session_error）由 Agent 终结协议后回传。
                    handle_session_msg(session_msg, &agent_id, &state_clone).await;
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

            tracing::info!(
                action = "AGENT_RESOURCE_CONNECTED",
                agent_id,
                request_id,
                channel_id,
                "agent resource connected"
            );
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

            tracing::warn!(
                action = "AGENT_RESOURCE_CONNECT_FAILED",
                agent_id,
                request_id,
                error,
                "agent resource connect failed"
            );
        }
        AgentMsg::Closed {
            payload: ChannelIdPayload { channel_id },
        } => {
            let mut channels = state.agent_tunnel.channels.write().await;
            channels.remove(&channel_id);
            tracing::info!(
                action = "AGENT_CHANNEL_CLOSED",
                agent_id,
                channel_id,
                "agent channel closed"
            );
        }
        AgentMsg::Auth { .. } => {
            // 已在握手阶段处理，忽略后续 auth 消息
        }
    }
}

/// 处理 Agent 回传的协议会话消息（v0.70.6 子任务 #7）。
///
/// - `session_opened`：完成 connect 握手（等价 `Connected`），并登记 channel→agent 映射。
/// - `session_response`：按 (channel_id, seq) 路由到等待中的 `session_responses` 接收端。
/// - `session_error`：connect 级失败写入 `pending_requests`；否则按 (channel_id, seq) 路由。
async fn handle_session_msg(
    msg: rex_common::agent_proto::AgentSessionMsg,
    agent_id: &str,
    state: &AppState,
) {
    match msg {
        rex_common::agent_proto::AgentSessionMsg::SessionOpened(payload) => {
            let mut channels = state.agent_tunnel.channels.write().await;
            channels.insert(payload.channel_id.clone(), agent_id.to_string());

            let mut pending = state.agent_tunnel.pending_requests.write().await;
            if let Some(tx) = pending.remove(&payload.request_id) {
                let _ = tx.send(ConnectResponse {
                    channel_id: Some(payload.channel_id.clone()),
                    error: None,
                });
            }
            tracing::info!(
                action = "AGENT_SESSION_OPENED",
                agent_id,
                request_id = %payload.request_id,
                channel_id = %payload.channel_id,
                "agent protocol session opened"
            );
            // v0.70.7：Agent 侧探测出的子类暂存，供 open_agent_session 消费回写资源 subtype。
            if let Some(dt) = payload.subtype {
                state
                    .agent_tunnel
                    .session_db_type
                    .write()
                    .await
                    .insert(payload.channel_id.clone(), dt);
            }
        }
        rex_common::agent_proto::AgentSessionMsg::SessionResponse(payload) => {
            let key = format!("{}:{}", payload.channel_id, payload.seq);
            let mut m = state.agent_tunnel.session_responses.write().await;
            if let Some(tx) = m.remove(&key) {
                let _ = tx.send(SessionRelay {
                    data: payload.data,
                    error: payload.error,
                });
            }
        }
        rex_common::agent_proto::AgentSessionMsg::SessionError(payload) => {
            if let Some(request_id) = payload.request_id {
                let mut pending = state.agent_tunnel.pending_requests.write().await;
                if let Some(tx) = pending.remove(&request_id) {
                    let _ = tx.send(ConnectResponse {
                        channel_id: None,
                        error: Some(payload.error.clone()),
                    });
                }
            } else {
                // 子请求级错误：尝试按最近一次 seq 路由（Agent 在请求失败时通常带 request_id）。
                tracing::warn!(
                    action = "AGENT_SESSION_ERROR",
                    agent_id,
                    channel_id = %payload.channel_id,
                    error = %payload.error,
                    "agent protocol session error"
                );
            }
        }
        _ => {
            // session_open / file_chunk 由其它路径处理，此处忽略。
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
        let json = r#"{"type":"connected","payload":{"request_id":"req_1","channel_id":"1"}}"#;
        let msg: AgentMsg = serde_json::from_str(json).unwrap();
        match msg {
            AgentMsg::Connected { payload } => {
                assert_eq!(payload.request_id, "req_1");
                assert_eq!(payload.channel_id, "1");
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
            channel_id: Some("1".to_string()),
            error: None,
        };
        let sender = pending.remove("req_1").unwrap();
        sender.send(resp).unwrap();

        let result = rx.try_recv().unwrap();
        assert_eq!(result.channel_id.unwrap(), "1");
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
