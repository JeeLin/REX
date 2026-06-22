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
use rex_docker::DockerConnector;

// ── WebSocket 消息协议（客户端 → Hub）─────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum DockerClientMsg {
    /// 执行 Docker 操作
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
pub enum DockerServerMsg {
    /// 操作结果
    #[serde(rename = "response")]
    Response { id: String, data: serde_json::Value },
    /// 操作错误
    #[serde(rename = "error")]
    Error { id: String, message: String },
    /// 心跳响应
    #[serde(rename = "pong")]
    Pong,
    /// 已连接到 Docker daemon
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

/// GET /ws/docker/:resource_id?token=xxx — Docker WebSocket 通道
pub async fn docker_ws_handler(
    ws: WebSocketUpgrade,
    Path(resource_id): Path<String>,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = query.token.ok_or(StatusCode::UNAUTHORIZED)?;
    if !auth::verify_token(&state.secret_key, &token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_docker_socket(socket, resource_id, state)))
}

async fn handle_docker_socket(socket: WebSocket, resource_id: String, state: Arc<AppState>) {
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

    // 2. 解析 Docker 配置
    let docker_config = match rex_docker::DockerConnectorImpl::from_json(&config_json) {
        Ok(connector) => connector.into_config(),
        Err(e) => {
            let _ = send_ws_error(&mut ws_write, &format!("Docker 配置解析失败: {e}")).await;
            return;
        }
    };

    // 3. 建立 Docker 连接（stub — 实际通过 Agent 代理）
    let mut connector = rex_docker::DockerConnectorImpl::new(docker_config.clone());
    if let Err(e) = connector.connect().await {
        let _ = send_ws_error(&mut ws_write, &format!("Docker 连接失败: {e}")).await;
        return;
    }

    // 4. 获取系统信息
    let server_info: HashMap<String, String> = connector.info().await.unwrap_or_default();

    // 5. 发送 connected 消息
    let connected = DockerServerMsg::Connected {
        server: server_info,
    };
    if send_ws_msg(&mut ws_write, &connected).await.is_err() {
        return;
    }

    tracing::info!(resource_id = %resource_id, "docker websocket connected");

    // 6. 消息循环
    loop {
        tokio::select! {
            ws_msg = ws_read.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<DockerClientMsg>(&text) {
                            Ok(DockerClientMsg::Command { id, action, params }) => {
                                let result = handle_docker_action(&connector as &dyn DockerConnector, &action, &params).await;
                                let msg = match result {
                                    Ok(data) => DockerServerMsg::Response { id, data },
                                    Err(e) => DockerServerMsg::Error {
                                        id,
                                        message: e.to_string(),
                                    },
                                };
                                if send_ws_msg(&mut ws_write, &msg).await.is_err() {
                                    break;
                                }
                            }
                            Ok(DockerClientMsg::Ping) => {
                                if send_ws_msg(&mut ws_write, &DockerServerMsg::Pong).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "invalid docker ws message");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(resource_id = %resource_id, "docker websocket closed");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // 7. 清理
    let _ = connector.close().await;
    tracing::info!(resource_id = %resource_id, "docker websocket disconnected");
}

// ── Docker 操作分发 ────────────────────────────────────────

async fn handle_docker_action(
    connector: &dyn DockerConnector,
    action: &str,
    params: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    match action {
        "list" => {
            let all = params.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
            let containers = connector.list_containers(all).await?;
            Ok(serde_json::to_value(containers)?)
        }
        "inspect" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'id' param"))?;
            let info = connector.inspect_container(id).await?;
            Ok(info)
        }
        "start" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'id' param"))?;
            connector.start_container(id).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "stop" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'id' param"))?;
            connector.stop_container(id).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "restart" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'id' param"))?;
            connector.restart_container(id).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "remove" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'id' param"))?;
            connector.remove_container(id).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "logs" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'id' param"))?;
            let tail = params.get("tail").and_then(|v| v.as_u64()).unwrap_or(100) as u32;
            let logs = connector.logs(id, tail).await?;
            Ok(serde_json::json!({ "logs": logs }))
        }
        _ => Err(anyhow::anyhow!("unknown action: {action}")),
    }
}

// ── 工具函数 ──────────────────────────────────────────────

async fn send_ws_msg(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &DockerServerMsg,
) -> Result<(), ()> {
    let json = serde_json::to_string(msg).map_err(|_| ())?;
    write.send(Message::Text(json)).await.map_err(|_| ())
}

async fn send_ws_error(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &str,
) -> Result<(), ()> {
    let err = DockerServerMsg::Error {
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
    fn docker_client_msg_command_deserialize() {
        let json = r#"{"type":"command","id":"cmd-1","action":"list","params":{"all":true}}"#;
        let msg: DockerClientMsg = serde_json::from_str(json).unwrap();
        match msg {
            DockerClientMsg::Command { id, action, params } => {
                assert_eq!(id, "cmd-1");
                assert_eq!(action, "list");
                assert_eq!(params["all"], true);
            }
            _ => panic!("expected Command variant"),
        }
    }

    #[test]
    fn docker_client_msg_ping_deserialize() {
        let json = r#"{"type":"ping"}"#;
        let msg: DockerClientMsg = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, DockerClientMsg::Ping));
    }

    #[test]
    fn docker_server_msg_response_serialize() {
        let msg = DockerServerMsg::Response {
            id: "cmd-1".into(),
            data: serde_json::json!([{ "id": "abc123", "name": "nginx" }]),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("response"));
        assert!(json.contains("cmd-1"));
    }

    #[test]
    fn docker_server_msg_error_serialize() {
        let msg = DockerServerMsg::Error {
            id: "cmd-2".into(),
            message: "container not found".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "error");
        assert_eq!(parsed["message"].as_str().unwrap(), "container not found");
    }

    #[test]
    fn docker_server_msg_connected_serialize() {
        let mut server = HashMap::new();
        server.insert("ServerVersion".into(), "24.0.0".into());
        let msg = DockerServerMsg::Connected { server };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("connected"));
        assert!(json.contains("ServerVersion"));
    }

    #[test]
    fn docker_server_msg_disconnected_serialize() {
        let msg = DockerServerMsg::Disconnected {
            reason: "connection lost".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"].as_str().unwrap(), "disconnected");
        assert_eq!(parsed["reason"].as_str().unwrap(), "connection lost");
    }

    #[test]
    fn docker_server_msg_pong_serialize() {
        let msg = DockerServerMsg::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"pong"}"#);
    }
}
