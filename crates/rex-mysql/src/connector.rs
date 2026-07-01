use anyhow::Result;
use async_trait::async_trait;
use rex_common::sql::{ColumnInfo, DatabaseInfo, ExplainResult, SqlColumn, SqlConnector, SqlResult, TableInfo};
use serde::{Deserialize, Serialize};
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::Column;
use sqlx::Row;
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
    pool: Option<MySqlPool>,
}

impl MySqlConnector {
    pub fn new(config: MySqlConfig) -> Self {
        Self { config, pool: None }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let config: MySqlConfig = serde_json::from_str(json)?;
        Ok(Self::new(config))
    }
}

#[async_trait]
impl SqlConnector for MySqlConnector {
    async fn connect(&mut self) -> Result<()> {
        let url = if let Some(ref db) = self.config.database {
            format!(
                "mysql://{}:{}@{}:{}/{}",
                self.config.user, self.config.password, self.config.host, self.config.port, db
            )
        } else {
            format!(
                "mysql://{}:{}@{}:{}",
                self.config.user, self.config.password, self.config.host, self.config.port
            )
        };

        info!(
            host = %self.config.host,
            port = self.config.port,
            user = %self.config.user,
            "connecting to MySQL"
        );

        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;

        self.pool = Some(pool);
        info!("MySQL connection established");
        Ok(())
    }

    async fn execute(&self, sql: &str) -> Result<SqlResult> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        let start = std::time::Instant::now();
        let rows = sqlx::query(sql).fetch_all(pool).await?;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        // 获取列信息
        let columns: Vec<SqlColumn> = if let Some(first_row) = rows.first() {
            first_row
                .columns()
                .iter()
                .map(|col| SqlColumn {
                    name: col.name().to_string(),
                    data_type: format!("{:?}", col.type_info().clone()),
                })
                .collect()
        } else {
            vec![]
        };

        // 转换行数据
        let mut result_rows = Vec::new();
        for row in &rows {
            let mut result_row = Vec::new();
            for (i, _col) in columns.iter().enumerate() {
                // 尝试将每列转换为 JSON 值
                let value = try_get_json_value(row, i);
                result_row.push(value);
            }
            result_rows.push(result_row);
        }

        let affected_rows = rows.len() as u64;

        Ok(SqlResult {
            columns,
            rows: result_rows,
            affected_rows,
            elapsed_ms,
        })
    }

    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        let rows = sqlx::query("SHOW DATABASES").fetch_all(pool).await?;

        let mut databases = Vec::new();
        for row in rows {
            let name: String = row.try_get(0)?;
            databases.push(DatabaseInfo { name });
        }

        Ok(databases)
    }

    async fn list_tables(&self, database: &str) -> Result<Vec<TableInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        let query = format!("SHOW TABLES FROM `{}`", database.replace('`', "``"));
        let rows = sqlx::query(&query).fetch_all(pool).await?;

        let mut tables = Vec::new();
        for row in rows {
            let name: String = row.try_get(0)?;
            tables.push(TableInfo {
                name,
                row_count: None,
            });
        }

        Ok(tables)
    }

    async fn list_columns(&self, database: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        let query = format!(
            "SHOW COLUMNS FROM `{}` FROM `{}`",
            table.replace('`', "``"),
            database.replace('`', "``")
        );
        let rows = sqlx::query(&query).fetch_all(pool).await?;

        let mut columns = Vec::new();
        for row in rows {
            let name: String = row.try_get("Field")?;
            let data_type: String = row.try_get("Type")?;
            let nullable: String = row.try_get("Null")?;
            let key: String = row.try_get("Key")?;

            columns.push(ColumnInfo {
                name,
                data_type,
                is_nullable: nullable == "YES",
                is_primary_key: key == "PRI",
            });
        }

        Ok(columns)
    }

    async fn explain(&self, sql: &str) -> Result<ExplainResult> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        let explain_sql = format!("EXPLAIN {sql}");
        let rows = sqlx::query(&explain_sql).fetch_all(pool).await?;

        let mut columns = Vec::new();
        let mut result_rows = Vec::new();

        if let Some(first_row) = rows.first() {
            columns = first_row
                .columns()
                .iter()
                .map(|col| col.name().to_string())
                .collect();
        }

        for row in &rows {
            let mut result_row = Vec::new();
            for (i, _) in columns.iter().enumerate() {
                let value = try_get_json_value(row, i);
                result_row.push(value);
            }
            result_rows.push(result_row);
        }

        // Build raw text output
        let mut raw_lines = Vec::new();
        raw_lines.push(columns.join("\t"));
        for row in &result_rows {
            let line: Vec<String> = row
                .iter()
                .map(|v| match v {
                    serde_json::Value::Null => "NULL".to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect();
            raw_lines.push(line.join("\t"));
        }

        Ok(ExplainResult {
            columns,
            rows: result_rows,
            raw_output: raw_lines.join("\n"),
        })
    }

    async fn close(&self) -> Result<()> {
        if let Some(pool) = self.pool.as_ref() {
            info!("closing MySQL connection pool");
            pool.close().await;
        }
        Ok(())
    }
}

