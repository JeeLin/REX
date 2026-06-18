use anyhow::{bail, Result};
use async_trait::async_trait;
use rex_common::sql::{ColumnInfo, DatabaseInfo, SqlConnector, SqlResult, TableInfo};
use serde::{Deserialize, Serialize};
use tracing::info;

/// PostgreSQL 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: Option<String>,
}

/// PostgreSQL 连接器
pub struct PostgresConnector {
    config: PostgresConfig,
    connected: bool,
}

impl PostgresConnector {
    pub fn new(config: PostgresConfig) -> Self {
        Self {
            config,
            connected: false,
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let config: PostgresConfig = serde_json::from_str(json)?;
        Ok(Self::new(config))
    }
}

#[async_trait]
impl SqlConnector for PostgresConnector {
    async fn connect(&mut self) -> Result<()> {
        info!(
            host = %self.config.host,
            port = self.config.port,
            user = %self.config.user,
            "connecting to PostgreSQL"
        );
        // TODO: 使用 tokio-postgres 建立连接
        self.connected = true;
        Ok(())
    }

    async fn execute(&self, sql: &str) -> Result<SqlResult> {
        if !self.connected {
            bail!("not connected");
        }
        info!(sql = %sql, "executing PostgreSQL query");
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
        // TODO: SELECT datname FROM pg_database
        Ok(vec![])
    }

    async fn list_tables(&self, database: &str) -> Result<Vec<TableInfo>> {
        if !self.connected {
            bail!("not connected");
        }
        info!(database = %database, "listing PostgreSQL tables");
        // TODO: SELECT tablename FROM pg_tables
        Ok(vec![])
    }

    async fn list_columns(&self, database: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        if !self.connected {
            bail!("not connected");
        }
        info!(database = %database, table = %table, "listing PostgreSQL columns");
        // TODO: SELECT column_name FROM information_schema.columns
        Ok(vec![])
    }

    async fn close(&self) -> Result<()> {
        info!("closing PostgreSQL connection");
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
    fn postgres_config_deserializes() {
        let json = r#"{"host":"127.0.0.1","port":5432,"user":"postgres","password":"pass","database":"test"}"#;
        let config: PostgresConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 5432);
        assert_eq!(config.user, "postgres");
        assert_eq!(config.database, Some("test".into()));
    }

    #[test]
    fn postgres_connector_from_json() {
        let json =
            r#"{"host":"localhost","port":5432,"user":"postgres","password":"","database":null}"#;
        let connector = PostgresConnector::from_json(json).unwrap();
        assert_eq!(connector.config.host, "localhost");
        assert!(!connector.connected);
    }

    #[test]
    fn postgres_connector_is_object_safe() {
        fn _assert_object_safe(_: &dyn SqlConnector) {}
    }

    #[tokio::test]
    async fn postgres_connect_sets_connected() {
        let json =
            r#"{"host":"localhost","port":5432,"user":"postgres","password":"","database":null}"#;
        let mut connector = PostgresConnector::from_json(json).unwrap();
        assert!(!connector.connected);
        connector.connect().await.unwrap();
        assert!(connector.connected);
    }

    #[tokio::test]
    async fn postgres_execute_fails_when_not_connected() {
        let json =
            r#"{"host":"localhost","port":5432,"user":"postgres","password":"","database":null}"#;
        let connector = PostgresConnector::from_json(json).unwrap();
        let result = connector.execute("SELECT 1").await;
        assert!(result.is_err());
    }
}
