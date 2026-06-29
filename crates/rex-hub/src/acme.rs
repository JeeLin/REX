use anyhow::{Context, Result};
use rustls::ServerConfig;
use rustls_acme::caches::DirCache;
use rustls_acme::{AcmeConfig, AcmeState, UseChallenge};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::{is_ip_address, AcmeConfig as HubAcmeConfig, HubConfig};

/// ACME 驱动状态（共享给 TLS 状态 API）
#[derive(Debug, Clone, serde::Serialize)]
pub struct AcmeStatus {
    /// 当前状态：requesting / ready / error
    pub status: String,
    /// 最后一次错误信息（如有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Default for AcmeStatus {
    fn default() -> Self {
        Self {
            status: "requesting".to_string(),
            error: None,
        }
    }
}

/// ACME 状态共享类型
pub type SharedAcmeStatus = Arc<RwLock<AcmeStatus>>;

/// 创建共享 ACME 状态
pub fn new_shared_acme_status() -> SharedAcmeStatus {
    Arc::new(RwLock::new(AcmeStatus::default()))
}

/// 构建 ACME AcmeState（用于 tokio::spawn 驱动证书申请）
pub fn build_acme_state(
    acme_cfg: &HubAcmeConfig,
    data_dir: &std::path::Path,
) -> Result<AcmeState<std::io::Error, std::io::Error>> {
    let cache_dir = data_dir.join("acme");
    std::fs::create_dir_all(&cache_dir)
        .with_context(|| format!("failed to create ACME cache dir: {}", cache_dir.display()))?;

    let challenge_type = if is_ip_address(&acme_cfg.domain) {
        UseChallenge::TlsAlpn01
    } else {
        UseChallenge::Http01
    };

    let contact = format!("mailto:{}", acme_cfg.email);

    let config = AcmeConfig::new(vec![acme_cfg.domain.clone()])
        .contact_push(&contact)
        .directory_lets_encrypt(!acme_cfg.staging)
        .challenge_type(challenge_type)
        .cache(DirCache::new(cache_dir));

    Ok(config.state())
}

/// 获取 domain 的 challenge 类型描述
pub fn challenge_description(domain: &str) -> &'static str {
    if is_ip_address(domain) {
        "TLS-ALPN-01"
    } else {
        "HTTP-01"
    }
}

/// 确定 TLS 模式（优先级：manual > acme > none）
pub fn determine_tls_mode(config: &HubConfig) -> TlsMode {
    // 1. 手动证书（最高优先级）— 需验证文件存在
    if let Some(ref tls) = config.tls {
        if !tls.cert.as_os_str().is_empty()
            && !tls.key.as_os_str().is_empty()
            && tls.cert.exists()
            && tls.key.exists()
        {
            return TlsMode::Manual;
        }
    }

    // 2. ACME 自动证书
    if let Some(ref acme) = config.acme {
        if !acme.domain.is_empty() && !acme.email.is_empty() {
            if is_ip_address(&acme.domain) {
                return TlsMode::AcmeIp;
            } else {
                return TlsMode::AcmeDomain;
            }
        }
    }

    // 3. 默认：无 TLS（HTTP only）
    TlsMode::None
}

/// TLS 模式
#[derive(Debug, Clone, PartialEq)]
pub enum TlsMode {
    /// 手动证书（tls.cert + tls.key）
    Manual,
    /// ACME 域名证书（HTTP-01）
    AcmeDomain,
    /// ACME IP 证书（TLS-ALPN-01）
    AcmeIp,
    /// 无 TLS（HTTP only）
    None,
}

impl std::fmt::Display for TlsMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsMode::Manual => write!(f, "manual"),
            TlsMode::AcmeDomain => write!(f, "acme-domain"),
            TlsMode::AcmeIp => write!(f, "acme-ip"),
            TlsMode::None => write!(f, "none"),
        }
    }
}

