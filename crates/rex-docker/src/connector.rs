use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

// ── 数据模型 ─────────────────────────────────────────────

/// Docker 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    /// Docker daemon 地址（unix:///var/run/docker.sock 或 tcp://host:port）
    pub host: String,
    pub name: Option<String>,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            host: "unix:///var/run/docker.sock".into(),
            name: None,
        }
    }
}

/// 容器状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContainerState {
    Running,
    Paused,
    Stopped,
    Created,
    Dead,
}

impl std::fmt::Display for ContainerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Stopped => write!(f, "stopped"),
            Self::Created => write!(f, "created"),
            Self::Dead => write!(f, "dead"),
        }
    }
}

/// 端口映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub private: u16,
    pub public: Option<u16>,
    pub protocol: String,
}

/// 容器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: ContainerState,
    pub status: String,
    pub created: String,
    pub ports: Vec<PortMapping>,
}

// ── DockerConnector trait ─────────────────────────────────

#[async_trait]
pub trait DockerConnector: Send + Sync {
    /// 连接到 Docker daemon
    async fn connect(&mut self) -> Result<()>;

    /// 列出容器
    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerInfo>>;

    /// 查看容器详情
    async fn inspect_container(&self, id: &str) -> Result<serde_json::Value>;

    /// 启动容器
    async fn start_container(&self, id: &str) -> Result<()>;

    /// 停止容器
    async fn stop_container(&self, id: &str) -> Result<()>;

    /// 重启容器
    async fn restart_container(&self, id: &str) -> Result<()>;

    /// 删除容器
    async fn remove_container(&self, id: &str) -> Result<()>;

    /// 获取容器日志
    async fn logs(&self, id: &str, tail: u32) -> Result<String>;

    /// 获取 Docker 系统信息
    async fn info(&self) -> Result<HashMap<String, String>>;

    /// 关闭连接
    async fn close(&self) -> Result<()>;
}

// ── DockerConnector stub ──────────────────────────────────

/// Docker 连接器（stub 实现）
///
/// 实际连接通过 Agent 代理或 Hub 直连的 HTTP 隧道完成。
/// 此 stub 用于 trait 定义和配置解析。
pub struct DockerConnectorImpl {
    config: DockerConfig,
    connected: bool,
}

impl DockerConnectorImpl {
    pub fn new(config: DockerConfig) -> Self {
        Self {
            config,
            connected: false,
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let config: DockerConfig = serde_json::from_str(json)?;
        Ok(Self::new(config))
    }

    pub fn config(&self) -> &DockerConfig {
        &self.config
    }

    pub fn into_config(self) -> DockerConfig {
        self.config
    }
}

#[async_trait]
impl DockerConnector for DockerConnectorImpl {
    async fn connect(&mut self) -> Result<()> {
        info!(
            host = %self.config.host,
            "connecting to Docker daemon"
        );
        // TODO: 实际 HTTP 连接到 Docker daemon
        self.connected = true;
        Ok(())
    }

    async fn list_containers(&self, _all: bool) -> Result<Vec<ContainerInfo>> {
        if !self.connected {
            bail!("not connected");
        }
        info!("listing containers");
        // TODO: GET /containers/json
        Ok(vec![])
    }

    async fn inspect_container(&self, id: &str) -> Result<serde_json::Value> {
        if !self.connected {
            bail!("not connected");
        }
        info!(id = %id, "inspecting container");
        // TODO: GET /containers/{id}/json
        Ok(serde_json::json!({}))
    }

    async fn start_container(&self, id: &str) -> Result<()> {
        if !self.connected {
            bail!("not connected");
        }
        info!(id = %id, "starting container");
        // TODO: POST /containers/{id}/start
        Ok(())
    }

    async fn stop_container(&self, id: &str) -> Result<()> {
        if !self.connected {
            bail!("not connected");
        }
        info!(id = %id, "stopping container");
        // TODO: POST /containers/{id}/stop
        Ok(())
    }

    async fn restart_container(&self, id: &str) -> Result<()> {
        if !self.connected {
            bail!("not connected");
        }
        info!(id = %id, "restarting container");
        // TODO: POST /containers/{id}/restart
        Ok(())
    }

    async fn remove_container(&self, id: &str) -> Result<()> {
        if !self.connected {
            bail!("not connected");
        }
        info!(id = %id, "removing container");
        // TODO: DELETE /containers/{id}
        Ok(())
    }

    async fn logs(&self, id: &str, _tail: u32) -> Result<String> {
        if !self.connected {
            bail!("not connected");
        }
        info!(id = %id, "fetching container logs");
        // TODO: GET /containers/{id}/logs
        Ok(String::new())
    }

