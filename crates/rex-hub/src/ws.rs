use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent;
use crate::routes::AppState;

// ── WebSocket 消息类型 ─────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage {
    pub msg_type: String,
    pub payload: serde_json::Value,
}

// ── Agent 连接管理 ─────────────────────────────────────────

pub struct AgentConnection {
    pub agent_id: String,
    pub environment_id: String,
    pub version: String,
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

pub type AgentConnections = RwLock<HashMap<String, AgentConnection>>;

pub fn new_connections() -> AgentConnections {
    RwLock::new(HashMap::new())
}

// ── WebSocket upgrade handler ──────────────────────────────

pub async fn agent_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_agent_socket(socket, state))
}

async fn handle_agent_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut write, mut read) = socket.split();

    // Wait for auth message
    let auth_msg = match read.next().await {
        Some(Ok(Message::Text(text))) => serde_json::from_str::<WsMessage>(&text).ok(),
        _ => None,
    };

    let auth_msg = match auth_msg {
        Some(m) if m.msg_type == "auth" => m,
        _ => {
            let _ = send_ws_msg(
                &mut write,
                "disconnect",
                serde_json::json!({"reason": "missing auth"}),
            )
            .await;
            return;
        }
    };

    let agent_id = auth_msg.payload["agent_id"].as_str().unwrap_or("");
    let token = auth_msg.payload["token"].as_str().unwrap_or("");

    if agent_id.is_empty() || token.is_empty() {
        let _ = send_ws_msg(
            &mut write,
            "disconnect",
            serde_json::json!({"reason": "invalid auth"}),
        )
        .await;
        return;
    }

    // Verify token hash matches agent
    let token_hash = agent::hash_token(token);

    // Find agent in database and verify token
    let db = state.db.clone();
    let agent_id_owned = agent_id.to_string();
    let token_hash_owned = token_hash.clone();

    let verify_result = tokio::task::spawn_blocking(move || {
        let conn = db.pool.get().map_err(|_| "pool error")?;

        // Find agent by id
        let found_hash: Option<String> = conn
            .query_row(
                "SELECT token_hash FROM agents WHERE id = ?1",
                rusqlite::params![agent_id_owned],
                |row| row.get(0),
            )
            .ok();

        match found_hash {
            Some(h) if h == token_hash_owned => Ok::<_, String>(()),
            _ => Err("token mismatch".to_string()),
        }
    })
    .await;

    match verify_result {
        Ok(Ok(())) => {}
        _ => {
            let _ = send_ws_msg(
                &mut write,
                "disconnect",
                serde_json::json!({"reason": "auth rejected"}),
            )
            .await;
            return;
        }
    }

    // Auth successful — send ack
    if send_ws_msg(&mut write, "auth_ack", serde_json::json!({}))
        .await
        .is_err()
    {
        return;
    }

    // Set agent online
    let version = auth_msg.payload["version"].as_str().unwrap_or("unknown");
    let sha256 = auth_msg.payload["sha256"].as_str().unwrap_or("");
    agent::update_heartbeat(&state.db, agent_id, version, sha256);

    // Create shutdown channel
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Register connection
    let env_id = {
        let db_clone = state.db.clone();
        let aid = agent_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = db_clone.pool.get().ok()?;
            conn.query_row(
                "SELECT environment_id FROM agents WHERE id = ?1",
                rusqlite::params![aid],
                |row| row.get::<_, String>(0),
            )
            .ok()
        })
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
    };

    {
        let mut connections = state.connections.write().await;
        connections.insert(
            agent_id.to_string(),
            AgentConnection {
                agent_id: agent_id.to_string(),
                environment_id: env_id,
                version: version.to_string(),
                shutdown_tx,
            },
        );
    }

    tracing::info!(agent_id = %agent_id, "agent websocket connected");

    // Message loop
    let db = state.db.clone();
    let aid = agent_id.to_string();
    let connections = state.connections.clone();

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                tracing::info!(agent_id = %aid, "agent shutdown signal received");
                break;
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            match ws_msg.msg_type.as_str() {
                                "heartbeat" => {
                                    let ver = ws_msg.payload["version"].as_str().unwrap_or(&version);
                                    let sha = ws_msg.payload["sha256"].as_str().unwrap_or("");
                                    // 存储 auto_update 配置（如果 Agent 上报了）
                                    if let Some(auto_update) = ws_msg.payload.get("auto_update") {
                                        let config_json = serde_json::json!({
                                            "auto_update": auto_update.as_bool().unwrap_or(true),
                                        });
                                        agent::update_heartbeat_with_config(&db, &aid, ver, sha, &config_json.to_string());
                                    } else {
                                        agent::update_heartbeat(&db, &aid, ver, sha);
                                    }
                                    // 版本对比：Agent 版本 ≠ Hub 版本 → needs_update
                                    let hub_version = rex_common::version::VERSION;
                                    let needs_update = ver != hub_version;
                                    let _ = send_ws_msg(&mut write, "heartbeat_ack", serde_json::json!({
                                        "hub_version": hub_version,
                                        "needs_update": needs_update,
                                    })).await;
                                }
                                "disconnect" => {
                                    tracing::info!(agent_id = %aid, "agent requested disconnect");
                                    break;
                                }
                                _ => {
                                    tracing::debug!(msg_type = %ws_msg.msg_type, "unknown message from agent");
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!(agent_id = %aid, "websocket closed by agent");
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::warn!(agent_id = %aid, error = %e, "websocket error");
                        break;
                    }
                    None => {
                        tracing::info!(agent_id = %aid, "websocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // Cleanup: remove from connections, set offline
    {
        let mut connections = connections.write().await;
        connections.remove(&aid);
    }
    agent::set_agent_status(&db, &aid, "offline");
    tracing::info!(agent_id = %aid, "agent websocket disconnected, marked offline");
}

async fn send_ws_msg(
    write: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg_type: &str,
    payload: serde_json::Value,
) -> Result<(), ()> {
    let msg = WsMessage {
        msg_type: msg_type.to_string(),
        payload,
    };
    let json = serde_json::to_string(&msg).map_err(|_| ())?;
    write.send(Message::Text(json)).await.map_err(|_| ())
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_message_serialization() {
        let msg = WsMessage {
            msg_type: "heartbeat_ack".to_string(),
            payload: serde_json::json!({}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.msg_type, "heartbeat_ack");
    }

    #[test]
    fn auth_message_parsing() {
        let msg = WsMessage {
            msg_type: "auth".to_string(),
            payload: serde_json::json!({
                "agent_id": "agt_123",
                "token": "my_secret_token",
            }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.payload["agent_id"].as_str().unwrap(), "agt_123");
        assert_eq!(parsed.payload["token"].as_str().unwrap(), "my_secret_token");
    }

    #[test]
    fn new_connections_is_empty() {
        let conns = new_connections();
        assert!(conns.blocking_read().is_empty());
    }

    #[test]
    fn heartbeat_message_parsing() {
        let msg = WsMessage {
            msg_type: "heartbeat".to_string(),
            payload: serde_json::json!({
                "version": "0.11.0",
                "sha256": "abc123"
            }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.msg_type, "heartbeat");
        assert_eq!(parsed.payload["version"].as_str().unwrap(), "0.11.0");
        assert_eq!(parsed.payload["sha256"].as_str().unwrap(), "abc123");
    }

    #[test]
    fn disconnect_message_parsing() {
        let msg = WsMessage {
            msg_type: "disconnect".to_string(),
            payload: serde_json::json!({}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.msg_type, "disconnect");
    }

    #[test]
    fn agent_connection_fields() {
        use tokio::sync::oneshot;
        let (tx, _rx) = oneshot::channel();
        let conn = AgentConnection {
            agent_id: "agt_123".to_string(),
            environment_id: "env_456".to_string(),
            version: "0.11.0".to_string(),
            shutdown_tx: tx,
        };
        assert_eq!(conn.agent_id, "agt_123");
        assert_eq!(conn.environment_id, "env_456");
        assert_eq!(conn.version, "0.11.0");
    }

    #[test]
    fn new_connections_can_insert_and_read() {
        let conns = new_connections();
        use tokio::sync::oneshot;
        let (tx, _rx) = oneshot::channel();
        let conn = AgentConnection {
            agent_id: "agt_123".to_string(),
            environment_id: "env_456".to_string(),
            version: "0.11.0".to_string(),
            shutdown_tx: tx,
        };
        conns.blocking_write().insert("agt_123".to_string(), conn);
        assert_eq!(conns.blocking_read().len(), 1);
        assert!(conns.blocking_read().contains_key("agt_123"));
    }
}
