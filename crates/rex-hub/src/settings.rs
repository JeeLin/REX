use axum::Json;
use serde::{Deserialize, Serialize};

use crate::acme::{self, SharedAcmeStatus, TlsMode};
use crate::config::HubConfig;
use crate::helpers::{err_resp, ApiResponse, ErrorResponse};

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
    /// ACME 状态（requesting / ready / error），仅 ACME 模式返回
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acme_status: Option<String>,
    /// ACME 错误信息，仅 ACME 模式且出错时返回
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acme_error: Option<String>,
}

/// GET /api/settings/tls
pub async fn get_tls_status(
    axum::extract::Extension(config): axum::extract::Extension<HubConfig>,
    axum::extract::Extension(shared_acme_status): axum::extract::Extension<SharedAcmeStatus>,
) -> Result<Json<TlsStatus>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let mode = acme::determine_tls_mode(&config);

    let (domain, port_80_required) = match &mode {
        TlsMode::AcmeDomain => {
            let domain = config.acme.as_ref().map(|a| a.domain.clone());
            (domain, true)
        }
        TlsMode::AcmeIp => {
            let domain = config.acme.as_ref().map(|a| a.domain.clone());
            (domain, false)
        }
        _ => (None, false),
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
        TlsMode::None => false,
    };

    // 解析证书过期时间
    let cert_path = match &mode {
        TlsMode::Manual => config.tls.as_ref().map(|tls| tls.cert.clone()),
        TlsMode::AcmeDomain | TlsMode::AcmeIp => {
            let acme_dir = config.data_dir.join("acme");
            let p0 = acme_dir.join("cached_cert_0");
            let p1 = acme_dir.join("cached_cert_1");
            if p0.exists() {
                Some(p0)
            } else if p1.exists() {
                Some(p1)
            } else {
                None
            }
        }
        TlsMode::None => None,
    };

    let cert_expires_at = cert_path.and_then(|path| {
        let pem_bytes = std::fs::read(&path).ok()?;
        let mut reader = std::io::BufReader::new(pem_bytes.as_slice());
        let der_cert = rustls_pemfile::certs(&mut reader)
            .filter_map(|r| r.ok())
            .next()?;
        // 解析 X.509 证书 DER 中的 notAfter 字段
        parse_cert_not_after(&der_cert)
    });
    let cert_issuer = match &mode {
        TlsMode::Manual => Some("Manual".to_string()),
        TlsMode::AcmeDomain | TlsMode::AcmeIp => Some("Let's Encrypt".to_string()),
        TlsMode::None => None,
    };

    // ACME 状态（仅 ACME 模式返回）
    let (acme_status, acme_error) = match &mode {
        TlsMode::AcmeDomain | TlsMode::AcmeIp => {
            let status = shared_acme_status.read().await;
            (Some(status.status.clone()), status.error.clone())
        }
        _ => (None, None),
    };

    Ok(Json(TlsStatus {
        mode: mode.to_string(),
        domain,
        cert_ready,
        cert_expires_at,
        cert_issuer,
        port_80_required,
        acme_status,
        acme_error,
    }))
}

// ── User Settings ────────────────────────────────────────────

const USER_SETTINGS_KEY: &str = "user_settings";

/// 后端用户设置结构体，所有字段可选（PATCH 语义）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserSettings {
    // Security
    pub session_timeout: Option<u32>,
    pub audit_enabled: Option<bool>,
    // Terminal
    pub terminal_font_size: Option<u32>,
    pub terminal_font_family: Option<String>,
    pub terminal_cursor_blink: Option<bool>,
    pub terminal_keepalive: Option<u32>,
    // Appearance
    pub theme: Option<String>,
    pub lang: Option<String>,
}

/// 服务端默认值
fn default_user_settings() -> UserSettings {
    UserSettings {
        session_timeout: Some(30),
        audit_enabled: Some(true),
        terminal_font_size: Some(13),
        terminal_font_family: Some("JetBrains Mono".to_string()),
        terminal_cursor_blink: Some(true),
        terminal_keepalive: Some(60),
        theme: Some("dark".to_string()),
        lang: Some("zh".to_string()),
    }
}

