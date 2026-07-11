use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;

use crate::audit::write_audit_log;
use crate::auth;
use crate::routes::AppState;
use crate::ws_common::{self, WsClientMsg, WsQuery, WsServerMsg};
use rex_common::sql::SqlConnector;
use rex_mysql::MySqlConnector;

// ── WebSocket handler ──────────────────────────────────────

/// GET /ws/mysql/:resource_id?token=xxx — MySQL WebSocket 通道
pub async fn mysql_ws_handler(
    ws: WebSocketUpgrade,
    Path(resource_id): Path<String>,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = query.token.ok_or(StatusCode::UNAUTHORIZED)?;
    if !auth::verify_token(&state.secret_key, &token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_mysql_socket(socket, resource_id, state)))
}

async fn handle_mysql_socket(socket: WebSocket, resource_id: String, state: Arc<AppState>) {
    let (mut ws_write, mut ws_read) = socket.split();

    // 1. 从数据库读取资源配置
    let config_json = match ws_common::read_resource_config(&state, &resource_id).await {
        Ok(json) => json,
        Err(_) => {
            let _ = ws_common::send_ws_error(&mut ws_write, "failed to read resource config").await;
            return;
        }
    };

    // 2. 解析 MySQL 配置
    let mysql_config = match MySqlConnector::from_json(&config_json) {
        Ok(_connector) => {
            let json_val: serde_json::Value =
                serde_json::from_str(&config_json).unwrap_or(serde_json::Value::Null);
            rex_mysql::MySqlConfig {
                host: json_val["host"].as_str().unwrap_or("localhost").to_string(),
                port: json_val["port"].as_u64().unwrap_or(3306) as u16,
                user: json_val["user"].as_str().unwrap_or("root").to_string(),
                password: json_val["password"].as_str().unwrap_or("").to_string(),
                database: json_val["database"].as_str().map(|s| s.to_string()),
            }
        }
        Err(e) => {
            let _ =
                ws_common::send_ws_error(&mut ws_write, &format!("MySQL 配置解析失败: {e}")).await;
            return;
        }
    };

    // 3. 建立 MySQL 连接
    let mut connector = MySqlConnector::new(mysql_config.clone());
    if let Err(e) = connector.connect().await {
        let _ = ws_common::send_ws_error(&mut ws_write, &format!("MySQL 连接失败: {e}")).await;
        return;
    }

    // 4. 构建服务器信息
    let mut server_info = HashMap::new();
    server_info.insert("host".into(), mysql_config.host.clone());
    server_info.insert("port".into(), mysql_config.port.to_string());
    server_info.insert("user".into(), mysql_config.user.clone());
    if let Some(ref db) = mysql_config.database {
        server_info.insert("database".into(), db.clone());
    }

    // 5. 发送 connected 消息
    let connected = WsServerMsg::Connected {
        server: server_info,
    };
    if ws_common::send_ws_msg(&mut ws_write, &connected)
        .await
        .is_err()
    {
        return;
    }

    tracing::info!(resource_id = %resource_id, "mysql websocket connected");
    // Audit: MySQL 连接成功
    {
        let resource_name = ws_common::read_resource_name(&state, &resource_id).await;
        let name = resource_name.as_deref().unwrap_or(&resource_id);
        let detail = serde_json::json!({
            "resource_id": resource_id,
            "resource_name": name,
            "protocol": "mysql",
            "host": mysql_config.host,
            "port": mysql_config.port,
        }).to_string();
        write_audit_log(
            &state.db, "mysql_connect", "success",
            &format!("MySQL 连接成功「{}」", name),
            None, Some(&resource_id), None, Some(&detail), None,
        );
    }

    // 6. 消息循环
    loop {
        tokio::select! {
            ws_msg = ws_read.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<WsClientMsg>(&text) {
                            Ok(WsClientMsg::Command { id, action, params }) => {
                                let result = handle_mysql_action(&connector, &action, &params).await;
                                let msg = match result {
                                    Ok(data) => WsServerMsg::Response { id, data },
                                    Err(e) => WsServerMsg::Error {
                                        id,
                                        message: e.to_string(),
                                    },
                                };
                                if ws_common::send_ws_msg(&mut ws_write, &msg).await.is_err() {
                                    break;
                                }
                            }
                            Ok(WsClientMsg::Ping) => {
                                if ws_common::send_ws_msg(&mut ws_write, &WsServerMsg::Pong).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "invalid mysql ws message");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(resource_id = %resource_id, "mysql websocket closed");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // 7. 清理
    let _ = connector.close().await;
    tracing::info!(resource_id = %resource_id, "mysql websocket disconnected");
    // Audit: MySQL 断开
    {
        let detail = serde_json::json!({
            "resource_id": resource_id,
        }).to_string();
        write_audit_log(
            &state.db, "mysql_disconnect", "success",
            "MySQL 断开连接",
            None, Some(&resource_id), None, Some(&detail), None,
        );
    }
}

// ── MySQL 操作分发 ────────────────────────────────────────

async fn handle_mysql_action(
    connector: &MySqlConnector,
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

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mysql_client_msg_command_deserialize() {
        let json =
            r#"{"type":"command","id":"cmd-1","action":"execute","params":{"sql":"SELECT 1"}}"#;
        let msg: WsClientMsg = serde_json::from_str(json).unwrap();
        match msg {
            WsClientMsg::Command { id, action, params } => {
                assert_eq!(id, "cmd-1");
                assert_eq!(action, "execute");
                assert_eq!(params["sql"], "SELECT 1");
            }
            _ => panic!("expected Command variant"),
        }
    }

    #[test]
    fn mysql_client_msg_ping_deserialize() {
        let json = r#"{"type":"ping"}"#;
        let msg: WsClientMsg = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, WsClientMsg::Ping));
    }

    #[test]
    fn mysql_server_msg_response_serialize() {
        let msg = WsServerMsg::Response {
            id: "cmd-1".into(),
            data: serde_json::json!({ "columns": [], "rows": [], "affected_rows": 0, "elapsed_ms": 1 }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("response"));
        assert!(json.contains("cmd-1"));
    }

    #[test]
    fn mysql_server_msg_error_serialize() {
        let msg = WsServerMsg::Error {
            id: "cmd-2".into(),
            message: "connection refused".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "error");
        assert_eq!(parsed["message"].as_str().unwrap(), "connection refused");
    }

    #[test]
    fn mysql_server_msg_connected_serialize() {
        let mut server = HashMap::new();
        server.insert("host".into(), "127.0.0.1".into());
        server.insert("port".into(), "3306".into());
        let msg = WsServerMsg::Connected { server };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("connected"));
        assert!(json.contains("host"));
    }

    #[test]
    fn mysql_server_msg_pong_serialize() {
        let msg = WsServerMsg::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"pong"}"#);
    }
}
