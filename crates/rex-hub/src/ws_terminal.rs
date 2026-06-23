use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth;
use crate::helpers::{bad_request, err_resp, not_found, ApiResponse, ErrorResponse};
use crate::resource::Resource;
use crate::routes::AppState;
use crate::ssh_config::SshResourceConfig;

// ── REST 请求/响应类型 ────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub resource_id: String,
    pub cols: u32,
    pub rows: u32,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
}

// ── WebSocket 消息协议 ────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TerminalMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct TerminalOutput {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: OutputPayload,
}

#[derive(Debug, Serialize)]
pub struct OutputPayload {
    pub data: String,
}

#[derive(Debug, Serialize)]
pub struct TerminalError {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: ErrorPayload,
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct TerminalClosed {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: ClosedPayload,
}

#[derive(Debug, Serialize)]
pub struct ClosedPayload {
    pub exit_status: i32,
}

// ── Query 参数 ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

// ── REST handlers ─────────────────────────────────────────

/// POST /api/ssh/sessions — 创建终端会话占位符
pub async fn create_session_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CreateSessionResponse>>), (StatusCode, Json<ErrorResponse>)>
{
    if input.resource_id.is_empty() {
        return Err(bad_request("resource_id is required"));
    }
    if input.cols == 0 || input.rows == 0 {
        return Err(bad_request("cols and rows must be > 0"));
    }

    // 验证资源存在且是 SSH 协议
    let db = state.db.clone();
    let resource_id = input.resource_id.clone();
    let resource = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| err_resp("INTERNAL_ERROR", "pool error"))?;
        conn.query_row(
            "SELECT id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at \
             FROM resources WHERE id = ?1",
            rusqlite::params![resource_id],
            |row| Ok(Resource {
                id: row.get(0)?, environment_id: row.get(1)?, name: row.get(2)?,
                protocol: row.get(3)?, agent_id: row.get(4)?,
                config_json: row.get(5)?, status: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
            }),
        )
        .map_err(|_| not_found("RESOURCE_NOT_FOUND", "资源不存在"))
    })
    .await
    .map_err(|_| err_resp("INTERNAL_ERROR", "internal error"))??;

    if resource.protocol != "ssh" {
        return Err(bad_request("资源不是 SSH 协议"));
    }

    // 验证 SSH 配置合法
    SshResourceConfig::from_json(&resource.config_json)
        .map_err(|e| bad_request(&format!("SSH 配置无效: {}", e)))?;

    // 创建会话占位符
    let session_id = state
        .sessions
        .create_session(&resource.id, input.cols, input.rows)
        .await;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse {
            data: CreateSessionResponse { session_id },
        }),
    ))
}

/// DELETE /api/ssh/sessions/:session_id — 关闭并移除终端会话
pub async fn delete_session_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.sessions.remove_session(&session_id).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(err_resp("INTERNAL_ERROR", &e.to_string())),
    }
}

/// GET /ws/terminal/:session_id?token=xxx — WebSocket 终端数据通道
pub async fn terminal_ws_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    // WebSocket 无法设置自定义 HTTP 头，通过 query string 验证 token
    let token = query.token.ok_or(StatusCode::UNAUTHORIZED)?;
    if !auth::verify_token(&state.secret_key, &token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_terminal_socket(socket, session_id, state)))
}

// ── WebSocket 主循环 ─────────────────────────────────────