/// 从 SQLite 读取 user_settings JSON，反序列化后合并默认值
fn load_user_settings(db: &crate::db::Database) -> UserSettings {
    let defaults = default_user_settings();
    let json_str: Option<String> = db
        .pool
        .get()
        .unwrap()
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![USER_SETTINGS_KEY],
            |row| row.get(0),
        )
        .ok();

    let stored: UserSettings = json_str
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    UserSettings {
        session_timeout: stored.session_timeout.or(defaults.session_timeout),
        audit_enabled: stored.audit_enabled.or(defaults.audit_enabled),
        terminal_font_size: stored.terminal_font_size.or(defaults.terminal_font_size),
        terminal_font_family: stored
            .terminal_font_family
            .or(defaults.terminal_font_family),
        terminal_cursor_blink: stored
            .terminal_cursor_blink
            .or(defaults.terminal_cursor_blink),
        terminal_keepalive: stored.terminal_keepalive.or(defaults.terminal_keepalive),
        theme: stored.theme.or(defaults.theme),
        lang: stored.lang.or(defaults.lang),
    }
}

/// GET /api/user/settings
pub async fn get_user_settings(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::routes::AppState>>,
) -> Result<Json<ApiResponse<UserSettings>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let settings = load_user_settings(&state.db);
    Ok(Json(ApiResponse { data: settings }))
}

/// PUT /api/user/settings — 合并更新（只覆盖请求中提供的字段）
pub async fn update_user_settings(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::routes::AppState>>,
    Json(input): Json<UserSettings>,
) -> Result<Json<ApiResponse<UserSettings>>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let mut current = load_user_settings(&state.db);

    // 只覆盖非 None 字段
    if input.session_timeout.is_some() {
        current.session_timeout = input.session_timeout;
    }
    if input.audit_enabled.is_some() {
        current.audit_enabled = input.audit_enabled;
    }
    if input.terminal_font_size.is_some() {
        current.terminal_font_size = input.terminal_font_size;
    }
    if input.terminal_font_family.is_some() {
        current.terminal_font_family = input.terminal_font_family;
    }
    if input.terminal_cursor_blink.is_some() {
        current.terminal_cursor_blink = input.terminal_cursor_blink;
    }
    if input.terminal_keepalive.is_some() {
        current.terminal_keepalive = input.terminal_keepalive;
    }
    if input.theme.is_some() {
        current.theme = input.theme;
    }
    if input.lang.is_some() {
        current.lang = input.lang;
    }

    let json = serde_json::to_string(&current)
        .map_err(|_| err_resp("INTERNAL_ERROR", "序列化设置失败"))?;

    state
        .db
        .pool
        .get()
        .unwrap()
        .execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![USER_SETTINGS_KEY, json],
        )
        .map_err(|_| err_resp("INTERNAL_ERROR", "保存设置失败"))?;

    Ok(Json(ApiResponse { data: current }))
}

