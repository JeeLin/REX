use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct HubConfig {
    pub listen: String,
    pub data_dir: PathBuf,
    pub secret_key: String,
    #[serde(default)]
    pub static_dir: Option<PathBuf>,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:3000".to_string(),
            data_dir: PathBuf::from("./data"),
            secret_key: String::new(),
            static_dir: None,
        }
    }
}

impl HubConfig {
    pub fn load(config_path: Option<&str>) -> Result<Self> {
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
    }

    #[test]
    fn load_missing_file_uses_defaults() {
        let config = HubConfig::load(Some("nonexistent.yaml")).unwrap();
        assert_eq!(config.listen, "0.0.0.0:3000");
    }

    #[test]
    fn load_with_env_override() {
        std::env::set_var("REX_LISTEN", "127.0.0.1:8080");
        let config = HubConfig::load(Some("nonexistent.yaml")).unwrap();
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

        let config = HubConfig::load(Some(path.to_str().unwrap())).unwrap();
        assert_eq!(config.listen, ":4000");
        assert_eq!(config.data_dir, PathBuf::from("/tmp/rex"));
        assert_eq!(config.secret_key, "test-key");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
