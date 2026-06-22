use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth;
use crate::routes::AppState;
use rex_redis::RedisConnector;

// ── REST 请求/响应类型 ────────────────────────────────────

// ── WebSocket 消息协议（客户端 → Hub）─────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum RedisClientMsg {
    /// 执行 Redis 命令
    #[serde(rename = "command")]
    Command { id: String, command: String },
    /// 心跳
    #[serde(rename = "ping")]
    Ping,
}

/// WebSocket 消息协议（Hub → 客户端）
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum RedisServerMsg {
    /// 命令执行结果
    #[serde(rename = "response")]
    Response {
        id: String,
        value: rex_redis::RedisValue,
        elapsed_ms: u64,
    },
    /// 命令执行错误
    #[serde(rename = "error")]
    Error { id: String, message: String },
    /// 心跳响应
    #[serde(rename = "pong")]
    Pong,
    /// 已连接到 Redis 服务器
    #[serde(rename = "connected")]
    Connected {
        server: std::collections::HashMap<String, String>,
    },
    /// 连接断开
    #[serde(rename = "disconnected")]
    Disconnected { reason: String },
}

// ── Query 参数 ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

// ── WebSocket handler ──────────────────────────────────────

/// GET /ws/redis/:resource_id?token=xxx — Redis WebSocket 通道
pub async fn redis_ws_handler(
    ws: WebSocketUpgrade,
    Path(resource_id): Path<String>,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = query.token.ok_or(StatusCode::UNAUTHORIZED)?;
    if !auth::verify_token(&state.secret_key, &token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_redis_socket(socket, resource_id, state)))
}

async fn handle_redis_socket(socket: WebSocket, resource_id: String, state: Arc<AppState>) {
    let (mut ws_write, mut ws_read) = socket.split();

    // 1. 从数据库读取资源配置
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

    // 2. 解析 Redis 配置
    let redis_config = match rex_redis::RedisConnectorImpl::from_json(&config_json) {
        Ok(connector) => connector.into_config(),
        Err(e) => {
            let _ = send_ws_error(&mut ws_write, &format!("Redis 配置解析失败: {e}")).await;
            return;
        }
    };

    // 3. 建立 Redis 连接（stub — 实际通过 Agent 代理）
    let mut connector = rex_redis::RedisConnectorImpl::new(redis_config.clone());
    if let Err(e) = connector.connect().await {
        let _ = send_ws_error(&mut ws_write, &format!("Redis 连接失败: {e}")).await;
        return;
    }

    // 4. 获取服务器信息
    let server_info: std::collections::HashMap<String, String> =
        connector.info().await.unwrap_or_default();

    // 5. 发送 connected 消息
    let connected = RedisServerMsg::Connected {
        server: server_info,
    };
    if send_ws_msg(&mut ws_write, &connected).await.is_err() {
        return;
    }

    tracing::info!(resource_id = %resource_id, "redis websocket connected");

    // 6. 消息循环
    loop {
        tokio::select! {
            ws_msg = ws_read.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<RedisClientMsg>(&text) {
                            Ok(RedisClientMsg::Command { id, command }) => {
                                match connector.execute(&command).await {
                                    Ok(response) => {
                                        let msg = RedisServerMsg::Response {
                                            id,
                                            value: response.value,
                                            elapsed_ms: response.elapsed_ms,
                                        };
                                        if send_ws_msg(&mut ws_write, &msg).await.is_err() {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        let msg = RedisServerMsg::Error {
                                            id,
                                            message: e.to_string(),
                                        };
                                        if send_ws_msg(&mut ws_write, &msg).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                            Ok(RedisClientMsg::Ping) => {
                                if send_ws_msg(&mut ws_write, &RedisServerMsg::Pong).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "invalid redis ws message");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(resource_id = %resource_id, "redis websocket closed");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // 7. 清理
    let _ = connector.close().await;
    tracing::info!(resource_id = %resource_id, "redis websocket disconnected");
}

// ── 工具函数 ──────────────────────────────────────────────

async fn send_ws_msg(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &RedisServerMsg,
) -> Result<(), ()> {
    let json = serde_json::to_string(msg).map_err(|_| ())?;
    write.send(Message::Text(json)).await.map_err(|_| ())
}

async fn send_ws_error(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &str,
) -> Result<(), ()> {
    let err = RedisServerMsg::Error {
        id: String::new(),
        message: msg.to_string(),
    };
    send_ws_msg(write, &err).await
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redis_client_msg_command_deserialize() {
        let json = r#"{"type":"command","id":"cmd-1","command":"GET mykey"}"#;
        let msg: RedisClientMsg = serde_json::from_str(json).unwrap();
        match msg {
            RedisClientMsg::Command { id, command } => {
                assert_eq!(id, "cmd-1");
                assert_eq!(command, "GET mykey");
            }
            _ => panic!("expected Command variant"),
        }
    }

    #[test]
    fn redis_client_msg_ping_deserialize() {
        let json = r#"{"type":"ping"}"#;
        let msg: RedisClientMsg = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, RedisClientMsg::Ping));
    }

    #[test]
    fn redis_server_msg_response_serialize() {
        let msg = RedisServerMsg::Response {
            id: "cmd-1".into(),
            value: rex_redis::RedisValue::Status("OK".into()),
            elapsed_ms: 2,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("response"));
        assert!(json.contains("cmd-1"));
        assert!(json.contains("elapsed_ms"));
    }

    #[test]
    fn redis_server_msg_error_serialize() {
        let msg = RedisServerMsg::Error {
            id: "cmd-2".into(),
            message: "ERR unknown command".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "error");
        assert_eq!(parsed["message"].as_str().unwrap(), "ERR unknown command");
    }

    #[test]
    fn redis_server_msg_connected_serialize() {
        let mut server = std::collections::HashMap::new();
        server.insert("redis_version".into(), "7.0.0".into());
        let msg = RedisServerMsg::Connected { server };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("connected"));
        assert!(json.contains("redis_version"));
    }

    #[test]
    fn redis_server_msg_disconnected_serialize() {
        let msg = RedisServerMsg::Disconnected {
            reason: "connection lost".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "disconnected");
        assert_eq!(parsed["reason"].as_str().unwrap(), "connection lost");
    }

    #[test]
    fn redis_server_msg_pong_serialize() {
        let msg = RedisServerMsg::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"pong"}"#);
    }
}