/// 尝试从行中获取 JSON 值
fn try_get_json_value(row: &sqlx::mysql::MySqlRow, index: usize) -> serde_json::Value {
    // 尝试不同的类型
    if let Ok(v) = row.try_get::<i64, _>(index) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<f64, _>(index) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<bool, _>(index) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<sqlx::types::JsonValue, _>(index) {
        return v;
    }
    if let Ok(v) = row.try_get::<Vec<u8>, _>(index) {
        return serde_json::json!(String::from_utf8_lossy(&v).to_string());
    }
    serde_json::Value::Null
}

// ── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(connector.pool.is_none());
    }

    #[test]
    fn mysql_connector_is_object_safe() {
        fn _assert_object_safe(_: &dyn SqlConnector) {}
    }

    #[tokio::test]
    async fn mysql_execute_fails_when_not_connected() {
        let json =
            r#"{"host":"localhost","port":3306,"user":"root","password":"","database":null}"#;
        let connector = MySqlConnector::from_json(json).unwrap();
        let result = connector.execute("SELECT 1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mysql_list_databases_fails_when_not_connected() {
        let json =
            r#"{"host":"localhost","port":3306,"user":"root","password":"","database":null}"#;
        let connector = MySqlConnector::from_json(json).unwrap();
        let result = connector.list_databases().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mysql_list_tables_fails_when_not_connected() {
        let json =
            r#"{"host":"localhost","port":3306,"user":"root","password":"","database":null}"#;
        let connector = MySqlConnector::from_json(json).unwrap();
        let result = connector.list_tables("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mysql_list_columns_fails_when_not_connected() {
        let json =
            r#"{"host":"localhost","port":3306,"user":"root","password":"","database":null}"#;
        let connector = MySqlConnector::from_json(json).unwrap();
        let result = connector.list_columns("test", "users").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mysql_connect_fails_on_bad_host() {
        let json =
            r#"{"host":"192.0.2.1","port":3306,"user":"root","password":"","database":null}"#;
        let mut connector = MySqlConnector::from_json(json).unwrap();
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(3), connector.connect()).await;
        assert!(result.is_err() || result.unwrap().is_err());
    }

    #[tokio::test]
    async fn mysql_explain_fails_when_not_connected() {
        let json =
            r#"{"host":"localhost","port":3306,"user":"root","password":"","database":null}"#;
        let connector = MySqlConnector::from_json(json).unwrap();
        let result = connector.explain("SELECT 1").await;
        assert!(result.is_err());
    }

    #[test]
    fn mysql_config_serializes() {
        let config = MySqlConfig {
            host: "localhost".to_string(),
            port: 3306,
            user: "root".to_string(),
            password: "pass".to_string(),
            database: Some("test".to_string()),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("localhost"));
        assert!(json.contains("3306"));
    }
}
