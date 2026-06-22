use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::resp::RedisValue;

// ── 数据模型 ─────────────────────────────────────────────

/// Redis 连接响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisResponse {
    pub value: RedisValue,
    pub elapsed_ms: u64,
}

/// Redis 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    #[serde(default)]
    pub db: u8,
    pub name: Option<String>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 6379,
            password: None,
            db: 0,
            name: None,
        }
    }
}

// ── RedisConnector trait ─────────────────────────────────

#[async_trait]
pub trait RedisConnector: Send + Sync {
    /// 连接到 Redis 服务器
    async fn connect(&mut self) -> Result<()>;

    /// 执行 Redis 命令
    async fn execute(&self, command: &str) -> Result<RedisResponse>;

    /// 获取服务器信息
    async fn info(&self) -> Result<HashMap<String, String>>;

    /// 关闭连接
    async fn close(&self) -> Result<()>;
}

// ── RedisConnector stub ──────────────────────────────────

/// Redis 连接器（stub 实现）
///
/// 实际连接通过 Agent 代理或 Hub 直连的 TCP 隧道完成。
/// 此 stub 用于 trait 定义和配置解析。
pub struct RedisConnectorImpl {
    config: RedisConfig,
    connected: bool,
}

impl RedisConnectorImpl {
    pub fn new(config: RedisConfig) -> Self {
        Self {
            config,
            connected: false,
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let config: RedisConfig = serde_json::from_str(json)?;
        Ok(Self::new(config))
    }

    pub fn config(&self) -> &RedisConfig {
        &self.config
    }
}

#[async_trait]
impl RedisConnector for RedisConnectorImpl {
    async fn connect(&mut self) -> Result<()> {
        info!(
            host = %self.config.host,
            port = self.config.port,
            db = self.config.db,
            "connecting to Redis"
        );
        // TODO: 实际 TCP 连接 + AUTH + SELECT
        self.connected = true;
        Ok(())
    }

    async fn execute(&self, command: &str) -> Result<RedisResponse> {
        if !self.connected {
            bail!("not connected");
        }
        info!(command = %command, "executing Redis command");
        // TODO: 通过 TCP 发送 RESP 命令并解析响应
        Ok(RedisResponse {
            value: RedisValue::Status("OK".into()),
            elapsed_ms: 0,
        })
    }

    async fn info(&self) -> Result<HashMap<String, String>> {
        if !self.connected {
            bail!("not connected");
        }
        // TODO: 执行 INFO server 并解析
        let mut info = HashMap::new();
        info.insert("redis_version".into(), "7.0.0".into());
        info.insert("mode".into(), "standalone".into());
        Ok(info)
    }

    async fn close(&self) -> Result<()> {
        info!("closing Redis connection");
        // TODO: 关闭 TCP 连接
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redis_config_default() {
        let config = RedisConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 6379);
        assert!(config.password.is_none());
        assert_eq!(config.db, 0);
    }

    #[test]
    fn redis_config_deserializes() {
        let json = r#"{"host":"10.0.0.1","port":6380,"password":"secret","db":2,"name":"cache"}"#;
        let config: RedisConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.host, "10.0.0.1");
        assert_eq!(config.port, 6380);
        assert_eq!(config.password, Some("secret".into()));
        assert_eq!(config.db, 2);
        assert_eq!(config.name, Some("cache".into()));
    }

    #[test]
    fn redis_config_optional_fields() {
        let json = r#"{"host":"localhost","port":6379}"#;
        let config: RedisConfig = serde_json::from_str(json).unwrap();
        assert!(config.password.is_none());
        assert_eq!(config.db, 0);
        assert!(config.name.is_none());
    }

    #[test]
    fn redis_connector_from_json() {
        let json = r#"{"host":"localhost","port":6379,"password":null,"db":0,"name":null}"#;
        let connector = RedisConnectorImpl::from_json(json).unwrap();
        assert_eq!(connector.config().host, "localhost");
        assert!(!connector.connected);
    }

    #[test]
    fn redis_connector_is_object_safe() {
        fn _assert_object_safe(_: &dyn RedisConnector) {}
    }

    #[tokio::test]
    async fn redis_connect_sets_connected() {
        let json = r#"{"host":"localhost","port":6379}"#;
        let mut connector = RedisConnectorImpl::from_json(json).unwrap();
        assert!(!connector.connected);
        connector.connect().await.unwrap();
        assert!(connector.connected);
    }

    #[tokio::test]
    async fn redis_execute_fails_when_not_connected() {
        let json = r#"{"host":"localhost","port":6379}"#;
        let connector = RedisConnectorImpl::from_json(json).unwrap();
        let result = connector.execute("PING").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn redis_info_returns_server_info() {
        let json = r#"{"host":"localhost","port":6379}"#;
        let mut connector = RedisConnectorImpl::from_json(json).unwrap();
        connector.connect().await.unwrap();
        let info = connector.info().await.unwrap();
        assert_eq!(info.get("redis_version").unwrap(), "7.0.0");
        assert_eq!(info.get("mode").unwrap(), "standalone");
    }

    #[tokio::test]
    async fn redis_close_succeeds() {
        let json = r#"{"host":"localhost","port":6379}"#;
        let mut connector = RedisConnectorImpl::from_json(json).unwrap();
        connector.connect().await.unwrap();
        connector.close().await.unwrap();
    }

    #[test]
    fn redis_response_serializes() {
        let resp = RedisResponse {
            value: RedisValue::Bulk(Some("hello".into())),
            elapsed_ms: 2,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Bulk"));
        assert!(json.contains("hello"));
        assert!(json.contains("elapsed_ms"));
    }
}
