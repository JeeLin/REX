use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth;
use crate::routes::AppState;
use rex_sqlite::SqliteConnector;

// ── WebSocket 消息协议（客户端 → Hub）─────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum SqliteClientMsg {
    /// 执行 SQL 操作
    #[serde(rename = "command")]
    Command {
        id: String,
        action: String,
        #[serde(default)]
        params: serde_json::Value,
    },
    /// 心跳
    #[serde(rename = "ping")]
    Ping,
}

/// WebSocket 消息协议（Hub → 客户端）
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum SqliteServerMsg {
    /// 操作结果
    #[serde(rename = "response")]
    Response { id: String, data: serde_json::Value },
    /// 操作错误
    #[serde(rename = "error")]
    Error { id: String, message: String },
    /// 心跳响应
    #[serde(rename = "pong")]
    Pong,
    /// 已连接到 SQLite 数据库
    #[serde(rename = "connected")]
    Connected { server: HashMap<String, String> },
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

/// GET /ws/sqlite/:resource_id?token=xxx — SQLite WebSocket 通道
pub async fn sqlite_ws_handler(
    ws: WebSocketUpgrade,
    Path(resource_id): Path<String>,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = query.token.ok_or(StatusCode::UNAUTHORIZED)?;
    if !auth::verify_token(&state.secret_key, &token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_sqlite_socket(socket, resource_id, state)))
}

async fn handle_sqlite_socket(socket: WebSocket, resource_id: String, state: Arc<AppState>) {
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

    // 2. 解析 SQLite 配置
    let sqlite_config = match rex_sqlite::SqliteConnectorImpl::from_json(&config_json) {
        Ok(connector) => connector.into_config(),
        Err(e) => {
            let _ = send_ws_error(&mut ws_write, &format!("SQLite 配置解析失败: {e}")).await;
            return;
        }
    };

    // 3. 建立 SQLite 连接（stub — 实际通过 Agent 代理）
    let mut connector = rex_sqlite::SqliteConnectorImpl::new(sqlite_config.clone());
    if let Err(e) = connector.connect().await {
        let _ = send_ws_error(&mut ws_write, &format!("SQLite 连接失败: {e}")).await;
        return;
    }

    // 4. 构建服务器信息
    let mut server_info = HashMap::new();
    server_info.insert("db_path".into(), sqlite_config.db_path.clone());

    // 5. 发送 connected 消息
    let connected = SqliteServerMsg::Connected {
        server: server_info,
    };
    if send_ws_msg(&mut ws_write, &connected).await.is_err() {
        return;
    }

    tracing::info!(resource_id = %resource_id, "sqlite websocket connected");

    // 6. 消息循环
    loop {
        tokio::select! {
            ws_msg = ws_read.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<SqliteClientMsg>(&text) {
                            Ok(SqliteClientMsg::Command { id, action, params }) => {
                                let result = handle_sqlite_action(&connector as &dyn SqliteConnector, &action, &params).await;
                                let msg = match result {
                                    Ok(data) => SqliteServerMsg::Response { id, data },
                                    Err(e) => SqliteServerMsg::Error {
                                        id,
                                        message: e.to_string(),
                                    },
                                };
                                if send_ws_msg(&mut ws_write, &msg).await.is_err() {
                                    break;
                                }
                            }
                            Ok(SqliteClientMsg::Ping) => {
                                if send_ws_msg(&mut ws_write, &SqliteServerMsg::Pong).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "invalid sqlite ws message");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(resource_id = %resource_id, "sqlite websocket closed");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // 7. 清理
    let _ = connector.close().await;
    tracing::info!(resource_id = %resource_id, "sqlite websocket disconnected");
}

// ── SQLite 操作分发 ────────────────────────────────────────

async fn handle_sqlite_action(
    connector: &dyn SqliteConnector,
    action: &str,
    params: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    match action {
        "execute" => {
            let sql = params
                .get("sql")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'sql' param"))?;
            let result = connector.execute(sql).await?;
            Ok(serde_json::to_value(result)?)
        }
        "tables" => {
            let tables = connector.list_tables().await?;
            Ok(serde_json::json!({ "tables": tables }))
        }
        "columns" => {
            let table = params
                .get("table")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'table' param"))?;
            let columns = connector.get_table_info(table).await?;
            Ok(serde_json::to_value(columns)?)
        }
        _ => Err(anyhow::anyhow!("unknown action: {action}")),
    }
}

// ── 工具函数 ──────────────────────────────────────────────

async fn send_ws_msg(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &SqliteServerMsg,
) -> Result<(), ()> {
    let json = serde_json::to_string(msg).map_err(|_| ())?;
    write.send(Message::Text(json)).await.map_err(|_| ())
}

async fn send_ws_error(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &str,
) -> Result<(), ()> {
    let err = SqliteServerMsg::Error {
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
    fn sqlite_client_msg_command_deserialize() {
        let json =
            r#"{"type":"command","id":"cmd-1","action":"execute","params":{"sql":"SELECT 1"}}"#;
        let msg: SqliteClientMsg = serde_json::from_str(json).unwrap();
        match msg {
            SqliteClientMsg::Command { id, action, params } => {
                assert_eq!(id, "cmd-1");
                assert_eq!(action, "execute");
                assert_eq!(params["sql"], "SELECT 1");
            }
            _ => panic!("expected Command variant"),
        }
    }

    #[test]
    fn sqlite_client_msg_ping_deserialize() {
        let json = r#"{"type":"ping"}"#;
        let msg: SqliteClientMsg = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, SqliteClientMsg::Ping));
    }

    #[test]
    fn sqlite_server_msg_response_serialize() {
        let msg = SqliteServerMsg::Response {
            id: "cmd-1".into(),
            data: serde_json::json!({ "columns": ["id"], "rows": [[1]], "affected_rows": 0, "elapsed_ms": 1 }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("response"));
        assert!(json.contains("cmd-1"));
    }

    #[test]
    fn sqlite_server_msg_error_serialize() {
        let msg = SqliteServerMsg::Error {
            id: "cmd-2".into(),
            message: "table not found".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "error");
        assert_eq!(parsed["message"].as_str().unwrap(), "table not found");
    }

    #[test]
    fn sqlite_server_msg_connected_serialize() {
        let mut server = HashMap::new();
        server.insert("db_path".into(), "/data/app.db".into());
        let msg = SqliteServerMsg::Connected { server };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("connected"));
        assert!(json.contains("db_path"));
    }

    #[test]
    fn sqlite_server_msg_disconnected_serialize() {
        let msg = SqliteServerMsg::Disconnected {
            reason: "connection lost".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "disconnected");
        assert_eq!(parsed["reason"].as_str().unwrap(), "connection lost");
    }

    #[test]
    fn sqlite_server_msg_pong_serialize() {
        let msg = SqliteServerMsg::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"pong"}"#);
    }
}
