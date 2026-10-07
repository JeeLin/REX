//! Tunnel WebSocket — 浏览器 ↔ Hub ↔ Agent ↔ 内网资源。
//!
//! 浏览器通过 /ws/tunnel 连接到 Hub，Hub 将数据通过 Agent 隧道转发到内网资源。
//! 对前端透明——和直连 SSH 的体验完全一致。

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::agent_ws::{AgentEvent, ConnectResponse};
use crate::db::{audit_log_scoped, AuditScope};
use crate::error::{send_ws_json, wire_code, ProtoKind};
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
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum TunnelMsg {
    #[serde(rename = "tunnel.connected")]
    Connected,
    #[serde(rename = "tunnel.data")]
    Data { data: String },
    /// 失败帧：`code`/`stage` 给出结构化诊断，`message` 是可直接展示的文案。
    ///
    /// 三个新字段都带缺省回落，缺 `code`/`stage` 的旧帧仍按 `message` 呈现；
    /// `retryable` 缺省按可重试处理（与 `ErrorPayload` 的缺省语义一致，
    /// 免得旧帧被误判成终止性而停掉重连）。
    #[serde(rename = "tunnel.error")]
    Error {
        #[serde(default)]
        code: String,
        #[serde(default)]
        stage: String,
        #[serde(default = "default_retryable")]
        retryable: bool,
        message: String,
    },
    #[serde(rename = "tunnel.disconnected")]
    Disconnected { reason: String },
}

fn default_retryable() -> bool {
    true
}

/// connect 握手的一个失败点。
///
/// `root` 是本侧的根因码，经 `error::wire_code` 合成为 `TUNNEL_<root>`；
/// `stage` 标识失败发生在握手的哪一步（与 Agent 侧 `STAGE_*` 同为小写单词）；
/// `retryable` 决定前端是否自动重连。
struct FailureKind {
    root: &'static str,
    stage: &'static str,
    retryable: bool,
}

/// 前端未发（或发了非 connect 的）首条消息：同一份客户端行为重连多少次都是
/// 同一结果，按终止性处理，让前端直接把 `message` 呈现给用户。
const FAIL_CLIENT_PROTOCOL: FailureKind = FailureKind {
    root: "BAD_REQUEST",
    stage: "client",
    retryable: false,
};
/// Agent 未连接：Agent 随时可能重新上线，值得重连。
const FAIL_AGENT_OFFLINE: FailureKind = FailureKind {
    root: "AGENT_NOT_CONNECTED",
    stage: "agent",
    retryable: true,
};
/// 向 Agent 下发 connect 失败：Agent 通道瞬时抖动，可重试。
const FAIL_AGENT_DISPATCH: FailureKind = FailureKind {
    root: "AGENT_SEND_FAILED",
    stage: "dispatch",
    retryable: true,
};
/// Agent 明确回错误（凭据、资源不存在等由 Agent 侧判定的问题）：文案原样透传，
/// 但不拿它当 Hub 的分类依据，且重连仍是同一结果，按终止性处理。
const FAIL_AGENT_REJECTED: FailureKind = FailureKind {
    root: "AGENT_CONNECT_REJECTED",
    stage: "agent_error",
    retryable: false,
};
/// Agent 回了响应却既无 channel_id 也无 error：Agent 侧协议异常，重连无用。
const FAIL_AGENT_EMPTY_RESPONSE: FailureKind = FailureKind {
    root: "AGENT_BAD_RESPONSE",
    stage: "agent_response",
    retryable: false,
};
/// 响应通道被关闭（Agent 崩溃 / 断连）：瞬时故障，可重试。
const FAIL_AGENT_CHANNEL_CLOSED: FailureKind = FailureKind {
    root: "AGENT_CHANNEL_CLOSED",
    stage: "agent_channel",
    retryable: true,
};
/// 等待 Agent 响应超时：内网链路慢或 Agent 卡住，可重试。
const FAIL_AGENT_TIMEOUT: FailureKind = FailureKind {
    root: "AGENT_TIMEOUT",
    stage: "timeout",
    retryable: true,
};

