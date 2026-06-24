use anyhow::Result;
use async_trait::async_trait;
use rex_common::sql::{ColumnInfo, DatabaseInfo, SqlColumn, SqlConnector, SqlResult, TableInfo};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Column, Row};
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
    pool: Option<PgPool>,
}

impl PostgresConnector {
    pub fn new(config: PostgresConfig) -> Self {
        Self { config, pool: None }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let config: PostgresConfig = serde_json::from_str(json)?;
        Ok(Self::new(config))
    }
}

#[async_trait]
impl SqlConnector for PostgresConnector {
    async fn connect(&mut self) -> Result<()> {
        // 连接到指定数据库，若未指定则连接到 postgres 系统数据库
        let db = self.config.database.as_deref().unwrap_or("postgres");
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            self.config.user, self.config.password, self.config.host, self.config.port, db
        );

        info!(
            host = %self.config.host,
            port = self.config.port,
            user = %self.config.user,
            database = %db,
            "connecting to PostgreSQL"
        );

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;

        self.pool = Some(pool);
        info!("PostgreSQL connection established");
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

        let rows = sqlx::query(
            "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname",
        )
        .fetch_all(pool)
        .await?;

        let mut databases = Vec::new();
        for row in rows {
            let name: String = row.try_get(0)?;
            databases.push(DatabaseInfo { name });
        }

        Ok(databases)
    }

    async fn list_tables(&self, _database: &str) -> Result<Vec<TableInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        let rows = sqlx::query(
            "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename",
        )
        .fetch_all(pool)
        .await?;

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

    async fn list_columns(&self, _database: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        let rows = sqlx::query(
            "SELECT column_name, data_type, is_nullable, \
             (SELECT constraint_type FROM information_schema.table_constraints tc \
              JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
              WHERE tc.table_name = $1 AND kcu.column_name = information_schema.columns.column_name \
              AND tc.constraint_type = 'PRIMARY KEY') IS NOT NULL as is_primary \
             FROM information_schema.columns \
             WHERE table_name = $1 AND table_schema = 'public' \
             ORDER BY ordinal_position"
        )
        .bind(table)
        .fetch_all(pool)
        .await?;

        let mut columns = Vec::new();
        for row in rows {
            let name: String = row.try_get("column_name")?;
            let data_type: String = row.try_get("data_type")?;
            let nullable: String = row.try_get("is_nullable")?;
            let is_pk: bool = row.try_get("is_primary")?;

            columns.push(ColumnInfo {
                name,
                data_type,
                is_nullable: nullable == "YES",
                is_primary_key: is_pk,
            });
        }

        Ok(columns)
    }

    async fn close(&self) -> Result<()> {
        if let Some(pool) = self.pool.as_ref() {
            info!("closing PostgreSQL connection pool");
            pool.close().await;
        }
        Ok(())
    }
}

/// 尝试从行中获取 JSON 值
fn try_get_json_value(row: &sqlx::postgres::PgRow, index: usize) -> serde_json::Value {
    if let Ok(v) = row.try_get::<i32, _>(index) {
        return serde_json::json!(v);
    }
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
    if let Ok(v) = row.try_get::<serde_json::Value, _>(index) {
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
        assert!(connector.pool.is_none());
    }

    #[test]
    fn postgres_connector_is_object_safe() {
        fn _assert_object_safe(_: &dyn SqlConnector) {}
    }

    #[tokio::test]
    async fn postgres_execute_fails_when_not_connected() {
        let json =
            r#"{"host":"localhost","port":5432,"user":"postgres","password":"","database":null}"#;
        let connector = PostgresConnector::from_json(json).unwrap();
        let result = connector.execute("SELECT 1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn postgres_list_databases_fails_when_not_connected() {
        let json =
            r#"{"host":"localhost","port":5432,"user":"postgres","password":"","database":null}"#;
        let connector = PostgresConnector::from_json(json).unwrap();
        let result = connector.list_databases().await;
        assert!(result.is_err());
    }
}
