//! TLS 配置 — 支持自签名、手动证书、ACME 三种模式。
//!
//! TODO: hyper 1.x TLS serve 实现需要更深入的 API 研究。
//! 当前所有 TLS 模式回退到 HTTP，环境变量配置已就绪。

use std::path::PathBuf;

/// TLS 配置
pub enum TlsConfig {
    None,
    SelfSigned,
    Manual {
        cert_path: PathBuf,
        key_path: PathBuf,
    },
    Acme {
        domain: String,
        email: String,
        staging: bool,
    },
}

impl TlsConfig {
    pub fn from_env() -> Self {
        if let (Ok(cert), Ok(key)) = (
            std::env::var("REX_TLS_CERT"),
            std::env::var("REX_TLS_KEY"),
        ) {
            return Self::Manual {
                cert_path: PathBuf::from(cert),
                key_path: PathBuf::from(key),
            };
        }
        if let Ok(domain) = std::env::var("REX_ACME_DOMAIN") {
            let email = std::env::var("REX_ACME_EMAIL").unwrap_or_default();
            let staging = std::env::var("REX_ACME_STAGING")
                .map(|v| v == "true")
                .unwrap_or(false);
            return Self::Acme { domain, email, staging };
        }
        if std::env::var("REX_TLS_SELF_SIGNED")
            .map(|v| v == "true")
            .unwrap_or(false)
        {
            return Self::SelfSigned;
        }
        Self::None
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::None)
    }
}

/// 启动监听 — 当前所有模式回退到 HTTP
///
/// TODO: 实现真正的 TLS serve（需要研究 hyper 1.x + tokio-rustls 集成）
pub async fn serve(
    app: axum::Router,
    listener: tokio::net::TcpListener,
    config: TlsConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    match config {
        TlsConfig::None => {
            tracing::info!("listening on HTTP");
        }
        TlsConfig::SelfSigned => {
            tracing::warn!("TLS self-signed mode: falling back to HTTP (TLS not yet implemented)");
        }
        TlsConfig::Manual { cert_path, .. } => {
            tracing::warn!(cert = %cert_path.display(), "TLS manual mode: falling back to HTTP (TLS not yet implemented)");
        }
        TlsConfig::Acme { domain, .. } => {
            tracing::warn!(domain = %domain, "TLS ACME mode: falling back to HTTP (TLS not yet implemented)");
        }
    }
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::from_env();
        assert!(!config.is_enabled());
    }
}
