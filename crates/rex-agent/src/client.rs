use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub id: String,
    pub token: String,
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub os: String,
    pub arch: String,
    pub hostname: Option<String>,
    pub os_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterResponse {
    pub id: String,
    pub environment_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiEnvelope<T> {
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub error: ErrorBody,
}

#[derive(Debug, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

pub struct HubClient {
    http: reqwest::Client,
    server: String,
}

impl HubClient {
    pub fn new(server: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            server: server.trim_end_matches('/').to_string(),
        }
    }

    pub async fn register(&self, req: &RegisterRequest) -> Result<RegisterResponse> {
        let url = format!("{}/api/agents/register", self.server);
        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(req)
            .send()
            .await
            .context("failed to send register request")?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .context("failed to read register response")?;

        if status.is_success() {
            let envelope: ApiEnvelope<RegisterResponse> =
                serde_json::from_str(&body).context("failed to parse register response")?;
            Ok(envelope.data)
        } else {
            let api_err: ApiError = serde_json::from_str(&body).unwrap_or_else(|_| ApiError {
                error: ErrorBody {
                    code: "UNKNOWN".to_string(),
                    message: body,
                },
            });
            anyhow::bail!(
                "register failed: {} - {}",
                api_err.error.code,
                api_err.error.message
            )
        }
    }

    pub fn websocket_url(&self) -> String {
        let base = self
            .server
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        format!("{}/ws/agent", base)
    }
}

pub fn platform_info() -> (String, String, Option<String>, Option<String>) {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let hostname = hostname::get()
        .ok()
        .map(|h| h.to_string_lossy().to_string());
    let os_version = os_info();
    (os, arch, hostname, os_version)
}

fn os_info() -> Option<String> {
    // Best-effort OS version
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|c| {
            c.lines().find(|l| l.starts_with("PRETTY_NAME=")).map(|l| {
                l.trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string()
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_info_returns_values() {
        let (os, arch, _hostname, _os_version) = platform_info();
        assert!(!os.is_empty());
        assert!(!arch.is_empty());
    }

    #[test]
    fn hub_client_construction() {
        let client = HubClient::new("http://localhost:3000");
        assert_eq!(client.websocket_url(), "ws://localhost:3000/ws/agent");
    }

    #[test]
    fn hub_client_trailing_slash() {
        let client = HubClient::new("http://localhost:3000/");
        assert_eq!(client.websocket_url(), "ws://localhost:3000/ws/agent");
    }

    #[test]
    fn hub_client_https_to_wss() {
        let client = HubClient::new("https://hub.example.com");
        assert_eq!(client.websocket_url(), "wss://hub.example.com/ws/agent");
    }
}
