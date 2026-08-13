//! MySQL 协议实现 — 基于 sqlx 的 SqlConnector。

use anyhow::{Context, Result};
use rex_common::bracket_host;
use rex_common::sql::{
    ColumnInfo, ConnectRequest, DdlResult, ForeignKeyInfo, IndexInfo, QueryResult, SqlConnector,
    TableInfo,
};
use sqlx::mysql::{MySqlPool, MySqlRow};
use sqlx::{Column, Row, TypeInfo};

/// 转义 MySQL 标识符（使用反引号包裹）
fn escape_identifier(s: &str) -> String {
    format!("`{}`", s.replace('`', "``"))
}

/// MySQL 连接器
pub struct MySqlConnector {
    pool: MySqlPool,
}

impl MySqlConnector {
    /// 建立 MySQL 连接
    pub async fn connect(req: ConnectRequest) -> Result<Self> {
        let database = req.database.as_deref().unwrap_or("");
        let host = bracket_host(&req.host);
        let url = format!(
            "mysql://{}:{}@{host}:{}/{}",
            req.username,
            req.password.as_deref().unwrap_or(""),
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

            let data: Vec<Vec<serde_json::Value>> = rows.iter().map(Self::row_to_values).collect();
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
        let escaped_db = escape_identifier(db);
        let sql = format!(
            "SELECT TABLE_NAME AS name, TABLE_TYPE AS table_type \
             FROM information_schema.TABLES \
             WHERE TABLE_SCHEMA = {escaped_db} \
             ORDER BY TABLE_NAME"
        );
        let result_rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(result_rows
            .iter()
            .map(|r| TableInfo {
                name: r.try_get("name").unwrap_or_default(),
                table_type: r.try_get("table_type").unwrap_or_default(),
            })
            .collect())
    }

    async fn columns(&mut self, db: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let escaped_db = escape_identifier(db);
        let escaped_table = escape_identifier(table);
        let sql = format!(
            "SELECT COLUMN_NAME AS name, DATA_TYPE AS data_type, \
             IS_NULLABLE AS nullable, \
             IF(COLUMN_KEY = 'PRI', 1, 0) AS is_primary_key \
             FROM information_schema.COLUMNS \
             WHERE TABLE_SCHEMA = {escaped_db} AND TABLE_NAME = {escaped_table} \
             ORDER BY ORDINAL_POSITION"
        );
        let result_rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(result_rows
            .iter()
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

    async fn indexes(&mut self, db: &str, table: &str) -> Result<Vec<IndexInfo>> {
        let sql = format!(
            "SELECT INDEX_NAME AS name, GROUP_CONCAT(COLUMN_NAME ORDER BY SEQ_IN_INDEX) AS columns, \
             NON_UNIQUE, INDEX_TYPE \
             FROM information_schema.STATISTICS \
             WHERE TABLE_SCHEMA = '{db}' AND TABLE_NAME = '{table}' \
             GROUP BY INDEX_NAME, NON_UNIQUE, INDEX_TYPE \
             ORDER BY INDEX_NAME"
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| {
                let non_unique: i32 = r.try_get("non_unique").unwrap_or(0);
                IndexInfo {
                    name: r.try_get("name").unwrap_or_default(),
                    columns: r
                        .try_get::<String, _>("columns")
                        .unwrap_or_default()
                        .split(',')
                        .map(String::from)
                        .collect(),
                    unique: non_unique == 0,
                    index_type: r.try_get("index_type").unwrap_or_default(),
                }
            })
            .collect())
    }

    async fn foreign_keys(&mut self, db: &str, table: &str) -> Result<Vec<ForeignKeyInfo>> {
        let sql = format!(
            "SELECT CONSTRAINT_NAME AS name, \
             GROUP_CONCAT(COLUMN_NAME ORDER BY ORDINAL_POSITION) AS columns, \
             REFERENCED_TABLE_NAME AS ref_table, \
             GROUP_CONCAT(REFERENCED_COLUMN_NAME ORDER BY ORDINAL_POSITION) AS ref_columns, \
             DELETE_RULE AS on_delete, UPDATE_RULE AS on_update \
             FROM information_schema.KEY_COLUMN_USAGE \
             JOIN information_schema.REFERENTIAL_CONSTRAINTS USING (CONSTRAINT_NAME, CONSTRAINT_SCHEMA) \
             WHERE TABLE_SCHEMA = '{db}' AND TABLE_NAME = '{table}' \
               AND REFERENCED_TABLE_NAME IS NOT NULL \
             GROUP BY CONSTRAINT_NAME, REFERENCED_TABLE_NAME, DELETE_RULE, UPDATE_RULE"
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| ForeignKeyInfo {
                name: r.try_get("name").unwrap_or_default(),
                columns: r
                    .try_get::<String, _>("columns")
                    .unwrap_or_default()
                    .split(',')
                    .map(String::from)
                    .collect(),
                ref_table: r.try_get("ref_table").unwrap_or_default(),
                ref_columns: r
                    .try_get::<String, _>("ref_columns")
                    .unwrap_or_default()
                    .split(',')
                    .map(String::from)
                    .collect(),
                on_delete: r.try_get("on_delete").unwrap_or_default(),
                on_update: r.try_get("on_update").unwrap_or_default(),
            })
            .collect())
    }

    async fn ddl(&mut self, _db: &str, table: &str) -> Result<DdlResult> {
        let rows = sqlx::query_scalar::<_, String>(&format!("SHOW CREATE TABLE `{table}`"))
            .fetch_all(&self.pool)
            .await?;
        let ddl = rows.get(1).cloned().unwrap_or_default();
        Ok(DdlResult { ddl })
    }
}
