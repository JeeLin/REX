use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
    auto_update: bool,
    data_dir: PathBuf,
}

impl AgentWs {
    pub fn new(
        server_ws_url: String,
        agent_id: String,
        token: String,
        version: String,
        auto_update: bool,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            server_ws_url,
            agent_id,
            token,
            version,
            auto_update,
            data_dir,
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
        let auto_update = self.auto_update;
        let mut heartbeat_timer = interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = heartbeat_timer.tick() => {
                    let heartbeat = WsMessage {
                        msg_type: "heartbeat".to_string(),
                        payload: serde_json::json!({
                            "agent_id": agent_id,
                            "version": version,
                            "auto_update": auto_update,
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
                                    let needs_update = ws_msg.payload["needs_update"].as_bool().unwrap_or(false);
                                    let hub_version = ws_msg.payload["hub_version"].as_str().unwrap_or("");
                                    if needs_update && auto_update {
                                        tracing::info!(
                                            hub_version = hub_version,
                                            "update available, starting auto-update"
                                        );
                                        self.perform_update(hub_version).await;
                                    }
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

    /// Perform the auto-update sequence: download → verify SHA256 → backup → write state → exit
    async fn perform_update(&self, hub_version: &str) {
        use sha2::{Digest, Sha256};

        let hub_url = self
            .server_ws_url
            .replace("wss://", "https://")
            .replace("ws://", "http://")
            .replace("/ws/agent", "");

        // Download new binary from Hub
        let url = format!(
            "{}/api/agent/download?os={}&arch={}",
            hub_url,
            rex_common::updater::UpdateChecker::current_os(),
            rex_common::updater::UpdateChecker::current_arch(),
        );

        let client = reqwest::Client::new();
        let resp = match client.get(&url).bearer_auth(&self.token).send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, "failed to download update from hub");
                return;
            }
        };

        if !resp.status().is_success() {
            tracing::error!(status = %resp.status(), "hub returned error for download");
            return;
        }

        let expected_sha256 = resp
            .headers()
            .get("X-Agent-SHA256")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let total = resp.content_length().unwrap_or(0);
        let mut bytes = Vec::with_capacity(total as usize);

        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(c) => {
                    bytes.extend_from_slice(&c);
                }
                Err(e) => {
                    tracing::error!(error = %e, "download stream error");
                    return;
                }
            }
        }

        // SHA256 verification
        if let Some(ref expected) = expected_sha256 {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let actual = format!("{:x}", hasher.finalize());
            if actual != *expected {
                tracing::error!(expected = %expected, actual = %actual, "SHA256 verification failed");
                return;
            }
            tracing::info!("SHA256 verification passed");
        } else {
            tracing::warn!("no SHA256 checksum provided by hub, skipping verification");
        }

        // Write to staging directory
        let staging_dir = self.data_dir.join("updates").join("staging");
        if let Err(e) = std::fs::create_dir_all(&staging_dir) {
            tracing::error!(error = %e, "failed to create staging directory");
            return;
        }

        let filename = format!(
            "agent-{}-{}",
            rex_common::updater::UpdateChecker::current_os(),
            rex_common::updater::UpdateChecker::current_arch(),
        );
        let staged_path = staging_dir.join(&filename);
        if let Err(e) = std::fs::write(&staged_path, &bytes) {
            tracing::error!(error = %e, "failed to write staged binary");
            return;
        }

        // chmod +x on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&staged_path)
                .and_then(|m| Ok(m.permissions()))
                .unwrap_or_else(|_| std::fs::Permissions::from_mode(0o755));
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&staged_path, perms);
        }

        // Backup current binary
        let rollback_path = match rex_common::updater::UpdateChecker::backup_current(&self.data_dir)
        {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(error = %e, "failed to backup current binary");
                return;
            }
        };

        // Write update state
        let state = rex_common::update_state::UpdateState {
            phase: rex_common::update_state::UpdatePhase::Requested,
            target_version: hub_version.to_string(),
            old_version: self.version.clone(),
            staged_path: staged_path.to_string_lossy().to_string(),
            rollback_path: rollback_path.to_string_lossy().to_string(),
            attempt: 0,
        };

        let state_path = self.data_dir.join("update-state.json");
        if let Err(e) = state.write(&state_path) {
            tracing::error!(error = %e, "failed to write update state");
            return;
        }

        tracing::info!(version = %hub_version, "update state written, exiting for supervisor replacement");
        std::process::exit(10);
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

    #[test]
    fn heartbeat_ack_needs_update_parsed() {
        let msg = WsMessage {
            msg_type: "heartbeat_ack".to_string(),
            payload: serde_json::json!({
                "hub_version": "v0.21.0",
                "needs_update": true
            }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.msg_type, "heartbeat_ack");
        assert_eq!(parsed.payload["needs_update"].as_bool(), Some(true));
        assert_eq!(parsed.payload["hub_version"].as_str(), Some("v0.21.0"));
    }

    #[test]
    fn heartbeat_ack_no_update() {
        let msg = WsMessage {
            msg_type: "heartbeat_ack".to_string(),
            payload: serde_json::json!({
                "hub_version": "v0.20.0",
                "needs_update": false
            }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.payload["needs_update"].as_bool(), Some(false));
    }
}
