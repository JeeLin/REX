use anyhow::{Context, Result};
use serde::Deserialize;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

/// 检测字符串是否为 IP 地址（IPv4 或 IPv6）
pub fn is_ip_address(s: &str) -> bool {
    s.parse::<IpAddr>().is_ok()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TlsConfig {
    pub cert: PathBuf,
    pub key: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AcmeConfig {
    /// 域名或公网 IP（如 hub.example.com 或 203.0.113.1）
    pub domain: String,
    /// Let's Encrypt 账户邮箱
    pub email: String,
    /// 是否使用 staging 环境（测试时设为 true，避免触发 rate limit）
    #[serde(default)]
    pub staging: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HubConfig {
    pub listen: String,
    pub data_dir: PathBuf,
    pub secret_key: String,
    #[serde(default)]
    pub static_dir: Option<PathBuf>,
    #[serde(default)]
    pub tls: Option<TlsConfig>,
    #[serde(default)]
    pub acme: Option<AcmeConfig>,
    /// 是否启用自签名证书（默认关闭，需显式开启）
    #[serde(default)]
    pub enable_self_signed: bool,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:3000".to_string(),
            data_dir: PathBuf::from("./data"),
            secret_key: String::new(),
            static_dir: None,
            tls: None,
            acme: None,
            enable_self_signed: false,
        }
    }
}

impl HubConfig {
    pub fn load(
        config_path: Option<&str>,
        cli_tls_cert: Option<&str>,
        cli_tls_key: Option<&str>,
        cli_acme_domain: Option<&str>,
        cli_acme_email: Option<&str>,
        cli_acme_staging: bool,
    ) -> Result<Self> {
        let path = config_path.unwrap_or("hub.yaml");
        let mut config = if Path::new(path).exists() {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read config file: {path}"))?;
            serde_yaml::from_str(&content)
                .with_context(|| format!("failed to parse config file: {path}"))?
        } else {
            Self::default()
        };

        // 环境变量覆盖
        if let Ok(val) = std::env::var("REX_LISTEN") {
            config.listen = val;
        }
        if let Ok(val) = std::env::var("REX_DATA_DIR") {
            config.data_dir = PathBuf::from(val);
        }
        if let Ok(val) = std::env::var("REX_SECRET_KEY") {
            config.secret_key = val;
        }
        if let Ok(val) = std::env::var("REX_STATIC_DIR") {
            config.static_dir = Some(PathBuf::from(val));
        }
        // TLS 环境变量覆盖
        if let Ok(val) = std::env::var("REX_TLS_CERT") {
            config.tls.get_or_insert_with(TlsConfig::default).cert = PathBuf::from(val);
        }
        if let Ok(val) = std::env::var("REX_TLS_KEY") {
            config.tls.get_or_insert_with(TlsConfig::default).key = PathBuf::from(val);
        }

        // CLI 参数覆盖（优先级最高）
        if let Some(cert) = cli_tls_cert {
            config.tls.get_or_insert_with(TlsConfig::default).cert = PathBuf::from(cert);
        }
        if let Some(key) = cli_tls_key {
            config.tls.get_or_insert_with(TlsConfig::default).key = PathBuf::from(key);
        }

        // 如果只设置了证书或私钥之一，清除 TLS 配置（必须成对）
        if let Some(ref tls) = config.tls {
            if tls.cert.as_os_str().is_empty() || tls.key.as_os_str().is_empty() {
                config.tls = None;
            }
        }

        // ACME 环境变量覆盖
        if let Ok(val) = std::env::var("REX_ACME_DOMAIN") {
            config.acme.get_or_insert_with(AcmeConfig::default).domain = val;
        }
        if let Ok(val) = std::env::var("REX_ACME_EMAIL") {
            config.acme.get_or_insert_with(AcmeConfig::default).email = val;
        }
        if let Ok(val) = std::env::var("REX_ACME_STAGING") {
            if let Some(acme) = &mut config.acme {
                acme.staging = val == "true" || val == "1";
            }
        }

        // CLI 参数覆盖（ACME）
        if let Some(domain) = cli_acme_domain {
            config.acme.get_or_insert_with(AcmeConfig::default).domain = domain.to_string();
        }
        if let Some(email) = cli_acme_email {
            config.acme.get_or_insert_with(AcmeConfig::default).email = email.to_string();
        }
        if cli_acme_staging {
            if let Some(acme) = &mut config.acme {
                acme.staging = true;
            }
        }

        // ACME 配置不完整时清除（domain 和 email 必须同时存在）
        if let Some(ref acme) = config.acme {
            if acme.domain.is_empty() || acme.email.is_empty() {
                config.acme = None;
            }
        }

        // 自签名证书开关（默认关闭，需显式开启）
        if let Ok(val) = std::env::var("REX_ENABLE_SELF_SIGNED") {
            config.enable_self_signed = val == "true" || val == "1";
        }

        // 兼容：如果 static_dir 未设置，尝试常见的默认路径
        if config.static_dir.is_none() {
            let candidates = [
                PathBuf::from("/app/static"),
                PathBuf::from("packages/rex-console-web/dist"),
            ];
            for candidate in &candidates {
                if candidate.is_dir() {
                    config.static_dir = Some(candidate.clone());
                    break;
                }
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_default(path: Option<&str>) -> Result<HubConfig> {
        HubConfig::load(path, None, None, None, None, false)
    }

    #[test]
    fn default_config() {
        let config = HubConfig::default();
        assert_eq!(config.listen, "0.0.0.0:3000");
        assert_eq!(config.data_dir, PathBuf::from("./data"));
        assert!(config.secret_key.is_empty());
        assert!(config.tls.is_none());
        assert!(config.acme.is_none());
    }

    #[test]
    fn load_missing_file_uses_defaults() {
        let config = load_default(Some("nonexistent.yaml")).unwrap();
        assert_eq!(config.listen, "0.0.0.0:3000");
        assert!(config.tls.is_none());
        assert!(config.acme.is_none());
    }

    #[test]
    fn load_with_env_override() {
        std::env::set_var("REX_LISTEN", "127.0.0.1:8080");
        let config = load_default(Some("nonexistent.yaml")).unwrap();
        assert_eq!(config.listen, "127.0.0.1:8080");
        std::env::remove_var("REX_LISTEN");
    }

    #[test]
    fn load_from_real_file() {
        let dir = std::env::temp_dir().join("rex_test_config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hub.yaml");
        std::fs::write(
            &path,
            "listen: \":4000\"\ndata_dir: \"/tmp/rex\"\nsecret_key: \"test-key\"\n",
        )
        .unwrap();

        let config = load_default(Some(path.to_str().unwrap())).unwrap();
        assert_eq!(config.listen, ":4000");
        assert_eq!(config.data_dir, PathBuf::from("/tmp/rex"));
        assert_eq!(config.secret_key, "test-key");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_tls_from_config_file() {
        let dir = std::env::temp_dir().join("rex_test_tls_config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hub.yaml");
        std::fs::write(
            &path,
            "listen: \":3000\"\ndata_dir: \"/tmp/rex\"\nsecret_key: \"test\"\ntls:\n  cert: /path/to/cert.pem\n  key: /path/to/key.pem\n",
        )
        .unwrap();

        let config = load_default(Some(path.to_str().unwrap())).unwrap();
        let tls = config.tls.unwrap();
        assert_eq!(tls.cert, PathBuf::from("/path/to/cert.pem"));
        assert_eq!(tls.key, PathBuf::from("/path/to/key.pem"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_tls_cli_override() {
        let dir = std::env::temp_dir().join("rex_test_tls_cli");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hub.yaml");
        std::fs::write(
            &path,
            "listen: \":3000\"\ndata_dir: \"/tmp/rex\"\nsecret_key: \"test\"\ntls:\n  cert: /old/cert.pem\n  key: /old/key.pem\n",
        )
        .unwrap();

        let config = HubConfig::load(
            Some(path.to_str().unwrap()),
            Some("/new/cert.pem"),
            Some("/new/key.pem"),
            None,
            None,
            false,
        )
        .unwrap();
        let tls = config.tls.unwrap();
        assert_eq!(tls.cert, PathBuf::from("/new/cert.pem"));
        assert_eq!(tls.key, PathBuf::from("/new/key.pem"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_tls_incomplete_pair_cleared() {
        let dir = std::env::temp_dir().join("rex_test_tls_incomplete");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hub.yaml");
        std::fs::write(
            &path,
            "listen: \":3000\"\ndata_dir: \"/tmp/rex\"\nsecret_key: \"test\"\ntls:\n  cert: /path/to/cert.pem\n  key: \"\"\n",
        )
        .unwrap();

        let config = load_default(Some(path.to_str().unwrap())).unwrap();
        assert!(config.tls.is_none());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_tls_env_override() {
        std::env::set_var("REX_TLS_CERT", "/env/cert.pem");
        std::env::set_var("REX_TLS_KEY", "/env/key.pem");
        let config = load_default(Some("nonexistent.yaml")).unwrap();
        let tls = config.tls.unwrap();
        assert_eq!(tls.cert, PathBuf::from("/env/cert.pem"));
        assert_eq!(tls.key, PathBuf::from("/env/key.pem"));
        std::env::remove_var("REX_TLS_CERT");
        std::env::remove_var("REX_TLS_KEY");
    }

    #[test]
    fn load_acme_from_config_file() {
        let dir = std::env::temp_dir().join("rex_test_acme_config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hub.yaml");
        std::fs::write(
            &path,
            "listen: \":3000\"\ndata_dir: \"/tmp/rex\"\nsecret_key: \"test\"\nacme:\n  domain: hub.example.com\n  email: admin@example.com\n  staging: true\n",
        )
        .unwrap();

        let config = load_default(Some(path.to_str().unwrap())).unwrap();
        let acme = config.acme.unwrap();
        assert_eq!(acme.domain, "hub.example.com");
        assert_eq!(acme.email, "admin@example.com");
        assert!(acme.staging);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_acme_env_override() {
        std::env::set_var("REX_ACME_DOMAIN", "192.168.1.100");
        std::env::set_var("REX_ACME_EMAIL", "test@local.dev");
        let config = load_default(Some("nonexistent.yaml")).unwrap();
        let acme = config.acme.unwrap();
        assert_eq!(acme.domain, "192.168.1.100");
        assert_eq!(acme.email, "test@local.dev");
        std::env::remove_var("REX_ACME_DOMAIN");
        std::env::remove_var("REX_ACME_EMAIL");
    }

    #[test]
    fn load_acme_cli_override() {
        let config = HubConfig::load(
            Some("nonexistent.yaml"),
            None,
            None,
            Some("cli.example.com"),
            Some("cli@example.com"),
            true,
        )
        .unwrap();
        let acme = config.acme.unwrap();
        assert_eq!(acme.domain, "cli.example.com");
        assert_eq!(acme.email, "cli@example.com");
        assert!(acme.staging);
    }

    #[test]
    fn load_acme_incomplete_cleared() {
        std::env::set_var("REX_ACME_DOMAIN", "example.com");
        // email 缺失 → 应被清除
        let config = load_default(Some("nonexistent.yaml")).unwrap();
        assert!(config.acme.is_none());
        std::env::remove_var("REX_ACME_DOMAIN");
    }

    #[test]
    fn is_ip_address_detection() {
        assert!(is_ip_address("192.168.1.100"));
        assert!(is_ip_address("203.0.113.1"));
        assert!(is_ip_address("::1"));
        assert!(is_ip_address("2001:db8::1"));
        assert!(!is_ip_address("hub.example.com"));
        assert!(!is_ip_address("example.com"));
        assert!(!is_ip_address(""));
    }
}
