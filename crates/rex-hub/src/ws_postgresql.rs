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
use rex_common::sql::SqlConnector;
use rex_postgresql::PostgresConnector;

// ── WebSocket 消息协议（客户端 → Hub）─────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum PostgresClientMsg {
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
pub enum PostgresServerMsg {
    /// 操作结果
    #[serde(rename = "response")]
    Response { id: String, data: serde_json::Value },
    /// 操作错误
    #[serde(rename = "error")]
    Error { id: String, message: String },
    /// 心跳响应
    #[serde(rename = "pong")]
    Pong,
    /// 已连接到 PostgreSQL
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

/// GET /ws/postgresql/:resource_id?token=xxx — PostgreSQL WebSocket 通道
pub async fn postgresql_ws_handler(
    ws: WebSocketUpgrade,
    Path(resource_id): Path<String>,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = query.token.ok_or(StatusCode::UNAUTHORIZED)?;
    if !auth::verify_token(&state.secret_key, &token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_postgresql_socket(socket, resource_id, state)))
}

async fn handle_postgresql_socket(socket: WebSocket, resource_id: String, state: Arc<AppState>) {
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

    // 2. 解析 PostgreSQL 配置
    let pg_config = match PostgresConnector::from_json(&config_json) {
        Ok(_connector) => {
            let json_val: serde_json::Value =
                serde_json::from_str(&config_json).unwrap_or(serde_json::Value::Null);
            rex_postgresql::PostgresConfig {
                host: json_val["host"].as_str().unwrap_or("localhost").to_string(),
                port: json_val["port"].as_u64().unwrap_or(5432) as u16,
                user: json_val["user"].as_str().unwrap_or("postgres").to_string(),
                password: json_val["password"].as_str().unwrap_or("").to_string(),
                database: json_val["database"].as_str().map(|s| s.to_string()),
            }
        }
        Err(e) => {
            let _ = send_ws_error(&mut ws_write, &format!("PostgreSQL 配置解析失败: {e}")).await;
            return;
        }
    };

    // 3. 建立 PostgreSQL 连接
    let mut connector = PostgresConnector::new(pg_config.clone());
    if let Err(e) = connector.connect().await {
        let _ = send_ws_error(&mut ws_write, &format!("PostgreSQL 连接失败: {e}")).await;
        return;
    }

    // 4. 构建服务器信息
    let mut server_info = HashMap::new();
    server_info.insert("host".into(), pg_config.host.clone());
    server_info.insert("port".into(), pg_config.port.to_string());
    server_info.insert("user".into(), pg_config.user.clone());
    if let Some(ref db) = pg_config.database {
        server_info.insert("database".into(), db.clone());
    }

    // 5. 发送 connected 消息
    let connected = PostgresServerMsg::Connected {
        server: server_info,
    };
    if send_ws_msg(&mut ws_write, &connected).await.is_err() {
        return;
    }

    tracing::info!(resource_id = %resource_id, "postgresql websocket connected");

    // 6. 消息循环
    loop {
        tokio::select! {
            ws_msg = ws_read.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<PostgresClientMsg>(&text) {
                            Ok(PostgresClientMsg::Command { id, action, params }) => {
                                let result = handle_postgres_action(&connector, &action, &params).await;
                                let msg = match result {
                                    Ok(data) => PostgresServerMsg::Response { id, data },
                                    Err(e) => PostgresServerMsg::Error {
                                        id,
                                        message: e.to_string(),
                                    },
                                };
                                if send_ws_msg(&mut ws_write, &msg).await.is_err() {
                                    break;
                                }
                            }
                            Ok(PostgresClientMsg::Ping) => {
                                if send_ws_msg(&mut ws_write, &PostgresServerMsg::Pong).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "invalid postgresql ws message");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(resource_id = %resource_id, "postgresql websocket closed");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // 7. 清理
    let _ = connector.close().await;
    tracing::info!(resource_id = %resource_id, "postgresql websocket disconnected");
}

// ── PostgreSQL 操作分发 ────────────────────────────────────

async fn handle_postgres_action(
    connector: &PostgresConnector,
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
        "databases" => {
            let databases = connector.list_databases().await?;
            Ok(serde_json::json!({ "databases": databases }))
        }
        "tables" => {
            let database = params
                .get("database")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'database' param"))?;
            let tables = connector.list_tables(database).await?;
            Ok(serde_json::json!({ "tables": tables }))
        }
        "columns" => {
            let database = params
                .get("database")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'database' param"))?;
            let table = params
                .get("table")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'table' param"))?;
            let columns = connector.list_columns(database, table).await?;
            Ok(serde_json::to_value(columns)?)
        }
        _ => Err(anyhow::anyhow!("unknown action: {action}")),
    }
}

// ── 工具函数 ──────────────────────────────────────────────

async fn send_ws_msg(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &PostgresServerMsg,
) -> Result<(), ()> {
    let json = serde_json::to_string(msg).map_err(|_| ())?;
    write.send(Message::Text(json)).await.map_err(|_| ())
}

async fn send_ws_error(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &str,
) -> Result<(), ()> {
    let err = PostgresServerMsg::Error {
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
    fn postgres_client_msg_command_deserialize() {
        let json =
            r#"{"type":"command","id":"cmd-1","action":"execute","params":{"sql":"SELECT 1"}}"#;
        let msg: PostgresClientMsg = serde_json::from_str(json).unwrap();
        match msg {
            PostgresClientMsg::Command { id, action, params } => {
                assert_eq!(id, "cmd-1");
                assert_eq!(action, "execute");
                assert_eq!(params["sql"], "SELECT 1");
            }
            _ => panic!("expected Command variant"),
        }
    }

    #[test]
    fn postgres_client_msg_ping_deserialize() {
        let json = r#"{"type":"ping"}"#;
        let msg: PostgresClientMsg = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, PostgresClientMsg::Ping));
    }

    #[test]
    fn postgres_server_msg_response_serialize() {
        let msg = PostgresServerMsg::Response {
            id: "cmd-1".into(),
            data: serde_json::json!({ "columns": [], "rows": [], "affected_rows": 0, "elapsed_ms": 1 }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("response"));
        assert!(json.contains("cmd-1"));
    }

    #[test]
    fn postgres_server_msg_error_serialize() {
        let msg = PostgresServerMsg::Error {
            id: "cmd-2".into(),
            message: "connection refused".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "error");
        assert_eq!(parsed["message"].as_str().unwrap(), "connection refused");
    }

    #[test]
    fn postgres_server_msg_connected_serialize() {
        let mut server = HashMap::new();
        server.insert("host".into(), "127.0.0.1".into());
        server.insert("port".into(), "5432".into());
        let msg = PostgresServerMsg::Connected { server };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("connected"));
        assert!(json.contains("host"));
    }

    #[test]
    fn postgres_server_msg_pong_serialize() {
        let msg = PostgresServerMsg::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"pong"}"#);
    }
}