/// 从 DER 编码的 X.509 证书中提取 notAfter 时间。
///
/// 手动搜索 DER 中的 UTCTime (tag 0x17) 或 GeneralizedTime (tag 0x18)，
/// 取最后一个出现的时间值（即 notAfter）。
fn parse_cert_not_after(der: &[u8]) -> Option<String> {
    // 搜索 validity 区域中最后一个 UTCTime 或 GeneralizedTime
    let mut last_time: Option<String> = None;
    let mut i = 0;
    while i < der.len() {
        let tag = der[i];
        i += 1;

        // 解析长度
        if i >= der.len() {
            break;
        }
        let len_byte = der[i];
        i += 1;

        let length = if len_byte & 0x80 == 0 {
            len_byte as usize
        } else {
            let num_bytes = (len_byte & 0x7F) as usize;
            if i + num_bytes > der.len() {
                break;
            }
            let mut len: usize = 0;
            for j in 0..num_bytes {
                len = (len << 8) | der[i + j] as usize;
            }
            i += num_bytes;
            len
        };

        if i + length > der.len() {
            break;
        }

        match tag {
            // UTCTime: 2-digit year
            0x17 if length == 15 => {
                let s = std::str::from_utf8(&der[i..i + length]).ok()?;
                // 格式: YYMMDDHHMMSSZ
                let yr: i32 = s[0..2].parse().ok()?;
                let mon: u32 = s[2..4].parse().ok()?;
                let day: u32 = s[4..6].parse().ok()?;
                let hr: u32 = s[6..8].parse().ok()?;
                let min: u32 = s[8..10].parse().ok()?;
                let sec: u32 = s[10..12].parse().ok()?;
                let year = if yr >= 50 { 1900 + yr } else { 2000 + yr };
                last_time = Some(format!(
                    "{year:04}-{mon:02}-{day:02}T{hr:02}:{min:02}:{sec:02}Z"
                ));
            }
            // GeneralizedTime: 4-digit year
            0x18 if length >= 15 => {
                let s = std::str::from_utf8(&der[i..i + length]).ok()?;
                let yr: i32 = s[0..4].parse().ok()?;
                let mon: u32 = s[4..6].parse().ok()?;
                let day: u32 = s[6..8].parse().ok()?;
                let hr: u32 = s[8..10].parse().ok()?;
                let min: u32 = s[10..12].parse().ok()?;
                let sec: u32 = s[12..14].parse().ok()?;
                last_time = Some(format!(
                    "{yr:04}-{mon:02}-{day:02}T{hr:02}:{min:02}:{sec:02}Z"
                ));
            }
            _ => {}
        }

        i += length;
    }

    last_time
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tls_status_manual_mode() {
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        std::fs::write(&cert, "cert").unwrap();
        std::fs::write(&key, "key").unwrap();
        let config = HubConfig {
            tls: Some(crate::config::TlsConfig { cert, key }),
            ..Default::default()
        };
        assert_eq!(acme::determine_tls_mode(&config), TlsMode::Manual);
    }

    #[test]
    fn tls_status_none_mode() {
        let config = HubConfig::default();
        assert_eq!(acme::determine_tls_mode(&config), TlsMode::None);
    }

    #[test]
    fn tls_status_struct_serializes() {
        let status = TlsStatus {
            mode: "none".to_string(),
            domain: None,
            cert_ready: false,
            cert_expires_at: None,
            cert_issuer: None,
            port_80_required: false,
            acme_status: None,
            acme_error: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("none"));
        assert!(!json.contains("domain"));
    }

    #[test]
    fn tls_status_acme_domain_mode() {
        let config = HubConfig {
            acme: Some(crate::config::AcmeConfig {
                domain: "hub.example.com".to_string(),
                email: "admin@example.com".to_string(),
                staging: false,
                http_port: 80,
            }),
            ..Default::default()
        };
        assert_eq!(acme::determine_tls_mode(&config), TlsMode::AcmeDomain);
    }

    #[test]
    fn tls_status_acme_ip_mode() {
        let config = HubConfig {
            acme: Some(crate::config::AcmeConfig {
                domain: "203.0.113.1".to_string(),
                email: "admin@example.com".to_string(),
                staging: false,
                http_port: 80,
            }),
            ..Default::default()
        };
        assert_eq!(acme::determine_tls_mode(&config), TlsMode::AcmeIp);
    }

    #[test]
    fn tls_status_struct_with_domain() {
        let status = TlsStatus {
            mode: "acme_domain".to_string(),
            domain: Some("hub.example.com".to_string()),
            cert_ready: true,
            cert_expires_at: Some("2025-12-31".to_string()),
            cert_issuer: Some("Let's Encrypt".to_string()),
            port_80_required: true,
            acme_status: Some("ready".to_string()),
            acme_error: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("hub.example.com"));
        assert!(json.contains("Let's Encrypt"));
    }

    #[test]
    fn tls_status_acme_error_serializes() {
        let status = TlsStatus {
            mode: "acme_domain".to_string(),
            domain: Some("hub.example.com".to_string()),
            cert_ready: false,
            cert_expires_at: None,
            cert_issuer: Some("Let's Encrypt".to_string()),
            port_80_required: true,
            acme_status: Some("error".to_string()),
            acme_error: Some("rate limit exceeded".to_string()),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("rate limit exceeded"));
    }

    #[test]
    fn tls_status_acme_fields_skipped_when_none() {
        let status = TlsStatus {
            mode: "none".to_string(),
            domain: None,
            cert_ready: false,
            cert_expires_at: None,
            cert_issuer: None,
            port_80_required: false,
            acme_status: None,
            acme_error: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(!json.contains("acme_status"));
        assert!(!json.contains("acme_error"));
    }

    #[tokio::test]
    async fn shared_acme_status_default_state() {
        let shared = acme::new_shared_acme_status();
        let status = shared.read().await;
        assert_eq!(status.status, "requesting");
        assert!(status.error.is_none());
    }

    #[tokio::test]
    async fn shared_acme_status_update() {
        let shared = acme::new_shared_acme_status();
        {
            let mut status = shared.write().await;
            status.status = "ready".to_string();
            status.error = None;
        }
        let status = shared.read().await;
        assert_eq!(status.status, "ready");
        assert!(status.error.is_none());
    }

    #[tokio::test]
    async fn shared_acme_status_error() {
        let shared = acme::new_shared_acme_status();
        {
            let mut status = shared.write().await;
            status.status = "error".to_string();
            status.error = Some("timeout".to_string());
        }
        let status = shared.read().await;
        assert_eq!(status.status, "error");
        assert_eq!(status.error.as_deref(), Some("timeout"));
    }
}
