use anyhow::{bail, Result};
use async_trait::async_trait;
use rex_common::sql::{ColumnInfo, DatabaseInfo, SqlConnector, SqlResult, TableInfo};
use serde::{Deserialize, Serialize};
use tracing::info;

/// MySQL 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySqlConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: Option<String>,
}

/// MySQL 连接器
pub struct MySqlConnector {
    config: MySqlConfig,
    connected: bool,
}

impl MySqlConnector {
    pub fn new(config: MySqlConfig) -> Self {
        Self {
            config,
            connected: false,
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let config: MySqlConfig = serde_json::from_str(json)?;
        Ok(Self::new(config))
    }
}

#[async_trait]
impl SqlConnector for MySqlConnector {
    async fn connect(&mut self) -> Result<()> {
        info!(
            host = %self.config.host,
            port = self.config.port,
            user = %self.config.user,
            "connecting to MySQL"
        );
        // TODO: 使用 sqlx 或 mysql crate 建立连接
        self.connected = true;
        Ok(())
    }

    async fn execute(&self, sql: &str) -> Result<SqlResult> {
        if !self.connected {
            bail!("not connected");
        }
        info!(sql = %sql, "executing MySQL query");
        // TODO: 实际执行 SQL
        Ok(SqlResult {
            columns: vec![],
            rows: vec![],
            affected_rows: 0,
            elapsed_ms: 0,
        })
    }

    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>> {
        if !self.connected {
            bail!("not connected");
        }
        // TODO: SHOW DATABASES
        Ok(vec![])
    }

    async fn list_tables(&self, database: &str) -> Result<Vec<TableInfo>> {
        if !self.connected {
            bail!("not connected");
        }
        info!(database = %database, "listing MySQL tables");
        // TODO: SHOW TABLES
        Ok(vec![])
    }

    async fn list_columns(&self, database: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        if !self.connected {
            bail!("not connected");
        }
        info!(database = %database, table = %table, "listing MySQL columns");
        // TODO: SHOW COLUMNS
        Ok(vec![])
    }

    async fn close(&self) -> Result<()> {
        info!("closing MySQL connection");
        // TODO: 关闭连接
        Ok(())
    }
}

// ── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rex_common::sql::SqlConnector;

    #[test]
    fn mysql_config_deserializes() {
        let json =
            r#"{"host":"127.0.0.1","port":3306,"user":"root","password":"pass","database":"test"}"#;
        let config: MySqlConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3306);
        assert_eq!(config.user, "root");
        assert_eq!(config.database, Some("test".into()));
    }

    #[test]
    fn mysql_connector_from_json() {
        let json =
            r#"{"host":"localhost","port":3306,"user":"root","password":"","database":null}"#;
        let connector = MySqlConnector::from_json(json).unwrap();
        assert_eq!(connector.config.host, "localhost");
        assert!(!connector.connected);
    }

    #[test]
    fn mysql_connector_is_object_safe() {
        fn _assert_object_safe(_: &dyn SqlConnector) {}
    }

    #[tokio::test]
    async fn mysql_connect_sets_connected() {
        let json =
            r#"{"host":"localhost","port":3306,"user":"root","password":"","database":null}"#;
        let mut connector = MySqlConnector::from_json(json).unwrap();
        assert!(!connector.connected);
        connector.connect().await.unwrap();
        assert!(connector.connected);
    }

    #[tokio::test]
    async fn mysql_execute_fails_when_not_connected() {
        let json =
            r#"{"host":"localhost","port":3306,"user":"root","password":"","database":null}"#;
        let connector = MySqlConnector::from_json(json).unwrap();
        let result = connector.execute("SELECT 1").await;
        assert!(result.is_err());
    }
}
