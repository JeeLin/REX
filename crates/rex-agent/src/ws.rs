use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::{interval, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage {
    pub msg_type: String,
    pub payload: serde_json::Value,
}

pub struct AgentWs {
    server_ws_url: String,
    agent_id: String,
    token: String,
    version: String,
}

impl AgentWs {
    pub fn new(server_ws_url: String, agent_id: String, token: String, version: String) -> Self {
        Self {
            server_ws_url,
            agent_id,
            token,
            version,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut backoff = Duration::from_secs(1);

        loop {
            match self.connect_and_run().await {
                Ok(()) => {
                    tracing::info!("websocket connection closed normally");
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(error = %e, backoff_secs = backoff.as_secs(), "websocket error, reconnecting");
                    tokio::time::sleep(backoff).await;
                    backoff = std::cmp::min(backoff * 2, Duration::from_secs(60));
                }
            }
        }
    }

    async fn connect_and_run(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.server_ws_url)
            .await
            .context("failed to connect websocket")?;

        let (mut write, mut read) = ws_stream.split();

        // Send auth message
        let auth_msg = WsMessage {
            msg_type: "auth".to_string(),
            payload: serde_json::json!({
                "agent_id": self.agent_id,
                "token": self.token,
            }),
        };
        let auth_json = serde_json::to_string(&auth_msg)?;
        write.send(Message::Text(auth_json)).await?;

        tracing::info!("sent auth message, waiting for response");

        // Wait for auth_ack
        if let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                let resp: WsMessage = serde_json::from_str(&text)?;
                if resp.msg_type == "disconnect" {
                    anyhow::bail!("auth rejected: {:?}", resp.payload);
                }
                tracing::info!("auth acknowledged");
            }
        }

        // Start heartbeat
        let version = self.version.clone();
        let agent_id = self.agent_id.clone();
        let mut heartbeat_timer = interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = heartbeat_timer.tick() => {
                    let heartbeat = WsMessage {
                        msg_type: "heartbeat".to_string(),
                        payload: serde_json::json!({
                            "agent_id": agent_id,
                            "version": version,
                        }),
                    };
                    let json = serde_json::to_string(&heartbeat)?;
                    if write.send(Message::Text(json)).await.is_err() {
                        anyhow::bail!("failed to send heartbeat");
                    }
                }
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            let ws_msg: WsMessage = serde_json::from_str(&text)?;
                            match ws_msg.msg_type.as_str() {
                                "heartbeat_ack" => {
                                    // OK
                                }
                                "disconnect" => {
                                    tracing::warn!("received disconnect from hub");
                                    break;
                                }
                                _ => {
                                    tracing::debug!(msg_type = %ws_msg.msg_type, "unknown message type");
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            tracing::info!("websocket closed by server");
                            break;
                        }
                        Some(Err(e)) => {
                            anyhow::bail!("websocket error: {}", e);
                        }
                        None => {
                            tracing::info!("websocket stream ended");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_message_serialization() {
        let msg = WsMessage {
            msg_type: "heartbeat".to_string(),
            payload: serde_json::json!({"agent_id": "agt_123"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.msg_type, "heartbeat");
    }

    #[test]
    fn ws_message_types() {
        for msg_type in &["auth", "heartbeat", "heartbeat_ack", "disconnect"] {
            let msg = WsMessage {
                msg_type: msg_type.to_string(),
                payload: serde_json::json!({}),
            };
            let json = serde_json::to_string(&msg).unwrap();
            let parsed: WsMessage = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.msg_type, *msg_type);
        }
    }
}
