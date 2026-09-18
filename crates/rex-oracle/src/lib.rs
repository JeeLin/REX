//! Oracle 数据库实现 — 基于 oracle-rs 的纯 Rust SqlConnector。
//!
//! 使用 oracle-rs 纯 Rust 驱动，无需安装 Oracle 客户端库。

use anyhow::{Context, Result};
use rex_common::bracket_host;
use rex_common::sql::{
    ColumnInfo, ConnectRequest, DdlResult, ForeignKeyInfo, IndexInfo, QueryResult, SqlConnector,
    TableInfo,
};

/// Oracle 连接器
pub struct OracleConnector {
    conn: Option<oracle_rs::Connection>,
}

impl OracleConnector {
    /// 建立 Oracle 连接
    pub async fn connect(req: ConnectRequest) -> Result<Self> {
        let host = bracket_host(&req.host);
        let database = req.database.as_deref().unwrap_or("ORCL");
        let username = &req.username;
        let password = req.password.as_deref().unwrap_or("");

        let config = oracle_rs::Config::new(host, req.port, database, username, password);

        let conn = oracle_rs::Connection::connect_with_config(config)
            .await
            .with_context(|| {
                format!(
                    "failed to connect to Oracle at {}:{}/{}",
                    req.host, req.port, database
                )
            })?;

        Ok(Self { conn: Some(conn) })
    }

    fn conn(&mut self) -> Result<&oracle_rs::Connection> {
        self.conn
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Oracle connection is closed"))
    }
}

#[async_trait::async_trait]
impl SqlConnector for OracleConnector {
    fn database_type(&self) -> rex_common::sql::DatabaseType {
        rex_common::sql::DatabaseType::Oracle
    }

