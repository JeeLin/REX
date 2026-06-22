use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TlsConfig {
    pub cert: PathBuf,
    pub key: PathBuf,
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
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:3000".to_string(),
            data_dir: PathBuf::from("./data"),
            secret_key: String::new(),
            static_dir: None,
            tls: None,
        }
    }
}

impl HubConfig {
    pub fn load(
        config_path: Option<&str>,
        cli_tls_cert: Option<&str>,
        cli_tls_key: Option<&str>,
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

    #[test]
    fn default_config() {
        let config = HubConfig::default();
        assert_eq!(config.listen, "0.0.0.0:3000");
        assert_eq!(config.data_dir, PathBuf::from("./data"));
        assert!(config.secret_key.is_empty());
        assert!(config.tls.is_none());
    }

    #[test]
    fn load_missing_file_uses_defaults() {
        let config = HubConfig::load(Some("nonexistent.yaml"), None, None).unwrap();
        assert_eq!(config.listen, "0.0.0.0:3000");
        assert!(config.tls.is_none());
    }

    #[test]
    fn load_with_env_override() {
        std::env::set_var("REX_LISTEN", "127.0.0.1:8080");
        let config = HubConfig::load(Some("nonexistent.yaml"), None, None).unwrap();
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

        let config = HubConfig::load(Some(path.to_str().unwrap()), None, None).unwrap();
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

        let config = HubConfig::load(Some(path.to_str().unwrap()), None, None).unwrap();
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

        let config = HubConfig::load(Some(path.to_str().unwrap()), None, None).unwrap();
        assert!(config.tls.is_none());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_tls_env_override() {
        std::env::set_var("REX_TLS_CERT", "/env/cert.pem");
        std::env::set_var("REX_TLS_KEY", "/env/key.pem");
        let config = HubConfig::load(Some("nonexistent.yaml"), None, None).unwrap();
        let tls = config.tls.unwrap();
        assert_eq!(tls.cert, PathBuf::from("/env/cert.pem"));
        assert_eq!(tls.key, PathBuf::from("/env/key.pem"));
        std::env::remove_var("REX_TLS_CERT");
        std::env::remove_var("REX_TLS_KEY");
    }
}
