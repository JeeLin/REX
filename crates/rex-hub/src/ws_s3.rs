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
use rex_s3::S3Connector;

// ── WebSocket 消息协议（客户端 → Hub）─────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum S3ClientMsg {
    /// 执行 S3 操作
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
pub enum S3ServerMsg {
    /// 操作结果
    #[serde(rename = "response")]
    Response { id: String, data: serde_json::Value },
    /// 操作错误
    #[serde(rename = "error")]
    Error { id: String, message: String },
    /// 心跳响应
    #[serde(rename = "pong")]
    Pong,
    /// 已连接到 S3 服务
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

/// GET /ws/s3/:resource_id?token=xxx — S3 WebSocket 通道
pub async fn s3_ws_handler(
    ws: WebSocketUpgrade,
    Path(resource_id): Path<String>,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = query.token.ok_or(StatusCode::UNAUTHORIZED)?;
    if !auth::verify_token(&state.secret_key, &token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_s3_socket(socket, resource_id, state)))
}

async fn handle_s3_socket(socket: WebSocket, resource_id: String, state: Arc<AppState>) {
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

    // 2. 解析 S3 配置
    let s3_config = match rex_s3::S3ConnectorImpl::from_json(&config_json) {
        Ok(connector) => connector.into_config(),
        Err(e) => {
            let _ = send_ws_error(&mut ws_write, &format!("S3 配置解析失败: {e}")).await;
            return;
        }
    };

    // 3. 建立 S3 连接（stub — 实际通过 Agent 代理）
    let mut connector = rex_s3::S3ConnectorImpl::new(s3_config.clone());
    if let Err(e) = connector.connect().await {
        let _ = send_ws_error(&mut ws_write, &format!("S3 连接失败: {e}")).await;
        return;
    }

    // 4. 构建服务器信息
    let bucket_count = match connector.list_buckets().await {
        Ok(buckets) => buckets.len(),
        Err(_) => 0,
    };
    let mut server_info = HashMap::new();
    server_info.insert("endpoint".into(), s3_config.endpoint.clone());
    server_info.insert("buckets".into(), bucket_count.to_string());

    // 5. 发送 connected 消息
    let connected = S3ServerMsg::Connected {
        server: server_info,
    };
    if send_ws_msg(&mut ws_write, &connected).await.is_err() {
        return;
    }

    tracing::info!(resource_id = %resource_id, "s3 websocket connected");

    // 6. 消息循环
    loop {
        tokio::select! {
            ws_msg = ws_read.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<S3ClientMsg>(&text) {
                            Ok(S3ClientMsg::Command { id, action, params }) => {
                                let result = handle_s3_action(&connector as &dyn S3Connector, &action, &params).await;
                                let msg = match result {
                                    Ok(data) => S3ServerMsg::Response { id, data },
                                    Err(e) => S3ServerMsg::Error {
                                        id,
                                        message: e.to_string(),
                                    },
                                };
                                if send_ws_msg(&mut ws_write, &msg).await.is_err() {
                                    break;
                                }
                            }
                            Ok(S3ClientMsg::Ping) => {
                                if send_ws_msg(&mut ws_write, &S3ServerMsg::Pong).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "invalid s3 ws message");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(resource_id = %resource_id, "s3 websocket closed");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // 7. 清理
    let _ = connector.close().await;
    tracing::info!(resource_id = %resource_id, "s3 websocket disconnected");
}

// ── S3 操作分发 ────────────────────────────────────────

async fn handle_s3_action(
    connector: &dyn S3Connector,
    action: &str,
    params: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    match action {
        "buckets" => {
            let buckets = connector.list_buckets().await?;
            Ok(serde_json::to_value(buckets)?)
        }
        "objects" => {
            let bucket = params
                .get("bucket")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'bucket' param"))?;
            let prefix = params.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
            let objects = connector.list_objects(bucket, prefix).await?;
            Ok(serde_json::to_value(objects)?)
        }
        "info" => {
            let bucket = params
                .get("bucket")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'bucket' param"))?;
            let key = params
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'key' param"))?;
            let info = connector.get_object_info(bucket, key).await?;
            Ok(serde_json::to_value(info)?)
        }
        "upload" => {
            let bucket = params
                .get("bucket")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'bucket' param"))?;
            let key = params
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'key' param"))?;
            let data_b64 = params
                .get("data")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'data' param"))?;
            let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_b64)
                .map_err(|e| anyhow::anyhow!("invalid base64 data: {e}"))?;
            connector.upload_object(bucket, key, data).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "download" => {
            let bucket = params
                .get("bucket")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'bucket' param"))?;
            let key = params
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'key' param"))?;
            let data = connector.download_object(bucket, key).await?;
            let data_b64 =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
            Ok(serde_json::json!({ "data": data_b64 }))
        }
        "delete" => {
            let bucket = params
                .get("bucket")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'bucket' param"))?;
            let key = params
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'key' param"))?;
            connector.delete_object(bucket, key).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        _ => Err(anyhow::anyhow!("unknown action: {action}")),
    }
}

// ── 工具函数 ──────────────────────────────────────────────

async fn send_ws_msg(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &S3ServerMsg,
) -> Result<(), ()> {
    let json = serde_json::to_string(msg).map_err(|_| ())?;
    write.send(Message::Text(json)).await.map_err(|_| ())
}

async fn send_ws_error(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &str,
) -> Result<(), ()> {
    let err = S3ServerMsg::Error {
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
    fn s3_client_msg_command_deserialize() {
        let json = r#"{"type":"command","id":"cmd-1","action":"buckets","params":{}}"#;
        let msg: S3ClientMsg = serde_json::from_str(json).unwrap();
        match msg {
            S3ClientMsg::Command { id, action, params } => {
                assert_eq!(id, "cmd-1");
                assert_eq!(action, "buckets");
                assert_eq!(params, serde_json::json!({}));
            }
            _ => panic!("expected Command variant"),
        }
    }

    #[test]
    fn s3_client_msg_command_with_params_deserialize() {
        let json = r#"{"type":"command","id":"cmd-2","action":"objects","params":{"bucket":"my-bucket","prefix":"images/"}}"#;
        let msg: S3ClientMsg = serde_json::from_str(json).unwrap();
        match msg {
            S3ClientMsg::Command { id, action, params } => {
                assert_eq!(id, "cmd-2");
                assert_eq!(action, "objects");
                assert_eq!(params["bucket"], "my-bucket");
                assert_eq!(params["prefix"], "images/");
            }
            _ => panic!("expected Command variant"),
        }
    }

    #[test]
    fn s3_client_msg_ping_deserialize() {
        let json = r#"{"type":"ping"}"#;
        let msg: S3ClientMsg = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, S3ClientMsg::Ping));
    }

    #[test]
    fn s3_server_msg_response_serialize() {
        let msg = S3ServerMsg::Response {
            id: "cmd-1".into(),
            data: serde_json::json!([{ "name": "my-bucket" }]),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("response"));
        assert!(json.contains("cmd-1"));
    }

    #[test]
    fn s3_server_msg_error_serialize() {
        let msg = S3ServerMsg::Error {
            id: "cmd-2".into(),
            message: "bucket not found".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "error");
        assert_eq!(parsed["message"].as_str().unwrap(), "bucket not found");
    }

    #[test]
    fn s3_server_msg_connected_serialize() {
        let mut server = HashMap::new();
        server.insert("endpoint".into(), "http://minio:9000".into());
        server.insert("buckets".into(), "5".into());
        let msg = S3ServerMsg::Connected { server };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("connected"));
        assert!(json.contains("endpoint"));
        assert!(json.contains("minio:9000"));
    }

    #[test]
    fn s3_server_msg_disconnected_serialize() {
        let msg = S3ServerMsg::Disconnected {
            reason: "connection lost".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "disconnected");
        assert_eq!(parsed["reason"].as_str().unwrap(), "connection lost");
    }

    #[test]
    fn s3_server_msg_pong_serialize() {
        let msg = S3ServerMsg::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"pong"}"#);
    }
}
