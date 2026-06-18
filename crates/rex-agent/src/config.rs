use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    pub server: String,
    pub token: String,
    pub name: String,
    pub data_dir: PathBuf,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server: "http://localhost:3000".to_string(),
            token: String::new(),
            name: "unnamed-agent".to_string(),
            data_dir: PathBuf::from("./data"),
        }
    }
}

impl AgentConfig {
    pub fn load(config_path: Option<&str>) -> Result<Self> {
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
        if let Ok(val) = std::env::var("REX_SERVER") {
            config.server = val;
        }
        if let Ok(val) = std::env::var("REX_TOKEN") {
            config.token = val;
        }
        if let Ok(val) = std::env::var("REX_NAME") {
            config.name = val;
        }
        if let Ok(val) = std::env::var("REX_DATA_DIR") {
            config.data_dir = PathBuf::from(val);
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

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
        let config = AgentConfig::load(Some("/nonexistent/path/agent.yaml")).unwrap();
        assert_eq!(config.server, "http://localhost:3000");
        assert_eq!(config.token, "");
    }

    #[test]
    fn load_with_env_override() {
        env::set_var("REX_SERVER", "http://hub.example.com");
        env::set_var("REX_TOKEN", "test-token-123");
        env::set_var("REX_NAME", "test-agent");
        env::set_var("REX_DATA_DIR", "/tmp/agent-data");

        let config = AgentConfig::load(Some("/nonexistent")).unwrap();
        assert_eq!(config.server, "http://hub.example.com");
        assert_eq!(config.token, "test-token-123");
        assert_eq!(config.name, "test-agent");
        assert_eq!(config.data_dir, PathBuf::from("/tmp/agent-data"));

        env::remove_var("REX_SERVER");
        env::remove_var("REX_TOKEN");
        env::remove_var("REX_NAME");
        env::remove_var("REX_DATA_DIR");
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

        let config = AgentConfig::load(Some(config_path.to_str().unwrap())).unwrap();
        assert_eq!(config.server, "http://custom:8080");
        assert_eq!(config.token, "my-token");
        assert_eq!(config.name, "my-agent");
        assert_eq!(config.data_dir, PathBuf::from("/custom/data"));
    }
}
