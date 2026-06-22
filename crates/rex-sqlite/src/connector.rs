use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

// ── 数据模型 ─────────────────────────────────────────────

/// SQLite 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    /// SQLite 数据库文件路径
    pub db_path: String,
    pub name: Option<String>,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            db_path: String::new(),
            name: None,
        }
    }
}

/// 列信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub cid: i32,
    pub name: String,
    pub r#type: String,
    pub notnull: bool,
    pub default_value: Option<String>,
    pub pk: bool,
}

/// 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub affected_rows: u64,
    pub elapsed_ms: u64,
}

// ── SqliteConnector trait ─────────────────────────────────

#[async_trait]
pub trait SqliteConnector: Send + Sync {
    /// 连接到 SQLite 数据库
    async fn connect(&mut self) -> Result<()>;

    /// 执行 SQL 查询
    async fn execute(&self, sql: &str) -> Result<SqliteResult>;

    /// 列出所有表
    async fn list_tables(&self) -> Result<Vec<String>>;

    /// 获取表结构信息
    async fn get_table_info(&self, table: &str) -> Result<Vec<ColumnInfo>>;

    /// 关闭连接
    async fn close(&self) -> Result<()>;
}

// ── SqliteConnector stub ──────────────────────────────────

/// SQLite 连接器（stub 实现）
///
/// 实际连接通过 Agent 代理或 Hub 直连的 HTTP 隧道完成。
/// 此 stub 用于 trait 定义和配置解析。
pub struct SqliteConnectorImpl {
    config: SqliteConfig,
    connected: bool,
}

impl SqliteConnectorImpl {
    pub fn new(config: SqliteConfig) -> Self {
        Self {
            config,
            connected: false,
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let config: SqliteConfig = serde_json::from_str(json)?;
        Ok(Self::new(config))
    }

    pub fn config(&self) -> &SqliteConfig {
        &self.config
    }

    pub fn into_config(self) -> SqliteConfig {
        self.config
    }
}

#[async_trait]
impl SqliteConnector for SqliteConnectorImpl {
    async fn connect(&mut self) -> Result<()> {
        info!(
            db_path = %self.config.db_path,
            "connecting to SQLite database"
        );
        // TODO: 实际连接 SQLite 数据库文件
        self.connected = true;
        Ok(())
    }

    async fn execute(&self, _sql: &str) -> Result<SqliteResult> {
        if !self.connected {
            bail!("not connected");
        }
        info!("executing SQL query");
        // TODO: 执行 SQL 查询
        Ok(SqliteResult {
            columns: vec![],
            rows: vec![],
            affected_rows: 0,
            elapsed_ms: 0,
        })
    }

    async fn list_tables(&self) -> Result<Vec<String>> {
        if !self.connected {
            bail!("not connected");
        }
        info!("listing tables");
        // TODO: SELECT name FROM sqlite_master WHERE type='table'
        Ok(vec![])
    }

    async fn get_table_info(&self, _table: &str) -> Result<Vec<ColumnInfo>> {
        if !self.connected {
            bail!("not connected");
        }
        info!("getting table info");
        // TODO: PRAGMA table_info(table)
        Ok(vec![])
    }

    async fn close(&self) -> Result<()> {
        info!("closing SQLite connection");
        // TODO: 关闭数据库连接
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_config_default() {
        let config = SqliteConfig::default();
        assert!(config.db_path.is_empty());
        assert!(config.name.is_none());
    }

    #[test]
    fn sqlite_config_deserializes() {
        let json = r#"{"db_path":"/data/app.db","name":"app-db"}"#;
        let config: SqliteConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.db_path, "/data/app.db");
        assert_eq!(config.name, Some("app-db".into()));
    }

    #[test]
    fn sqlite_config_optional_fields() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let config: SqliteConfig = serde_json::from_str(json).unwrap();
        assert!(config.name.is_none());
    }

    #[test]
    fn sqlite_connector_from_json() {
        let json = r#"{"db_path":"/data/app.db","name":null}"#;
        let connector = SqliteConnectorImpl::from_json(json).unwrap();
        assert_eq!(connector.config().db_path, "/data/app.db");
        assert!(!connector.connected);
    }

    #[test]
    fn sqlite_connector_is_object_safe() {
        fn _assert_object_safe(_: &dyn SqliteConnector) {}
    }

    #[tokio::test]
    async fn sqlite_connect_sets_connected() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let mut connector = SqliteConnectorImpl::from_json(json).unwrap();
        assert!(!connector.connected);
        connector.connect().await.unwrap();
        assert!(connector.connected);
    }

    #[tokio::test]
    async fn sqlite_execute_fails_when_not_connected() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let connector = SqliteConnectorImpl::from_json(json).unwrap();
        let result = connector.execute("SELECT 1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn sqlite_list_tables_fails_when_not_connected() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let connector = SqliteConnectorImpl::from_json(json).unwrap();
        let result = connector.list_tables().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn sqlite_get_table_info_fails_when_not_connected() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let connector = SqliteConnectorImpl::from_json(json).unwrap();
        let result = connector.get_table_info("users").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn sqlite_execute_returns_empty_when_connected() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let mut connector = SqliteConnectorImpl::from_json(json).unwrap();
        connector.connect().await.unwrap();
        let result = connector.execute("SELECT 1").await.unwrap();
        assert!(result.columns.is_empty());
        assert!(result.rows.is_empty());
    }

    #[tokio::test]
    async fn sqlite_list_tables_returns_empty_when_connected() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let mut connector = SqliteConnectorImpl::from_json(json).unwrap();
        connector.connect().await.unwrap();
        let tables = connector.list_tables().await.unwrap();
        assert!(tables.is_empty());
    }

    #[tokio::test]
    async fn sqlite_close_succeeds() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let mut connector = SqliteConnectorImpl::from_json(json).unwrap();
        connector.connect().await.unwrap();
        connector.close().await.unwrap();
    }

    #[test]
    fn sqlite_result_serializes() {
        let result = SqliteResult {
            columns: vec!["id".into(), "name".into()],
            rows: vec![
                vec![serde_json::json!(1), serde_json::json!("Alice")],
                vec![serde_json::json!(2), serde_json::json!("Bob")],
            ],
            affected_rows: 0,
            elapsed_ms: 1,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("Alice"));
    }

    #[test]
    fn column_info_serializes() {
        let col = ColumnInfo {
            cid: 0,
            name: "id".into(),
            r#type: "INTEGER".into(),
            notnull: true,
            default_value: None,
            pk: true,
        };
        let json = serde_json::to_string(&col).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("INTEGER"));
    }
}