/// 启动 ACME 后台驱动任务
///
/// 驱动 AcmeState stream，处理证书申请和续期。
/// 返回 `(default_config, challenge_config)` 用于构建 TLS 服务器。
///
/// `shared_status` 用于向 TLS 状态 API 报告 ACME 当前状态。
/// ACME stream 结束后最多重试 3 次。
pub async fn start_acme_driver(
    acme_cfg: HubAcmeConfig,
    data_dir: std::path::PathBuf,
    shared_status: SharedAcmeStatus,
) -> Result<(
    Arc<ServerConfig>,
    Option<Arc<ServerConfig>>,
    Option<rustls_acme::tower::TowerHttp01ChallengeService>,
)> {
    let state = build_acme_state(&acme_cfg, &data_dir)?;

    let default_config = state.default_rustls_config();
    let is_ip = is_ip_address(&acme_cfg.domain);

    let challenge_config = if is_ip {
        Some(state.challenge_rustls_config())
    } else {
        None
    };

    let http01_service = if !is_ip {
        Some(state.http01_challenge_tower_service())
    } else {
        None
    };

    // 后台驱动 AcmeState stream（处理证书申请和续期，最多重试 3 次）
    tokio::spawn(async move {
        use futures_util::StreamExt;

        let max_retries = 3;
        let mut attempt = 0u32;

        loop {
            let mut state = match build_acme_state(&acme_cfg, &data_dir) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(error = %e, "ACME: failed to build state");
                    let mut status = shared_status.write().await;
                    status.status = "error".to_string();
                    status.error = Some(format!("failed to build ACME state: {e}"));
                    break;
                }
            };

            tracing::info!(attempt = attempt + 1, "ACME driver started");

            loop {
                match state.next().await {
                    Some(Ok(event)) => {
                        tracing::info!(?event, "ACME event");
                        // 证书就绪时更新状态
                        if matches!(event, rustls_acme::EventOk::DeployedCachedCert | rustls_acme::EventOk::DeployedNewCert) {
                            let mut status = shared_status.write().await;
                            status.status = "ready".to_string();
                            status.error = None;
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!(error = %e, "ACME error");
                        let mut status = shared_status.write().await;
                        status.status = "error".to_string();
                        status.error = Some(e.to_string());
                    }
                    None => {
                        tracing::warn!(
                            attempt = attempt + 1,
                            "ACME stream ended"
                        );
                        break;
                    }
                }
            }

            attempt += 1;
            if attempt >= max_retries {
                tracing::error!(
                    attempts = max_retries,
                    "ACME: max retries reached, giving up"
                );
                let mut status = shared_status.write().await;
                status.status = "error".to_string();
                status.error = Some(format!(
                    "ACME stream ended after {max_retries} attempts"
                ));
                break;
            }

            tracing::info!(
                attempt = attempt + 1,
                max_retries,
                "ACME: retrying in 5 seconds"
            );
            let mut status = shared_status.write().await;
            status.status = "requesting".to_string();
            status.error = None;
            drop(status);

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    Ok((default_config, challenge_config, http01_service))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_description_domain() {
        assert_eq!(challenge_description("example.com"), "HTTP-01");
        assert_eq!(challenge_description("hub.local"), "HTTP-01");
    }

    #[test]
    fn challenge_description_ip() {
        assert_eq!(challenge_description("192.168.1.100"), "TLS-ALPN-01");
        assert_eq!(challenge_description("203.0.113.1"), "TLS-ALPN-01");
        assert_eq!(challenge_description("::1"), "TLS-ALPN-01");
    }

    #[test]
    fn build_acme_state_staging() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = HubAcmeConfig {
            domain: "example.com".to_string(),
            email: "admin@example.com".to_string(),
            staging: true,
            http_port: 80,
        };
        let state = build_acme_state(&cfg, dir.path()).unwrap();
        drop(state);
    }

    #[test]
    fn build_acme_state_ip_uses_tls_alpn() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = HubAcmeConfig {
            domain: "192.168.1.100".to_string(),
            email: "admin@example.com".to_string(),
            staging: true,
            http_port: 80,
        };
        let state = build_acme_state(&cfg, dir.path()).unwrap();
        drop(state);
    }

    #[test]
    fn determine_tls_mode_none_when_no_config() {
        let config = crate::config::HubConfig::default();
        assert_eq!(determine_tls_mode(&config), TlsMode::None);
    }

    #[test]
    fn determine_tls_mode_manual_takes_priority() {
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        std::fs::write(&cert, "cert").unwrap();
        std::fs::write(&key, "key").unwrap();
        let config = crate::config::HubConfig {
            tls: Some(crate::config::TlsConfig { cert, key }),
            acme: Some(HubAcmeConfig {
                domain: "hub.example.com".to_string(),
                email: "admin@example.com".to_string(),
                staging: false,
                http_port: 80,
            }),
            ..Default::default()
        };
        assert_eq!(determine_tls_mode(&config), TlsMode::Manual);
    }

    #[test]
    fn determine_tls_mode_manual_missing_files_falls_back() {
        let config = crate::config::HubConfig {
            tls: Some(crate::config::TlsConfig {
                cert: std::path::PathBuf::from("/nonexistent/cert.pem"),
                key: std::path::PathBuf::from("/nonexistent/key.pem"),
            }),
            ..Default::default()
        };
        assert_eq!(determine_tls_mode(&config), TlsMode::None);
    }

    #[test]
    fn determine_tls_mode_acme_domain() {
        let config = crate::config::HubConfig {
            acme: Some(HubAcmeConfig {
                domain: "hub.example.com".to_string(),
                email: "admin@example.com".to_string(),
                staging: false,
                http_port: 80,
            }),
            ..Default::default()
        };
        assert_eq!(determine_tls_mode(&config), TlsMode::AcmeDomain);
    }

    #[test]
    fn determine_tls_mode_acme_ip() {
        let config = crate::config::HubConfig {
            acme: Some(HubAcmeConfig {
                domain: "203.0.113.1".to_string(),
                email: "admin@example.com".to_string(),
                staging: false,
                http_port: 80,
            }),
            ..Default::default()
        };
        assert_eq!(determine_tls_mode(&config), TlsMode::AcmeIp);
    }

    #[test]
    fn acme_status_default() {
        let status = AcmeStatus::default();
        assert_eq!(status.status, "requesting");
        assert!(status.error.is_none());
    }

    #[test]
    fn acme_status_serializes() {
        let status = AcmeStatus {
            status: "ready".to_string(),
            error: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("ready"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn acme_status_with_error_serializes() {
        let status = AcmeStatus {
            status: "error".to_string(),
            error: Some("rate limit".to_string()),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("rate limit"));
    }

    #[test]
    fn tls_mode_display() {
        assert_eq!(TlsMode::Manual.to_string(), "manual");
        assert_eq!(TlsMode::AcmeDomain.to_string(), "acme-domain");
        assert_eq!(TlsMode::AcmeIp.to_string(), "acme-ip");
        assert_eq!(TlsMode::None.to_string(), "none");
    }
}
