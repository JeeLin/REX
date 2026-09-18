//! ClickHouse 协议实现 — 基于 HTTP API 的 SqlConnector。

use anyhow::{Context, Result};
use rex_common::bracket_host;
use rex_common::sql::{
    ColumnInfo, ConnectRequest, DdlResult, ForeignKeyInfo, IndexInfo, QueryResult, SqlConnector,
    TableInfo,
};

/// ClickHouse 连接器（使用 HTTP 接口）
pub struct ClickHouseConnector {
    client: reqwest::Client,
    base_url: String,
    database: String,
    username: String,
    password: Option<String>,
}

impl ClickHouseConnector {
    /// 建立 ClickHouse 连接
    pub async fn connect(req: ConnectRequest) -> Result<Self> {
        let host = bracket_host(&req.host);
        let base_url = format!("http://{}:{}", host, req.port);

        let client = reqwest::Client::new();

        // 测试连接
        let url = format!("{}/?query=SELECT%201", base_url);
        let mut request = client.get(&url);

        if !req.username.is_empty() {
            request = request.basic_auth(&req.username, req.password.as_deref());
        }

        request.send().await.with_context(|| {
            format!(
                "failed to connect to ClickHouse at {}:{}",
                req.host, req.port
            )
        })?;

        let connector = Self {
            client,
            base_url,
            database: req.database.as_deref().unwrap_or("default").to_string(),
            username: req.username,
            password: req.password,
        };

        // 设置默认数据库
        if !connector.database.is_empty() {
            let _ = connector
                .http_query(&format!(
                    "SET default_database = '{}'",
                    connector.database.replace("'", "''")
                ))
                .await;
        }

        Ok(connector)
    }

    /// 执行 HTTP 查询
    async fn http_query(&self, sql: &str) -> Result<String> {
        // Request TabSeparatedWithNames format to include header row
        let url = format!(
            "{}/?query={}&default_format=TabSeparatedWithNames",
            self.base_url,
            urlencoding::encode(sql)
        );
        let mut request = self.client.get(&url);

        if !self.username.is_empty() {
            request = request.basic_auth(&self.username, self.password.as_deref());
        }

        let response = request
            .send()
            .await
            .with_context(|| format!("failed to execute query: {sql}"))?;

        // 检查 HTTP 状态码
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("HTTP {}: {}", status, error_text);
        }

        response
            .text()
            .await
            .with_context(|| format!("failed to read response for query: {sql}"))
    }

    /// 尝试获取非查询语句的受影响行数（ClickHouse 私有方法）
    async fn get_affected_rows(&mut self, sql: &str) -> Result<u64> {
        let trimmed_sql = sql.trim().to_uppercase();
        if trimmed_sql.starts_with("INSERT")
            || trimmed_sql.starts_with("UPDATE")
            || trimmed_sql.starts_with("DELETE")
            || trimmed_sql.starts_with("CREATE")
            || trimmed_sql.starts_with("ALTER")
            || trimmed_sql.starts_with("DROP")
            || trimmed_sql.starts_with("TRUNCATE")
        {
            let response = self
                .http_query(&format!(
                    "SELECT rows FROM system.query_log WHERE query = '{}' ORDER BY event_time DESC LIMIT 1",
                    ch_escape(sql)
                ))
                .await?;
            let (_, rows) = parse_csv_response(&response);

            if let Some(row) = rows.first() {
                if let Some(value) = row.first() {
                    if let Some(rows_affected) = value.as_u64() {
                        return Ok(rows_affected);
                    }
                }
            }
        }

        Ok(0)
    }
}

/// Escape a string for use in ClickHouse SQL string literals (single-quote escaping).
fn ch_escape(s: &str) -> String {
    s.replace('\'', "''")
}

