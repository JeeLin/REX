//! MySQL 协议实现 — 基于 sqlx 的 SqlConnector。

use anyhow::{Context, Result};
use rex_common::sql::{ColumnInfo, ConnectRequest, QueryResult, SqlConnector, TableInfo};
use sqlx::mysql::{MySqlPool, MySqlRow};
use sqlx::{Column, Row, TypeInfo};

/// MySQL 连接器
pub struct MySqlConnector {
    pool: MySqlPool,
}

impl MySqlConnector {
    /// 建立 MySQL 连接
    pub async fn connect(req: ConnectRequest) -> Result<Self> {
        let database = req.database.as_deref().unwrap_or("");
        let url = format!(
            "mysql://{}:{}@{}:{}/{}",
            req.username,
            req.password.as_deref().unwrap_or(""),
            req.host,
            req.port,
            database
        );

        let pool = MySqlPool::connect(&url)
            .await
            .with_context(|| format!("failed to connect to MySQL at {}:{}", req.host, req.port))?;

        Ok(Self { pool })
    }

    fn row_to_values(row: &MySqlRow) -> Vec<serde_json::Value> {
        let mut values = Vec::new();
        for i in 0..row.columns().len() {
            let val: serde_json::Value = if row.try_get_raw(i).is_ok() {
                if let Ok(v) = row.try_get::<i64, _>(i) {
                    serde_json::json!(v)
                } else if let Ok(v) = row.try_get::<f64, _>(i) {
                    serde_json::json!(v)
                } else if let Ok(v) = row.try_get::<String, _>(i) {
                    serde_json::json!(v)
                } else if let Ok(v) = row.try_get::<bool, _>(i) {
                    serde_json::json!(v)
                } else {
                    serde_json::Value::Null
                }
            } else {
                serde_json::Value::Null
            };
            values.push(val);
        }
        values
    }
}

#[async_trait::async_trait]
impl SqlConnector for MySqlConnector {
    async fn execute(&mut self, sql: &str) -> Result<QueryResult> {
        let start = std::time::Instant::now();
        let trimmed = sql.trim_start().to_uppercase();
        let is_query = trimmed.starts_with("SELECT")
            || trimmed.starts_with("SHOW")
            || trimmed.starts_with("DESCRIBE")
            || trimmed.starts_with("EXPLAIN");

        if is_query {
            let rows = sqlx::query(sql)
                .fetch_all(&self.pool)
                .await
                .with_context(|| format!("failed to execute query: {sql}"))?;

            let columns = if let Some(first) = rows.first() {
                first
                    .columns()
                    .iter()
                    .map(|c| ColumnInfo {
                        name: c.name().to_string(),
                        data_type: c.type_info().name().to_string(),
                        nullable: true,
                        is_primary_key: false,
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let data: Vec<Vec<serde_json::Value>> =
                rows.iter().map(Self::row_to_values).collect();
            let elapsed = start.elapsed().as_millis() as u64;

            Ok(QueryResult {
                columns,
                rows: data,
                affected_rows: 0,
                elapsed_ms: elapsed,
            })
        } else {
            let result = sqlx::query(sql)
                .execute(&self.pool)
                .await
                .with_context(|| format!("failed to execute statement: {sql}"))?;
            let elapsed = start.elapsed().as_millis() as u64;
            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: result.rows_affected(),
                elapsed_ms: elapsed,
            })
        }
    }

    async fn databases(&mut self) -> Result<Vec<String>> {
        let rows = sqlx::query_scalar::<_, String>("SHOW DATABASES")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn tables(&mut self, db: &str) -> Result<Vec<TableInfo>> {
        let sql = format!(
            "SELECT TABLE_NAME AS name, TABLE_TYPE AS table_type \
             FROM information_schema.TABLES \
             WHERE TABLE_SCHEMA = '{db}' \
             ORDER BY TABLE_NAME"
        );
        let mut result_rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(result_rows
            .iter_mut()
            .map(|r| TableInfo {
                name: r.try_get("name").unwrap_or_default(),
                table_type: r.try_get("table_type").unwrap_or_default(),
            })
            .collect())
    }

    async fn columns(&mut self, db: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let sql = format!(
            "SELECT COLUMN_NAME AS name, DATA_TYPE AS data_type, \
             IS_NULLABLE AS nullable, \
             IF(COLUMN_KEY = 'PRI', 1, 0) AS is_primary_key \
             FROM information_schema.COLUMNS \
             WHERE TABLE_SCHEMA = '{db}' AND TABLE_NAME = '{table}' \
             ORDER BY ORDINAL_POSITION"
        );
        let mut result_rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(result_rows
            .iter_mut()
            .map(|r| {
                let nullable: String = r.try_get("nullable").unwrap_or_default();
                let is_pk: i32 = r.try_get("is_primary_key").unwrap_or(0);
                ColumnInfo {
                    name: r.try_get("name").unwrap_or_default(),
                    data_type: r.try_get("data_type").unwrap_or_default(),
                    nullable: nullable == "YES",
                    is_primary_key: is_pk != 0,
                }
            })
            .collect())
    }

    async fn close(&mut self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }
}