async fn handle_terminal_socket(socket: WebSocket, session_id: String, state: Arc<AppState>) {
    let (mut ws_write, mut ws_read) = socket.split();

    // 1. 从 SessionManager 取出会话（临时独占，不持锁）
    let mut session = {
        let mut sessions = state.sessions.lock().await;
        match sessions.remove(&session_id) {
            Some(s) => s,
            None => {
                let _ = send_ws_error(&mut ws_write, "session not found").await;
                return;
            }
        }
    };

    // 2. 从数据库读取资源 SSH 配置
    let resource_id = session.resource_id.clone();
    let config_json = {
        let db = state.db.clone();
        let rid = resource_id.clone();
        match tokio::task::spawn_blocking(move || {
            let conn = db.pool.get().map_err(|_| "pool error")?;
            conn.query_row(
                "SELECT config_json FROM resources WHERE id = ?1",
                rusqlite::params![rid],
                |row| row.get::<_, String>(0),
            )
            .map_err(|_| "resource not found")
        })
        .await
        {
            Ok(Ok(json)) => json,
            _ => {
                let _ = send_ws_error(&mut ws_write, "failed to read resource config").await;
                return;
            }
        }
    };

    // 3. 解密并转换为 AuthMethod
    let ssh_config = match SshResourceConfig::from_encrypted_json(&config_json, &state.secret_key) {
        Ok(c) => c,
        Err(e) => {
            let _ = send_ws_error(&mut ws_write, &format!("SSH 配置解密失败: {}", e)).await;
            return;
        }
    };

    let auth = match ssh_config.to_auth_method(&state.secret_key) {
        Ok(a) => a,
        Err(e) => {
            let _ = send_ws_error(&mut ws_write, &format!("认证方式转换失败: {}", e)).await;
            return;
        }
    };

    // 4. SSH 连接 + PTY + shell
    if let Err(e) = session
        .connect(
            &ssh_config.host,
            ssh_config.port,
            &ssh_config.username,
            auth,
        )
        .await
    {
        let _ = send_ws_error(&mut ws_write, &format!("SSH 连接失败: {}", e)).await;
        return;
    }

    if let Err(e) = session.init_shell().await {
        let _ = send_ws_error(&mut ws_write, &format!("shell 初始化失败: {}", e)).await;
        return;
    }

    tracing::info!(session_id = %session_id, "terminal session connected");

    // 5. 双向桥接：WebSocket ↔ SSH
    loop {
        tokio::select! {
            // 前端 → 后端
            ws_msg = ws_read.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(msg) = serde_json::from_str::<TerminalMessage>(&text) {
                            match msg.msg_type.as_str() {
                                "terminal.input" => {
                                    if let Some(data_b64) = msg.payload["data"].as_str() {
                                        if let Ok(data) = base64::engine::general_purpose::STANDARD.decode(data_b64) {
                                            if session.send_data(&data).await.is_err() {
                                                let _ = send_ws_error(&mut ws_write, "发送数据失败").await;
                                                break;
                                            }
                                        }
                                    }
                                }
                                "terminal.resize" => {
                                    let cols = msg.payload["cols"].as_u64().unwrap_or(80) as u32;
                                    let rows = msg.payload["rows"].as_u64().unwrap_or(24) as u32;
                                    let _ = session.resize(cols, rows).await;
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(session_id = %session_id, "WebSocket closed");
                        break;
                    }
                    Some(Ok(Message::Binary(data)))
                        if session.send_data(&data).await.is_err() =>
                    {
                        break;
                    }
                    _ => {}
                }
            }
            // 后端 → 前端
            ssh_event = session.recv() => {
                if let Some(event) = ssh_event {
                    if !event.data.is_empty() {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&event.data);
                        let output = TerminalOutput {
                            msg_type: "terminal.output".to_string(),
                            payload: OutputPayload { data: b64 },
                        };
                        if let Ok(json) = serde_json::to_string(&output) {
                            if ws_write.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    if event.closed {
                        let closed = TerminalClosed {
                            msg_type: "terminal.closed".to_string(),
                            payload: ClosedPayload { exit_status: 0 },
                        };
                        if let Ok(json) = serde_json::to_string(&closed) {
                            let _ = ws_write.send(Message::Text(json)).await;
                        }
                        break;
                    }
                    if let Some(err) = event.error {
                        let _ = send_ws_error(&mut ws_write, &err).await;
                        break;
                    }
                }
            }
        }
    }

    // 6. 清理
    let _ = session.close().await;
    tracing::info!(session_id = %session_id, "terminal session closed");
}

// ── 工具函数 ──────────────────────────────────────────────

async fn send_ws_error(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &str,
) -> Result<(), ()> {
    let err = TerminalError {
        msg_type: "terminal.error".to_string(),
        payload: ErrorPayload {
            message: msg.to_string(),
        },
    };
    let json = serde_json::to_string(&err).map_err(|_| ())?;
    write.send(Message::Text(json)).await.map_err(|_| ())
}

// ── Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_message_deserialize() {
        let json = r#"{"type":"terminal.input","payload":{"data":"aGVsbG8="}}"#;
        let msg: TerminalMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.msg_type, "terminal.input");
        assert_eq!(msg.payload["data"].as_str().unwrap(), "aGVsbG8=");
    }

    #[test]
    fn terminal_resize_deserialize() {
        let json = r#"{"type":"terminal.resize","payload":{"cols":120,"rows":40}}"#;
        let msg: TerminalMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.msg_type, "terminal.resize");
        assert_eq!(msg.payload["cols"].as_u64().unwrap(), 120);
        assert_eq!(msg.payload["rows"].as_u64().unwrap(), 40);
    }

    #[test]
    fn terminal_output_serialize() {
        let output = TerminalOutput {
            msg_type: "terminal.output".to_string(),
            payload: OutputPayload {
                data: "aGVsbG8=".to_string(),
            },
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("terminal.output"));
        assert!(json.contains("aGVsbG8="));
    }

    #[test]
    fn terminal_error_serialize() {
        let err = TerminalError {
            msg_type: "terminal.error".to_string(),
            payload: ErrorPayload {
                message: "连接失败".to_string(),
            },
        };
        let json = serde_json::to_string(&err).unwrap();
        let parsed: TerminalMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.msg_type, "terminal.error");
        assert_eq!(parsed.payload["message"].as_str().unwrap(), "连接失败");
    }

    #[test]
    fn terminal_closed_serialize() {
        let closed = TerminalClosed {
            msg_type: "terminal.closed".to_string(),
            payload: ClosedPayload { exit_status: 0 },
        };
        let json = serde_json::to_string(&closed).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "terminal.closed");
        assert_eq!(parsed["payload"]["exit_status"].as_i64().unwrap(), 0);
    }

    #[test]
    fn create_session_request_deserialize() {
        let json = r#"{"resource_id":"res_123","cols":80,"rows":24}"#;
        let req: CreateSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.resource_id, "res_123");
        assert_eq!(req.cols, 80);
        assert_eq!(req.rows, 24);
    }

    #[test]
    fn create_session_response_serialize() {
        let resp = CreateSessionResponse {
            session_id: "sess_abc".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("sess_abc"));
    }

    #[test]
    fn terminal_closed_non_zero_exit_status() {
        let closed = TerminalClosed {
            msg_type: "terminal.closed".to_string(),
            payload: ClosedPayload { exit_status: 1 },
        };
        let json = serde_json::to_string(&closed).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["payload"]["exit_status"].as_i64().unwrap(), 1);
    }
}
