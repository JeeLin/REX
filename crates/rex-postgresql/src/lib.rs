//! PostgreSQL 协议实现 — 基于 sqlx 的 SqlConnector。

use anyhow::{Context, Result};
use rex_common::sql::{
    ColumnInfo, ConnectRequest, DdlResult, ForeignKeyInfo, IndexInfo, QueryResult, SqlConnector,
    TableInfo,
};
use sqlx::postgres::{PgPool, PgRow};
use sqlx::{Column, Row, TypeInfo};

/// 转义 PostgreSQL 标识符（使用双引号包裹）
fn escape_identifier(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// PostgreSQL 连接器
pub struct PostgresConnector {
    pool: PgPool,
}

impl PostgresConnector {
    /// 建立 PostgreSQL 连接
    pub async fn connect(req: ConnectRequest) -> Result<Self> {
        let database = req.database.as_deref().unwrap_or("postgres");
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            req.username,
            req.password.as_deref().unwrap_or(""),
            req.host,
            req.port,
            database
        );

        let pool = PgPool::connect(&url).await.with_context(|| {
            format!(
                "failed to connect to PostgreSQL at {}:{}",
                req.host, req.port
            )
        })?;

        Ok(Self { pool })
    }

    fn row_to_values(row: &PgRow) -> Vec<serde_json::Value> {
        let mut values = Vec::new();
        for i in 0..row.columns().len() {
            let val: serde_json::Value = if let Ok(v) = row.try_get::<i64, _>(i) {
                serde_json::json!(v)
            } else if let Ok(v) = row.try_get::<f64, _>(i) {
                serde_json::json!(v)
            } else if let Ok(v) = row.try_get::<String, _>(i) {
                serde_json::json!(v)
            } else if let Ok(v) = row.try_get::<bool, _>(i) {
                serde_json::json!(v)
            } else {
                serde_json::Value::Null
            };
            values.push(val);
        }
        values
    }
}

#[async_trait::async_trait]
impl SqlConnector for PostgresConnector {
    async fn execute(&mut self, sql: &str) -> Result<QueryResult> {
        let start = std::time::Instant::now();
        let trimmed = sql.trim_start().to_uppercase();
        let is_query = trimmed.starts_with("SELECT")
            || trimmed.starts_with("SHOW")
            || trimmed.starts_with("WITH")
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
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT datname FROM pg_database WHERE NOT datistemplate ORDER BY datname",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn tables(&mut self, db: &str) -> Result<Vec<TableInfo>> {
        let escaped_db = escape_identifier(db);
        let sql = format!(
            "SELECT c.relname AS name, \
             CASE WHEN c.relkind = 'v' THEN 'VIEW' ELSE 'BASE TABLE' END AS table_type \
             FROM pg_class c \
             JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE n.nspname = {escaped_db} \
             AND c.relkind IN ('r', 'v') \
             ORDER BY c.relname"
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
            "SELECT c.column_name AS name, c.data_type AS data_type, \
             c.is_nullable AS nullable, \
             CASE WHEN pk.column_name IS NOT NULL THEN 1 ELSE 0 END AS is_primary_key \
             FROM information_schema.columns c \
             LEFT JOIN ( \
                 SELECT ku.column_name \
                 FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage ku \
                 ON tc.constraint_name = ku.constraint_name \
                 WHERE tc.table_schema = {escaped_db} \
                 AND tc.table_name = {escaped_table} \
                 AND tc.constraint_type = 'PRIMARY KEY' \
             ) pk ON c.column_name = pk.column_name \
             WHERE c.table_schema = {escaped_db} \
             AND c.table_name = {escaped_table} \
             ORDER BY c.ordinal_position"
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
            "SELECT i.relname AS name, \
             array_agg(a.attname ORDER BY x.indkey) AS columns, \
             NOT x.indisunique AS non_unique, \
             am.amname AS index_type \
             FROM pg_class t \
             JOIN pg_index x ON t.oid = x.indrelid \
             JOIN pg_class i ON i.oid = x.indexrelid \
             JOIN pg_am am ON i.relam = am.oid \
             JOIN pg_namespace n ON n.oid = t.relnamespace \
             JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(x.indkey) \
             WHERE n.nspname = '{db}' AND t.relname = '{table}' \
             GROUP BY i.relname, x.indisunique, am.amname \
             ORDER BY i.relname"
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| {
                let non_unique: bool = r.try_get("non_unique").unwrap_or(true);
                let cols_str: String = r.try_get("columns").unwrap_or_default();
                IndexInfo {
                    name: r.try_get("name").unwrap_or_default(),
                    columns: cols_str
                        .trim_matches('{')
                        .trim_matches('}')
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    unique: !non_unique,
                    index_type: r.try_get("index_type").unwrap_or_default(),
                }
            })
            .collect())
    }

    async fn foreign_keys(&mut self, db: &str, table: &str) -> Result<Vec<ForeignKeyInfo>> {
        let sql = format!(
            "SELECT tc.constraint_name AS name, \
             array_agg(kcu.column_name ORDER BY kcu.ordinal_position) AS columns, \
             ccu.table_name AS ref_table, \
             array_agg(ccu.column_name ORDER BY kcu.ordinal_position) AS ref_columns, \
             rc.delete_rule AS on_delete, rc.update_rule AS on_update \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
             JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name \
             JOIN information_schema.referential_constraints rc ON tc.constraint_name = rc.constraint_name \
             WHERE tc.table_schema = '{db}' AND tc.table_name = '{table}' \
               AND tc.constraint_type = 'FOREIGN KEY' \
             GROUP BY tc.constraint_name, ccu.table_name, rc.delete_rule, rc.update_rule"
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| {
                let cols_str: String = r.try_get("columns").unwrap_or_default();
                let ref_cols_str: String = r.try_get("ref_columns").unwrap_or_default();
                ForeignKeyInfo {
                    name: r.try_get("name").unwrap_or_default(),
                    columns: cols_str
                        .trim_matches('{')
                        .trim_matches('}')
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    ref_table: r.try_get("ref_table").unwrap_or_default(),
                    ref_columns: ref_cols_str
                        .trim_matches('{')
                        .trim_matches('}')
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    on_delete: r.try_get("on_delete").unwrap_or_default(),
                    on_update: r.try_get("on_update").unwrap_or_default(),
                }
            })
            .collect())
    }

    async fn ddl(&mut self, db: &str, table: &str) -> Result<DdlResult> {
        let sql = format!(
            "SELECT 'CREATE TABLE {table} (' || \
             string_agg(column_def, ', ' ORDER BY ordinal_position) || \
             ') ' || COALESCE(table_options, '') AS ddl \
             FROM ( \
               SELECT c.column_name, \
                 c.data_type || \
                 CASE WHEN c.character_maximum_length IS NOT NULL \
                   THEN '(' || c.character_maximum_length || ')' ELSE '' END || \
                 CASE WHEN c.is_nullable = 'NO' THEN ' NOT NULL' ELSE '' END || \
                 CASE WHEN c.column_default IS NOT NULL THEN ' DEFAULT ' || c.column_default ELSE '' END \
                 AS column_def, \
                 c.ordinal_position, \
                 '' AS table_options \
               FROM information_schema.columns c \
               WHERE c.table_schema = '{db}' AND c.table_name = '{table}' \
             ) sub"
        );
        let rows = sqlx::query_scalar::<_, String>(&sql)
            .fetch_all(&self.pool)
            .await?;
        let ddl = rows.first().cloned().unwrap_or_default();
        Ok(DdlResult { ddl })
    }
}