/// 构造结构化失败帧：`code` 走共享层的组合规则（`<TUNNEL>_<root>`），
/// `message` 为可直接展示的文案。
fn connect_failure(kind: &FailureKind, message: &str) -> TunnelMsg {
    TunnelMsg::Error {
        code: wire_code(ProtoKind::Tunnel, kind.root),
        stage: kind.stage.to_string(),
        retryable: kind.retryable,
        message: message.to_string(),
    }
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
            let _ = send_error(
                &mut ws,
                &connect_failure(&FAIL_CLIENT_PROTOCOL, "expected connect message"),
            )
            .await;
            return;
        }
    };

    tracing::info!(
        action = "TUNNEL_CONNECT",
        agent_id = %params.agent_id,
        protocol = %connect_req.protocol,
        host = %connect_req.host,
        "tunnel connect requested"
    );
    // 隧道事件的归属：Agent 由 `TunnelQuery.agent_id` 给出，资源由 `resource_id`
    // 给出。不带归属时按 Agent / 资源过滤的审计查看器查不到隧道记录。
    let scope = AuditScope {
        environment_id: None,
        resource_id: Some(params.resource_id.clone()),
        agent_id: Some(params.agent_id.clone()),
    };
    audit_log_scoped(
        &state.db,
        "TUNNEL_CONNECT",
        "success",
        Some(format!("{}@{}", connect_req.protocol, connect_req.host)),
        scope.clone(),
    );
    let audit_target = format!("{}@{}", connect_req.protocol, connect_req.host);

    // 2. 查找 Agent 连接
    let agent_conn = {
        let conns = state.agent_tunnel.connections.read().await;
        conns.get(&params.agent_id).cloned()
    };

    let agent_conn = match agent_conn {
        Some(c) => c,
        None => {
            audit_log_scoped(
                &state.db,
                "TUNNEL_CONNECT",
                "failure",
                Some(audit_target.clone()),
                scope.clone(),
            );
            let _ = send_error(
                &mut ws,
                &connect_failure(&FAIL_AGENT_OFFLINE, "agent not connected"),
            )
            .await;
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
        audit_log_scoped(
            &state.db,
            "TUNNEL_CONNECT",
            "failure",
            Some(audit_target.clone()),
            scope.clone(),
        );
        let _ = send_error(
            &mut ws,
            &connect_failure(&FAIL_AGENT_DISPATCH, "failed to send connect to agent"),
        )
        .await;
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
            audit_log_scoped(
                &state.db,
                "TUNNEL_CONNECT",
                "failure",
                Some(audit_target.clone()),
                scope.clone(),
            );
            let _ = send_error(&mut ws, &connect_failure(&FAIL_AGENT_REJECTED, &e)).await;
            return;
        }
        Ok(Ok(ConnectResponse { .. })) => {
            audit_log_scoped(
                &state.db,
                "TUNNEL_CONNECT",
                "failure",
                Some(audit_target.clone()),
                scope.clone(),
            );
            let _ = send_error(
                &mut ws,
                &connect_failure(&FAIL_AGENT_EMPTY_RESPONSE, "agent returned empty response"),
            )
            .await;
            return;
        }
        Ok(Err(_)) => {
            audit_log_scoped(
                &state.db,
                "TUNNEL_CONNECT",
                "failure",
                Some(audit_target.clone()),
                scope.clone(),
            );
            let _ = send_error(
                &mut ws,
                &connect_failure(&FAIL_AGENT_CHANNEL_CLOSED, "agent response channel closed"),
            )
            .await;
            return;
        }
        Err(_) => {
            // 超时 — 清理 pending request
            let mut pending = state.agent_tunnel.pending_requests.write().await;
            pending.remove(&request_id);
            audit_log_scoped(
                &state.db,
                "TUNNEL_CONNECT",
                "failure",
                Some(audit_target.clone()),
                scope.clone(),
            );
            let _ = send_error(
                &mut ws,
                &connect_failure(&FAIL_AGENT_TIMEOUT, "agent response timeout"),
            )
            .await;
            return;
        }
    };

    tracing::info!(action = "TUNNEL_ESTABLISHED", channel_id = %channel_id, "tunnel established");

    // 5. 通知前端连接成功
    let _ = send_ws_json(&mut ws, &TunnelMsg::Connected).await;

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
    let bytes_forwarded = total_bytes_frontend_to_agent.load(Ordering::Relaxed)
        + total_bytes_agent_to_frontend.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);
    tracing::info!(
        action = "TUNNEL_CLOSE",
        channel_id = %channel_id,
        duration_ms,
        bytes_forwarded,
        error_count = errors,
        "tunnel closed"
    );
    audit_log_scoped(
        &state.db,
        "TUNNEL_CLOSE",
        "success",
        Some(channel_id),
        scope,
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

/// 发送错误消息到前端（序列化走共享层 `error::send_ws_json`）
async fn send_error(ws: &mut WebSocket, msg: &TunnelMsg) -> Result<(), axum::Error> {
    send_ws_json(ws, msg).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(kind: &FailureKind, message: &str) -> (String, String, bool, String) {
        let TunnelMsg::Error {
            code,
            stage,
            retryable,
            message,
        } = connect_failure(kind, message)
        else {
            panic!("connect_failure always returns Error variant");
        };
        (code, stage, retryable, message)
    }

    #[test]
    fn connect_failures_have_distinct_stages_and_prefixed_codes() {
        // (失败点, 展示文案, code, stage, retryable)
        let cases: [(&FailureKind, &str, &str, &str, bool); 7] = [
            (
                &FAIL_CLIENT_PROTOCOL,
                "expected connect message",
                "TUNNEL_BAD_REQUEST",
                "client",
                false,
            ),
            (
                &FAIL_AGENT_OFFLINE,
                "agent not connected",
                "TUNNEL_AGENT_NOT_CONNECTED",
                "agent",
                true,
            ),
            (
                &FAIL_AGENT_DISPATCH,
                "failed to send connect to agent",
                "TUNNEL_AGENT_SEND_FAILED",
                "dispatch",
                true,
            ),
            (
                &FAIL_AGENT_REJECTED,
                "connection refused",
                "TUNNEL_AGENT_CONNECT_REJECTED",
                "agent_error",
                false,
            ),
            (
                &FAIL_AGENT_EMPTY_RESPONSE,
                "agent returned empty response",
                "TUNNEL_AGENT_BAD_RESPONSE",
                "agent_response",
                false,
            ),
            (
                &FAIL_AGENT_CHANNEL_CLOSED,
                "agent response channel closed",
                "TUNNEL_AGENT_CHANNEL_CLOSED",
                "agent_channel",
                true,
            ),
            (
                &FAIL_AGENT_TIMEOUT,
                "agent response timeout",
                "TUNNEL_AGENT_TIMEOUT",
                "timeout",
                true,
            ),
        ];
        for (kind, message, code, stage, retryable) in cases {
            let (got_code, got_stage, got_retryable, got_message) = fields(kind, message);
            assert_eq!(got_code, code, "code for {message}");
            assert_eq!(got_stage, stage, "stage for {message}");
            assert_eq!(got_retryable, retryable, "retryable for {message}");
            assert_eq!(got_message, message);
        }
    }

    #[test]
    fn agent_error_text_is_kept_verbatim_but_not_used_for_hub_classification() {
        // Agent 侧文案可能是任意字符串（含 "connection refused" 这类关键词），
        // 分类必须来自 Hub 侧的失败点常量。
        let agent_msg = "authentication failed: password rejected";
        let (code, stage, retryable, message) = fields(&FAIL_AGENT_REJECTED, agent_msg);
        assert_eq!(code, "TUNNEL_AGENT_CONNECT_REJECTED");
        assert_eq!(stage, "agent_error");
        assert!(!retryable);
        assert_eq!(message, agent_msg);
    }

    #[test]
    fn error_frame_serializes_with_code_and_stage() {
        let json = serde_json::to_value(connect_failure(
            &FAIL_AGENT_TIMEOUT,
            "agent response timeout",
        ))
        .unwrap();
        assert_eq!(json["type"], "tunnel.error");
        assert_eq!(json["code"], "TUNNEL_AGENT_TIMEOUT");
        assert_eq!(json["stage"], "timeout");
        assert_eq!(json["retryable"], true);
        assert_eq!(json["message"], "agent response timeout");
    }

    #[test]
    fn fatal_error_frame_serializes_retryable_false() {
        let json = serde_json::to_value(connect_failure(
            &FAIL_CLIENT_PROTOCOL,
            "expected connect message",
        ))
        .unwrap();
        assert_eq!(json["retryable"], false);
    }

    /// 隧道审计事件的归属来自握手查询参数（`TunnelQuery`）：Agent 与资源都
    /// 真实可得；环境 id 在此上下文不存在，填 None 而非编造。缺归属时按
    /// Agent / 资源过滤的审计查看器查不到任何隧道记录。
    #[test]
    fn tunnel_scope_carries_agent_and_resource_and_leaves_env_unset() {
        let params = TunnelQuery {
            agent_id: "agent-7".into(),
            resource_id: "res-9".into(),
        };
        let scope = AuditScope {
            environment_id: None,
            resource_id: Some(params.resource_id.clone()),
            agent_id: Some(params.agent_id.clone()),
        };
        assert_eq!(scope.agent_id.as_deref(), Some("agent-7"));
        assert_eq!(scope.resource_id.as_deref(), Some("res-9"));
        assert!(
            scope.environment_id.is_none(),
            "tunnel handshake carries no environment id — leave the dimension unset \
             rather than inventing one"
        );
    }

    #[test]
    fn legacy_error_frame_without_new_fields_still_parses() {
        let legacy: TunnelMsg =
            serde_json::from_str(r#"{"type":"tunnel.error","message":"agent not connected"}"#)
                .unwrap();
        let TunnelMsg::Error {
            code,
            stage,
            retryable,
            message,
        } = legacy
        else {
            panic!("legacy frame should parse as Error");
        };
        assert_eq!(code, "");
        assert_eq!(stage, "");
        assert!(
            retryable,
            "a legacy frame must not be treated as fatal and stop reconnecting"
        );
        assert_eq!(message, "agent not connected");
    }
}