    async fn execute(&mut self, sql: &str) -> Result<QueryResult> {
        let start = std::time::Instant::now();
        let conn = self.conn()?;
        let trimmed = sql.trim_start().to_uppercase();
        let is_query = trimmed.starts_with("SELECT") || trimmed.starts_with("WITH");

        if is_query {
            let result = conn
                .query(sql, &[])
                .await
                .with_context(|| format!("failed to execute query: {sql}"))?;

            let columns = result
                .columns
                .iter()
                .map(|c| ColumnInfo {
                    name: c.name.clone(),
                    data_type: format!("{:?}", c.oracle_type),
                    nullable: c.nullable,
                    is_primary_key: false,
                })
                .collect();

            let mut data = Vec::new();
            for row in &result.rows {
                let mut values = Vec::new();
                for i in 0..row.len() {
                    let val = if let Some(v) = row.get_i64(i) {
                        serde_json::json!(v)
                    } else if let Some(v) = row.get_f64(i) {
                        serde_json::json!(v)
                    } else if let Some(v) = row.get_string(i) {
                        serde_json::json!(v)
                    } else if row.is_null(i) {
                        serde_json::Value::Null
                    } else {
                        // Fallback: try Value enum
                        match row.get(i) {
                            Some(oracle_rs::Value::String(s)) => serde_json::json!(s),
                            Some(oracle_rs::Value::Integer(n)) => serde_json::json!(n),
                            Some(oracle_rs::Value::Float(f)) => serde_json::json!(f),
                            Some(oracle_rs::Value::Boolean(b)) => serde_json::json!(b),
                            _ => serde_json::Value::Null,
                        }
                    };
                    values.push(val);
                }
                data.push(values);
            }

            let elapsed = start.elapsed().as_millis() as u64;
            Ok(QueryResult {
                columns,
                rows: data,
                affected_rows: 0,
                elapsed_ms: elapsed,
            })
        } else {
            let result = conn
                .execute(sql, &[])
                .await
                .with_context(|| format!("failed to execute statement: {sql}"))?;
            let elapsed = start.elapsed().as_millis() as u64;
            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: result.rows_affected,
                elapsed_ms: elapsed,
            })
        }
    }

    async fn databases(&mut self) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let result = conn
            .query("SELECT name FROM v$database ORDER BY name", &[])
            .await?;
        let mut dbs = Vec::new();
        for row in &result.rows {
            if let Some(name) = row.get_string(0) {
                dbs.push(name.to_string());
            }
        }
        Ok(dbs)
    }

    async fn tables(&mut self, db: &str) -> Result<Vec<TableInfo>> {
        let conn = self.conn()?;
        let owner = db.to_uppercase();
        let result = conn
            .query(
                "SELECT table_name, 'BASE TABLE' AS table_type \
                 FROM all_tables \
                 WHERE owner = :1 \
                 UNION ALL \
                 SELECT view_name, 'VIEW' AS table_type \
                 FROM all_views \
                 WHERE owner = :1 \
                 ORDER BY table_name",
                &[oracle_rs::Value::String(owner)],
            )
            .await?;

        let mut tables = Vec::new();
        for row in &result.rows {
            let name = row.get_string(0).unwrap_or_default().to_string();
            let table_type = row.get_string(1).unwrap_or_default().to_string();
            tables.push(TableInfo { name, table_type });
        }
        Ok(tables)
    }

    async fn columns(&mut self, db: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let conn = self.conn()?;
        let owner = db.to_uppercase();
        let tbl = table.to_uppercase();
        let result = conn
            .query(
                "SELECT tc.column_name, tc.data_type, tc.nullable, \
                 CASE WHEN pk.column_name IS NOT NULL THEN 1 ELSE 0 END AS is_primary_key \
                 FROM all_tab_columns tc \
                 LEFT JOIN ( \
                     SELECT acc.column_name \
                     FROM all_constraints ac \
                     JOIN all_cons_columns acc ON ac.constraint_name = acc.constraint_name \
                         AND ac.owner = acc.owner \
                     WHERE ac.constraint_type = 'P' \
                         AND ac.owner = :1 \
                         AND ac.table_name = :2 \
                 ) pk ON tc.column_name = pk.column_name \
                 WHERE tc.owner = :1 \
                     AND tc.table_name = :2 \
                 ORDER BY tc.column_id",
                &[
                    oracle_rs::Value::String(owner),
                    oracle_rs::Value::String(tbl),
                ],
            )
            .await?;

        let mut columns = Vec::new();
        for row in &result.rows {
            let name = row.get_string(0).unwrap_or_default().to_string();
            let data_type = row.get_string(1).unwrap_or_default().to_string();
            let nullable_str = row.get_string(2).unwrap_or_default();
            let is_pk = row.get_i64(3).unwrap_or(0);

            columns.push(ColumnInfo {
                name,
                data_type,
                nullable: nullable_str == "Y",
                is_primary_key: is_pk != 0,
            });
        }
        Ok(columns)
    }

    async fn indexes(&mut self, db: &str, table: &str) -> Result<Vec<IndexInfo>> {
        let conn = self.conn()?;
        let owner = db.to_uppercase();
        let tbl = table.to_uppercase();
        let result = conn
            .query(
                "SELECT ai.index_name, \
                        LISTAGG(aic.column_name, ',') WITHIN GROUP (ORDER BY aic.column_position) AS columns, \
                        CASE WHEN ai.uniqueness = 'UNIQUE' THEN 1 ELSE 0 END AS is_unique, \
                        ai.index_type \
                 FROM all_indexes ai \
                 JOIN all_ind_columns aic ON ai.index_name = aic.index_name \
                     AND ai.owner = aic.index_owner \
                 WHERE ai.table_owner = :1 \
                     AND ai.table_name = :2 \
                 GROUP BY ai.index_name, ai.uniqueness, ai.index_type \
                 ORDER BY ai.index_name",
                &[
                    oracle_rs::Value::String(owner),
                    oracle_rs::Value::String(tbl),
                ],
            )
            .await?;

        let mut indexes = Vec::new();
        for row in &result.rows {
            let name = row.get_string(0).unwrap_or_default().to_string();
            let columns_str = row.get_string(1).unwrap_or_default();
            let unique = row.get_i64(2).unwrap_or(0);
            let index_type = row.get_string(3).unwrap_or_default().to_string();

            let columns = columns_str.split(',').map(String::from).collect();

            indexes.push(IndexInfo {
                name,
                index_type,
                columns,
                unique: unique != 0,
            });
        }
        Ok(indexes)
    }

    async fn foreign_keys(&mut self, db: &str, table: &str) -> Result<Vec<ForeignKeyInfo>> {
        let conn = self.conn()?;
        let owner = db.to_uppercase();
        let tbl = table.to_uppercase();
        let result = conn
            .query(
                "SELECT ac.constraint_name, \
                        LISTAGG(acc.column_name, ',') WITHIN GROUP (ORDER BY acc.position) AS columns, \
                        arc.table_name AS ref_table, \
                        LISTAGG(arcc.column_name, ',') WITHIN GROUP (ORDER BY arcc.position) AS ref_columns, \
                        ac.delete_rule, ac.update_rule \
                 FROM all_constraints ac \
                 JOIN all_cons_columns acc ON ac.constraint_name = acc.constraint_name \
                     AND ac.owner = acc.owner \
                 JOIN all_constraints arc ON ac.r_constraint_name = arc.constraint_name \
                     AND ac.r_owner = arc.owner \
                 JOIN all_cons_columns arcc ON arc.constraint_name = arcc.constraint_name \
                     AND arc.owner = arcc.owner \
                 WHERE ac.constraint_type = 'R' \
                     AND ac.owner = :1 \
                     AND ac.table_name = :2 \
                 GROUP BY ac.constraint_name, arc.table_name, ac.delete_rule, ac.update_rule \
                 ORDER BY ac.constraint_name",
                &[
                    oracle_rs::Value::String(owner),
                    oracle_rs::Value::String(tbl),
                ],
            )
            .await?;

        let mut fks = Vec::new();
        for row in &result.rows {
            let name = row.get_string(0).unwrap_or_default().to_string();
            let columns_str = row.get_string(1).unwrap_or_default();
            let ref_table = row.get_string(2).unwrap_or_default().to_string();
            let ref_columns_str = row.get_string(3).unwrap_or_default();
            let on_delete = row.get_string(4).unwrap_or_default().to_string();
            let on_update = row.get_string(5).unwrap_or_default().to_string();

            let columns = columns_str.split(',').map(String::from).collect();
            let ref_columns = ref_columns_str.split(',').map(String::from).collect();

            fks.push(ForeignKeyInfo {
                name,
                columns,
                ref_table,
                ref_columns,
                on_delete,
                on_update,
            });
        }
        Ok(fks)
    }

    async fn ddl(&mut self, db: &str, table: &str) -> Result<DdlResult> {
        let conn = self.conn()?;
        let owner = db.to_uppercase();
        let tbl = table.to_uppercase();
        let result = conn
            .query(
                "SELECT dbms_metadata.get_ddl('TABLE', :1, :2) AS ddl FROM dual",
                &[
                    oracle_rs::Value::String(tbl),
                    oracle_rs::Value::String(owner),
                ],
            )
            .await?;

        if let Some(row) = result.rows.first() {
            let ddl = row.get_string(0).unwrap_or_default().to_string();
            if !ddl.is_empty() {
                return Ok(DdlResult { ddl });
            }
        }

        // Fallback: construct a basic DDL from metadata
        let cols = self.columns(db, table).await?;
        let mut ddl = format!("CREATE TABLE {}.{} (\n", db, table);
        let col_defs: Vec<String> = cols
            .iter()
            .map(|c| {
                let mut def = format!("  {} {}", c.name, c.data_type);
                if !c.nullable {
                    def.push_str(" NOT NULL");
                }
                def
            })
            .collect();
        ddl.push_str(&col_defs.join(",\n"));
        ddl.push_str("\n)");

        Ok(DdlResult { ddl })
    }

    async fn close(&mut self) -> Result<()> {
        // Drop the connection
        self.conn = None;
        Ok(())
    }
}
