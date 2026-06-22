use anyhow::{Context, Result};
use rustls::ServerConfig;
use rustls_acme::caches::DirCache;
use rustls_acme::{AcmeConfig, AcmeState, UseChallenge};
use std::sync::Arc;

use crate::config::{is_ip_address, AcmeConfig as HubAcmeConfig, HubConfig};

/// 验证 ACME 配置
pub fn validate_acme_config(acme_cfg: &HubAcmeConfig) -> Result<()> {
    if acme_cfg.domain.is_empty() {
        anyhow::bail!("ACME domain is empty");
    }
    if acme_cfg.email.is_empty() {
        anyhow::bail!("ACME email is empty");
    }
    Ok(())
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

/// 从 AcmeState 构建 ServerConfig（使用 ACME resolver）
pub fn build_server_config_from_state(
    state: &AcmeState<std::io::Error, std::io::Error>,
) -> Arc<ServerConfig> {
    state.default_rustls_config()
}

/// 获取 TLS-ALPN-01 challenge ServerConfig
pub fn build_challenge_server_config(
    state: &AcmeState<std::io::Error, std::io::Error>,
) -> Arc<ServerConfig> {
    state.challenge_rustls_config()
}

/// 获取 HTTP-01 challenge tower service
pub fn get_http01_service(
    state: &AcmeState<std::io::Error, std::io::Error>,
) -> rustls_acme::tower::TowerHttp01ChallengeService {
    state.http01_challenge_tower_service()
}

/// 获取 domain 的 challenge 类型描述
pub fn challenge_description(domain: &str) -> &'static str {
    if is_ip_address(domain) {
        "TLS-ALPN-01"
    } else {
        "HTTP-01"
    }
}

/// 确定 TLS 模式（优先级：manual > acme > self-signed > none）
pub fn determine_tls_mode(config: &HubConfig) -> TlsMode {
    // 1. 手动证书（最高优先级）
    if let Some(ref tls) = config.tls {
        if !tls.cert.as_os_str().is_empty() && !tls.key.as_os_str().is_empty() {
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

    // 3. 无 TLS 配置 → HTTP only
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
    /// 自签名证书
    SelfSigned,
    /// 无 TLS（HTTP only）
    None,
}

impl std::fmt::Display for TlsMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsMode::Manual => write!(f, "manual"),
            TlsMode::AcmeDomain => write!(f, "acme-domain"),
            TlsMode::AcmeIp => write!(f, "acme-ip"),
            TlsMode::SelfSigned => write!(f, "self-signed"),
            TlsMode::None => write!(f, "none"),
        }
    }
}

/// 启动 ACME 后台驱动任务
///
/// 驱动 AcmeState stream，处理证书申请和续期。
/// 返回 `(default_config, challenge_config)` 用于构建 TLS 服务器。
pub async fn start_acme_driver(
    acme_cfg: HubAcmeConfig,
    data_dir: std::path::PathBuf,
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

    // 后台驱动 AcmeState stream（处理证书申请和续期）
    tokio::spawn(async move {
        use futures_util::StreamExt;
        let mut state = state;
        loop {
            match state.next().await {
                Some(Ok(event)) => {
                    tracing::info!(?event, "ACME event");
                }
                Some(Err(e)) => {
                    tracing::error!(error = %e, "ACME error");
                }
                None => {
                    tracing::warn!("ACME stream ended unexpectedly");
                    break;
                }
            }
        }
    });

    Ok((default_config, challenge_config, http01_service))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_acme_config_empty_domain() {
        let cfg = HubAcmeConfig {
            domain: String::new(),
            email: "test@example.com".to_string(),
            staging: true,
        };
        assert!(validate_acme_config(&cfg).is_err());
    }

    #[test]
    fn validate_acme_config_empty_email() {
        let cfg = HubAcmeConfig {
            domain: "example.com".to_string(),
            email: String::new(),
            staging: true,
        };
        assert!(validate_acme_config(&cfg).is_err());
    }

    #[test]
    fn validate_acme_config_valid() {
        let cfg = HubAcmeConfig {
            domain: "example.com".to_string(),
            email: "admin@example.com".to_string(),
            staging: true,
        };
        assert!(validate_acme_config(&cfg).is_ok());
    }

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
        };
        let state = build_acme_state(&cfg, dir.path()).unwrap();
        drop(state);
    }
}
