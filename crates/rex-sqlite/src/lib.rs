//! SQLite 协议实现 — 基于 rusqlite 的 SqlConnector。

use std::path::Path;

use anyhow::{Context, Result};
use rex_common::sql::{ColumnInfo, ConnectRequest, QueryResult, SqlConnector, TableInfo};

/// SQLite 连接器
pub struct SqliteConnector {
    conn: rusqlite::Connection,
}

impl SqliteConnector {
    /// 建立 SQLite 连接
    pub async fn connect(req: ConnectRequest) -> Result<Self> {
        // SQLite 的 host 字段作为数据库文件路径
        let db_path = if req.host.is_empty() || req.host == ":memory:" {
            ":memory:".to_string()
        } else {
            req.host.clone()
        };

        let conn = if db_path == ":memory:" {
            rusqlite::Connection::open_in_memory()
                .context("failed to open in-memory SQLite database")?
        } else {
            Self::ensure_parent_dir(&db_path)?;
            rusqlite::Connection::open(&db_path)
                .with_context(|| format!("failed to open SQLite database: {db_path}"))?
        };

        // 启用 WAL 模式（提升并发性能）
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .context("failed to set WAL mode")?;

        Ok(Self { conn })
    }

    fn ensure_parent_dir(path: &str) -> Result<()> {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory: {}", parent.display()))?;
            }
        }
        Ok(())
    }

    fn map_value(val: rusqlite::types::Value) -> serde_json::Value {
        use rusqlite::types::Value;
        match val {
            Value::Null => serde_json::Value::Null,
            Value::Integer(i) => serde_json::json!(i),
            Value::Real(f) => serde_json::json!(f),
            Value::Text(s) => serde_json::json!(s),
            Value::Blob(b) => {
                use base64::Engine;
                serde_json::json!(base64::engine::general_purpose::STANDARD.encode(&b))
            }
        }
    }
}

#[async_trait::async_trait]
impl SqlConnector for SqliteConnector {
    async fn execute(&mut self, sql: &str) -> Result<QueryResult> {
        let start = std::time::Instant::now();

        let trimmed = sql.trim_start().to_uppercase();
        let is_query = trimmed.starts_with("SELECT")
            || trimmed.starts_with("PRAGMA")
            || trimmed.starts_with("EXPLAIN");

        if is_query {
            let mut stmt = self
                .conn
                .prepare(sql)
                .with_context(|| format!("failed to prepare SQL: {sql}"))?;

            let columns: Vec<ColumnInfo> = stmt
                .column_names()
                .iter()
                .map(|name| ColumnInfo {
                    name: name.to_string(),
                    data_type: String::new(), // SQLite 动态类型，运行时推断
                    nullable: true,
                    is_primary_key: false,
                })
                .collect();

            let col_count = stmt.column_count();

            let mut rows = Vec::new();
            let mut row_iter = stmt.query([])?;
            while let Some(row) = row_iter.next()? {
                let values: Vec<serde_json::Value> = (0..col_count)
                    .map(|i| Self::map_value(row.get(i).unwrap_or(rusqlite::types::Value::Null)))
                    .collect();
                rows.push(values);
            }

            let elapsed = start.elapsed().as_millis() as u64;
            Ok(QueryResult {
                columns,
                rows,
                affected_rows: 0,
                elapsed_ms: elapsed,
            })
        } else {
            let affected = self
                .conn
                .execute(sql, [])
                .context("failed to execute SQL")?;
            let elapsed = start.elapsed().as_millis() as u64;
            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: affected as u64,
                elapsed_ms: elapsed,
            })
        }
    }

    async fn databases(&mut self) -> Result<Vec<String>> {
        let name: String = self
            .conn
            .query_row("PRAGMA database_list", [], |row| row.get(1))?;
        Ok(vec![name])
    }

    async fn tables(&mut self, _db: &str) -> Result<Vec<TableInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, type FROM sqlite_master \
             WHERE type IN ('table', 'view') \
             AND name NOT LIKE 'sqlite_%' \
             ORDER BY name",
        )?;
        let tables = stmt
            .query_map([], |row| {
                Ok(TableInfo {
                    name: row.get(0)?,
                    table_type: row.get(1)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(tables)
    }

    async fn columns(&mut self, _db: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let sql = format!("PRAGMA table_info('{table}')");
        let mut stmt = self.conn.prepare(&sql)?;
        let columns = stmt
            .query_map([], |row| {
                Ok(ColumnInfo {
                    name: row.get(1)?,
                    data_type: row.get(2)?,
                    nullable: !row.get::<_, bool>(3)?,
                    is_primary_key: row.get::<_, bool>(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(columns)
    }

    async fn close(&mut self) -> Result<()> {
        // rusqlite 连接在 drop 时自动关闭
        Ok(())
    }
}