    async fn info(&self) -> Result<HashMap<String, String>> {
        if !self.connected {
            bail!("not connected");
        }
        // TODO: GET /info
        let mut info = HashMap::new();
        info.insert("ServerVersion".into(), "24.0.0".into());
        info.insert("OSType".into(), "linux".into());
        Ok(info)
    }

    async fn close(&self) -> Result<()> {
        info!("closing Docker connection");
        // TODO: 关闭 HTTP 连接
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_config_default() {
        let config = DockerConfig::default();
        assert_eq!(config.host, "unix:///var/run/docker.sock");
        assert!(config.name.is_none());
    }

    #[test]
    fn docker_config_deserializes() {
        let json = r#"{"host":"tcp://10.0.0.1:2375","name":"prod-server"}"#;
        let config: DockerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.host, "tcp://10.0.0.1:2375");
        assert_eq!(config.name, Some("prod-server".into()));
    }

    #[test]
    fn docker_config_optional_fields() {
        let json = r#"{"host":"unix:///var/run/docker.sock"}"#;
        let config: DockerConfig = serde_json::from_str(json).unwrap();
        assert!(config.name.is_none());
    }

    #[test]
    fn docker_connector_from_json() {
        let json = r#"{"host":"unix:///var/run/docker.sock","name":null}"#;
        let connector = DockerConnectorImpl::from_json(json).unwrap();
        assert_eq!(connector.config().host, "unix:///var/run/docker.sock");
        assert!(!connector.connected);
    }

    #[test]
    fn docker_connector_is_object_safe() {
        fn _assert_object_safe(_: &dyn DockerConnector) {}
    }

    #[tokio::test]
    async fn docker_connect_sets_connected() {
        let json = r#"{"host":"unix:///var/run/docker.sock"}"#;
        let mut connector = DockerConnectorImpl::from_json(json).unwrap();
        assert!(!connector.connected);
        connector.connect().await.unwrap();
        assert!(connector.connected);
    }

    #[tokio::test]
    async fn docker_list_fails_when_not_connected() {
        let json = r#"{"host":"unix:///var/run/docker.sock"}"#;
        let connector = DockerConnectorImpl::from_json(json).unwrap();
        let result = connector.list_containers(false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn docker_list_returns_empty_when_connected() {
        let json = r#"{"host":"unix:///var/run/docker.sock"}"#;
        let mut connector = DockerConnectorImpl::from_json(json).unwrap();
        connector.connect().await.unwrap();
        let containers = connector.list_containers(false).await.unwrap();
        assert!(containers.is_empty());
    }

    #[tokio::test]
    async fn docker_info_returns_server_info() {
        let json = r#"{"host":"unix:///var/run/docker.sock"}"#;
        let mut connector = DockerConnectorImpl::from_json(json).unwrap();
        connector.connect().await.unwrap();
        let info = connector.info().await.unwrap();
        assert_eq!(info.get("ServerVersion").unwrap(), "24.0.0");
        assert_eq!(info.get("OSType").unwrap(), "linux");
    }

    #[tokio::test]
    async fn docker_close_succeeds() {
        let json = r#"{"host":"unix:///var/run/docker.sock"}"#;
        let mut connector = DockerConnectorImpl::from_json(json).unwrap();
        connector.connect().await.unwrap();
        connector.close().await.unwrap();
    }

    #[test]
    fn container_state_display() {
        assert_eq!(ContainerState::Running.to_string(), "running");
        assert_eq!(ContainerState::Paused.to_string(), "paused");
        assert_eq!(ContainerState::Stopped.to_string(), "stopped");
        assert_eq!(ContainerState::Created.to_string(), "created");
        assert_eq!(ContainerState::Dead.to_string(), "dead");
    }

    #[test]
    fn container_state_serializes() {
        let json = serde_json::to_string(&ContainerState::Running).unwrap();
        assert_eq!(json, "\"running\"");
    }

    #[test]
    fn container_info_serializes() {
        let info = ContainerInfo {
            id: "abc123".into(),
            name: "nginx".into(),
            image: "nginx:latest".into(),
            state: ContainerState::Running,
            status: "Up 2 hours".into(),
            created: "2024-01-01T00:00:00Z".into(),
            ports: vec![PortMapping {
                private: 80,
                public: Some(8080),
                protocol: "tcp".into(),
            }],
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("nginx"));
        assert!(json.contains("running"));
        assert!(json.contains("8080"));
    }
}
