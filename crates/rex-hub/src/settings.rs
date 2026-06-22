use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::acme::TlsMode;
use crate::config::{is_ip_address, HubConfig};
use crate::helpers::ErrorResponse;
use crate::routes::AppState;

/// TLS 状态响应
#[derive(Serialize)]
pub struct TlsStatus {
    /// TLS 模式
    pub mode: String,
    /// 域名或 IP（ACME 模式）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// 证书是否已就绪
    pub cert_ready: bool,
    /// 证书到期时间（ACME/自签名）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_expires_at: Option<String>,
    /// 证书颁发者
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_issuer: Option<String>,
    /// 是否需要 80 端口（HTTP-01）
    pub port_80_required: bool,
}

/// GET /api/settings/tls
pub async fn get_tls_status(
    State(_state): State<std::sync::Arc<AppState>>,
    axum::extract::Extension(config): axum::extract::Extension<HubConfig>,
) -> Result<Json<TlsStatus>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let mode = determine_tls_mode(&config);

    let (mode_str, domain, port_80_required) = match &mode {
        TlsMode::Manual => ("manual".to_string(), None, false),
        TlsMode::AcmeDomain => {
            let domain = config.acme.as_ref().map(|a| a.domain.clone());
            ("acme-domain".to_string(), domain, true)
        }
        TlsMode::AcmeIp => {
            let domain = config.acme.as_ref().map(|a| a.domain.clone());
            ("acme-ip".to_string(), domain, false)
        }
        TlsMode::SelfSigned => ("self-signed".to_string(), None, false),
        TlsMode::None => ("none".to_string(), None, false),
    };

    // 检查证书是否存在
    let cert_ready = match &mode {
        TlsMode::Manual => config
            .tls
            .as_ref()
            .map(|tls| tls.cert.exists() && tls.key.exists())
            .unwrap_or(false),
        TlsMode::AcmeDomain | TlsMode::AcmeIp => {
            config.data_dir.join("acme").join("cached_cert_0").exists()
                || config.data_dir.join("acme").join("cached_cert_1").exists()
        }
        TlsMode::SelfSigned => {
            let dir = config.data_dir.join("self-signed");
            dir.join("cert.pem").exists() && dir.join("cert_key.pem").exists()
        }
        TlsMode::None => false,
    };

    let cert_expires_at = None; // TODO: 解析证书过期时间
    let cert_issuer = match &mode {
        TlsMode::Manual => Some("Manual".to_string()),
        TlsMode::AcmeDomain | TlsMode::AcmeIp => Some("Let's Encrypt".to_string()),
        TlsMode::SelfSigned => Some("Self-Signed".to_string()),
        TlsMode::None => None,
    };

    Ok(Json(TlsStatus {
        mode: mode_str,
        domain,
        cert_ready,
        cert_expires_at,
        cert_issuer,
        port_80_required,
    }))
}

fn determine_tls_mode(config: &HubConfig) -> TlsMode {
    if let Some(ref tls) = config.tls {
        if !tls.cert.as_os_str().is_empty() && !tls.key.as_os_str().is_empty() {
            return TlsMode::Manual;
        }
    }
    if let Some(ref acme) = config.acme {
        if !acme.domain.is_empty() && !acme.email.is_empty() {
            if is_ip_address(&acme.domain) {
                return TlsMode::AcmeIp;
            } else {
                return TlsMode::AcmeDomain;
            }
        }
    }
    TlsMode::SelfSigned
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn tls_status_manual_mode() {
        let config = HubConfig {
            tls: Some(crate::config::TlsConfig {
                cert: PathBuf::from("/path/cert.pem"),
                key: PathBuf::from("/path/key.pem"),
            }),
            ..Default::default()
        };
        assert_eq!(determine_tls_mode(&config), TlsMode::Manual);
    }

    #[test]
    fn tls_status_self_signed_mode() {
        let config = HubConfig::default();
        assert_eq!(determine_tls_mode(&config), TlsMode::SelfSigned);
    }
}