/// 解析 CSV 响应为行数据（实际上是 TabSeparatedWithNames）
fn parse_csv_response(response: &str) -> (Vec<String>, Vec<Vec<serde_json::Value>>) {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .from_reader(response.as_bytes());
    let headers: Vec<String> = reader
        .headers()
        .map(|h| h.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let mut rows = Vec::new();
    for record in reader.records().flatten() {
        let values: Vec<serde_json::Value> = record
            .iter()
            .map(|field| {
                if let Ok(n) = field.parse::<i64>() {
                    serde_json::json!(n)
                } else if let Ok(f) = field.parse::<f64>() {
                    serde_json::json!(f)
                } else {
                    serde_json::json!(field)
                }
            })
            .collect();
        rows.push(values);
    }

    (headers, rows)
}

#[async_trait::async_trait]
impl SqlConnector for ClickHouseConnector {
    fn database_type(&self) -> rex_common::sql::DatabaseType {
        rex_common::sql::DatabaseType::ClickHouse
    }

    async fn execute(&mut self, sql: &str) -> Result<QueryResult> {
        let start = std::time::Instant::now();
        let trimmed = sql.trim_start().to_uppercase();
        let is_query = trimmed.starts_with("SELECT")
            || trimmed.starts_with("SHOW")
            || trimmed.starts_with("DESCRIBE")
            || trimmed.starts_with("EXPLAIN");

        if is_query {
            // For queries, use TabSeparatedWithNames format to get column names and data
            let response = self.http_query(sql).await?;
            let (headers, rows) = parse_csv_response(&response);

            let columns = headers
                .iter()
                .map(|name| ColumnInfo {
                    name: name.clone(),
                    data_type: "String".to_string(),
                    nullable: true,
                    is_primary_key: false,
                })
                .collect();

            let elapsed = start.elapsed().as_millis() as u64;
            Ok(QueryResult {
                columns,
                rows,
                affected_rows: 0,
                elapsed_ms: elapsed,
            })
        } else {
            // For non-queries (INSERT, UPDATE, DELETE, etc.), we try to get affected rows
            // ClickHouse returns summary information in a different format
            let affected_rows = self.get_affected_rows(sql).await?;
            let elapsed = start.elapsed().as_millis() as u64;
            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows,
                elapsed_ms: elapsed,
            })
        }
    }

    async fn databases(&mut self) -> Result<Vec<String>> {
        let response = self
            .http_query("SELECT name FROM system.databases ORDER BY name")
            .await?;
        let (_, rows) = parse_csv_response(&response);

        Ok(rows
            .into_iter()
            .filter_map(|row| row.first().and_then(|v| v.as_str().map(String::from)))
            .collect())
    }

    async fn tables(&mut self, db: &str) -> Result<Vec<TableInfo>> {
        let response = self
            .http_query(&format!(
                "SELECT name, engine FROM system.tables WHERE database = '{}' ORDER BY name",
                ch_escape(db)
            ))
            .await?;
        let (_, rows) = parse_csv_response(&response);

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                if row.len() >= 2 {
                    Some(TableInfo {
                        name: row[0].as_str()?.to_string(),
                        table_type: row[1].as_str()?.to_string(),
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    async fn columns(&mut self, db: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let response = self
            .http_query(&format!(
                "SELECT name, type, is_in_primary_key FROM system.columns WHERE database = '{}' AND table = '{}' ORDER BY position",
                ch_escape(db), ch_escape(table)
            ))
            .await?;
        let (_, rows) = parse_csv_response(&response);

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                if row.len() >= 3 {
                    Some(ColumnInfo {
                        name: row[0].as_str()?.to_string(),
                        data_type: row[1].as_str()?.to_string(),
                        nullable: true,
                        is_primary_key: row[2].as_str().unwrap_or("0") == "1",
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    async fn indexes(&mut self, db: &str, table: &str) -> Result<Vec<IndexInfo>> {
        // Query system.indices for index information
        let response = self
            .http_query(&format!(
                "SELECT name, type, columns FROM system.indices WHERE database = '{}' AND table = '{}'",
                ch_escape(db), ch_escape(table)
            ))
            .await?;
        let (_, rows) = parse_csv_response(&response);

        let mut indexes = Vec::new();
        for row in rows {
            if row.len() >= 3 {
                // Parse columns array from string representation
                let index_name = row[0].as_str().unwrap_or("").to_string();
                let index_type = row[1].as_str().unwrap_or("").to_string();
                let columns_str = row[2].as_str().unwrap_or("[]").to_string();

                // Parse columns from string like "[col1,col2]" or "'col1','col2'"
                let mut columns = Vec::new();
                if columns_str.starts_with('[') && columns_str.ends_with(']') {
                    let inner = &columns_str[1..columns_str.len() - 1];
                    if !inner.is_empty() {
                        for col in inner.split(',') {
                            let trimmed = col.trim().trim_matches(|c| c == '"' || c == '\'');
                            if !trimmed.is_empty() {
                                columns.push(trimmed.to_string());
                            }
                        }
                    }
                }

                indexes.push(IndexInfo {
                    name: index_name,
                    index_type,
                    columns,
                    unique: false, // ClickHouse indices are not necessarily unique in traditional sense
                });
            }
        }

        Ok(indexes)
    }

    async fn foreign_keys(&mut self, _db: &str, _table: &str) -> Result<Vec<ForeignKeyInfo>> {
        Ok(vec![])
    }

    async fn ddl(&mut self, db: &str, table: &str) -> Result<DdlResult> {
        // Use SHOW CREATE TABLE to get the DDL statement
        // ClickHouse identifiers need backtick escaping
        let escaped_db = db.replace('`', "``");
        let escaped_table = table.replace('`', "``");
        let response = self
            .http_query(&format!(
                "SHOW CREATE TABLE `{}`.`{}`",
                escaped_db, escaped_table
            ))
            .await?;
        let (_, rows) = parse_csv_response(&response);

        // SHOW CREATE TABLE returns two columns: 'table' and 'statement'
        // We want the statement column (second column)
        if let Some(row) = rows.first() {
            if row.len() >= 2 {
                if let Some(ddl) = row[1].as_str() {
                    return Ok(DdlResult {
                        ddl: ddl.to_string(),
                    });
                }
            }
        }

        // If we couldn't get the DDL, return an error
        anyhow::bail!("Failed to retrieve DDL for {}.{}", db, table)
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
