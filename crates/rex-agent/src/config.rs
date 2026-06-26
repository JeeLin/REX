use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConfig {
    #[serde(default)]
    pub source: rex_common::updater::UpdateSource,
    #[serde(default = "default_auto_update")]
    pub auto_update: bool,
}

fn default_auto_update() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    pub server: String,
    pub token: String,
    pub name: String,
    pub data_dir: PathBuf,
    #[serde(default)]
    pub update: UpdateConfig,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server: "http://localhost:3000".to_string(),
            token: String::new(),
            name: "unnamed-agent".to_string(),
            data_dir: PathBuf::from("./data"),
            update: UpdateConfig::default(),
        }
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            source: rex_common::updater::UpdateSource::GitHub,
            auto_update: true,
        }
    }
}

impl AgentConfig {
    pub fn load(config_path: Option<&str>) -> Result<Self> {
        Self::load_with_env(config_path, |key| std::env::var(key))
    }

    /// 从指定的环境变量读取器加载配置（测试友好，避免并行污染）
    pub fn load_with_env(
        config_path: Option<&str>,
        env_fn: impl Fn(&str) -> std::result::Result<String, std::env::VarError>,
    ) -> Result<Self> {
        let mut config = match config_path {
            Some(path) => {
                let p = Path::new(path);
                if p.exists() {
                    let content = std::fs::read_to_string(p)
                        .with_context(|| format!("failed to read config: {}", p.display()))?;
                    serde_yaml::from_str(&content)
                        .with_context(|| format!("failed to parse config: {}", p.display()))?
                } else {
                    tracing::warn!("config file not found: {}, using defaults", p.display());
                    Self::default()
                }
            }
            None => {
                let default_path = Path::new("agent.yaml");
                if default_path.exists() {
                    let content = std::fs::read_to_string(default_path)
                        .with_context(|| "failed to read agent.yaml")?;
                    serde_yaml::from_str(&content).with_context(|| "failed to parse agent.yaml")?
                } else {
                    Self::default()
                }
            }
        };

        // 环境变量覆盖
        if let Ok(val) = env_fn("REX_SERVER") {
            config.server = val;
        }
        if let Ok(val) = env_fn("REX_TOKEN") {
            config.token = val;
        }
        if let Ok(val) = env_fn("REX_NAME") {
            config.name = val;
        }
        if let Ok(val) = env_fn("REX_DATA_DIR") {
            config.data_dir = PathBuf::from(val);
        }
        if let Ok(val) = env_fn("REX_UPDATE_SOURCE") {
            config.update.source = match val.to_lowercase().as_str() {
                "hub" => rex_common::updater::UpdateSource::Hub,
                _ => rex_common::updater::UpdateSource::GitHub,
            };
        }
        if let Ok(val) = env_fn("REX_AUTO_UPDATE") {
            config.update.auto_update = matches!(val.to_lowercase().as_str(), "true" | "1" | "yes");
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 创建一个基于 HashMap 的环境变量读取器（线程安全，无进程级副作用）
    fn fake_env(
        vars: &[(&str, &str)],
    ) -> impl Fn(&str) -> std::result::Result<String, std::env::VarError> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key: &str| -> std::result::Result<String, std::env::VarError> {
            map.get(key).cloned().ok_or(std::env::VarError::NotPresent)
        }
    }

    #[test]
    fn default_config() {
        let config = AgentConfig::default();
        assert_eq!(config.server, "http://localhost:3000");
        assert_eq!(config.token, "");
        assert_eq!(config.name, "unnamed-agent");
        assert_eq!(config.data_dir, PathBuf::from("./data"));
    }

    #[test]
    fn load_missing_file_uses_default() {
        let empty = fake_env(&[]);
        let config =
            AgentConfig::load_with_env(Some("/nonexistent/path/agent.yaml"), empty).unwrap();
        assert_eq!(config.server, "http://localhost:3000");
        assert_eq!(config.token, "");
    }

    #[test]
    fn load_with_env_override() {
        let env = fake_env(&[
            ("REX_SERVER", "http://hub.example.com"),
            ("REX_TOKEN", "test-token-123"),
            ("REX_NAME", "test-agent"),
            ("REX_DATA_DIR", "/tmp/agent-data"),
        ]);

        let config = AgentConfig::load_with_env(Some("/nonexistent"), env).unwrap();
        assert_eq!(config.server, "http://hub.example.com");
        assert_eq!(config.token, "test-token-123");
        assert_eq!(config.name, "test-agent");
        assert_eq!(config.data_dir, PathBuf::from("/tmp/agent-data"));
    }

    #[test]
    fn load_yaml_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("agent.yaml");
        std::fs::write(
            &config_path,
            "server: http://custom:8080\ntoken: my-token\nname: my-agent\ndata_dir: /custom/data\n",
        )
        .unwrap();

        let empty = fake_env(&[]);
        let config =
            AgentConfig::load_with_env(Some(config_path.to_str().unwrap()), empty).unwrap();
        assert_eq!(config.server, "http://custom:8080");
        assert_eq!(config.token, "my-token");
        assert_eq!(config.name, "my-agent");
        assert_eq!(config.data_dir, PathBuf::from("/custom/data"));
    }

    #[test]
    fn default_update_source_is_github() {
        let config = AgentConfig::default();
        assert_eq!(
            config.update.source,
            rex_common::updater::UpdateSource::GitHub
        );
    }

    #[test]
    fn load_update_source_from_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("agent.yaml");
        std::fs::write(
            &config_path,
            "server: http://localhost:3000\ntoken: t\nname: a\ndata_dir: /tmp/d\nupdate:\n  source: hub\n",
        )
        .unwrap();

        let empty = fake_env(&[]);
        let config =
            AgentConfig::load_with_env(Some(config_path.to_str().unwrap()), empty).unwrap();
        assert_eq!(config.update.source, rex_common::updater::UpdateSource::Hub);
    }

    #[test]
    fn load_update_source_from_env() {
        let env = fake_env(&[("REX_UPDATE_SOURCE", "hub")]);
        let config = AgentConfig::load_with_env(Some("/nonexistent"), env).unwrap();
        assert_eq!(config.update.source, rex_common::updater::UpdateSource::Hub);
    }

    #[test]
    fn default_auto_update_is_true() {
        let config = AgentConfig::default();
        assert!(config.update.auto_update);
    }

    #[test]
    fn load_auto_update_from_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("agent.yaml");
        std::fs::write(
            &config_path,
            "server: http://localhost:3000\ntoken: t\nname: a\ndata_dir: /tmp/d\nupdate:\n  source: hub\n  auto_update: false\n",
        )
        .unwrap();

        let empty = fake_env(&[]);
        let config =
            AgentConfig::load_with_env(Some(config_path.to_str().unwrap()), empty).unwrap();
        assert!(!config.update.auto_update);
    }

    #[test]
    fn load_auto_update_from_env() {
        let env = fake_env(&[("REX_AUTO_UPDATE", "false")]);
        let config = AgentConfig::load_with_env(Some("/nonexistent"), env).unwrap();
        assert!(!config.update.auto_update);
    }

    #[test]
    fn load_auto_update_from_env_true_variants() {
        for val in &["true", "1", "yes", "True", "YES"] {
            let env = fake_env(&[("REX_AUTO_UPDATE", val)]);
            let config = AgentConfig::load_with_env(Some("/nonexistent"), env).unwrap();
            assert!(config.update.auto_update, "expected true for value: {val}");
        }
    }
}
