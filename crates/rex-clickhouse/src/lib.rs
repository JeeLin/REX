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
    _database: String,
}

impl ClickHouseConnector {
    /// 建立 ClickHouse 连接
    pub async fn connect(req: ConnectRequest) -> Result<Self> {
        let database = req.database.as_deref().unwrap_or("default");
        let host = bracket_host(&req.host);
        let base_url = format!("http://{}:{}", host, req.port);

        let client = reqwest::Client::new();

        // 测试连接
        let url = format!("{}/?query=SELECT%201", base_url);
        let mut request = client.get(&url);

        if !req.username.is_empty() {
            request = request.basic_auth(&req.username, req.password.as_deref());
        }

        request
            .send()
            .await
            .with_context(|| format!("failed to connect to ClickHouse at {}:{}", req.host, req.port))?;

        Ok(Self {
            client,
            base_url,
            _database: database.to_string(),
        })
    }

    /// 执行 HTTP 查询
    async fn http_query(&self, sql: &str) -> Result<String> {
        let url = format!("{}/?query={}", self.base_url, urlencoding::encode(sql));
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to execute query: {sql}"))?;

        response
            .text()
            .await
            .with_context(|| format!("failed to read response for query: {sql}"))
    }

    /// 解析 CSV 响应为行数据
    fn parse_csv_response(response: &str) -> (Vec<String>, Vec<Vec<serde_json::Value>>) {
        let mut reader = csv::Reader::from_reader(response.as_bytes());
        let headers: Vec<String> = reader
            .headers()
            .map(|h| h.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        let mut rows = Vec::new();
        for result in reader.records() {
            if let Ok(record) = result {
                let values: Vec<serde_json::Value> = record
                    .iter()
                    .map(|field| {
                        // 尝试解析为数字
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
        }

        (headers, rows)
    }
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
            let response = self.http_query(sql).await?;
            let (headers, rows) = Self::parse_csv_response(&response);

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
            self.http_query(sql).await?;
            let elapsed = start.elapsed().as_millis() as u64;
            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                affected_rows: 0,
                elapsed_ms: elapsed,
            })
        }
    }

    async fn databases(&mut self) -> Result<Vec<String>> {
        let response = self
            .http_query("SELECT name FROM system.databases ORDER BY name")
            .await?;
        let (_, rows) = Self::parse_csv_response(&response);

        Ok(rows
            .into_iter()
            .filter_map(|row| row.first().and_then(|v| v.as_str().map(String::from)))
            .collect())
    }

    async fn tables(&mut self, db: &str) -> Result<Vec<TableInfo>> {
        let response = self
            .http_query(&format!(
                "SELECT name, engine FROM system.tables WHERE database = '{}' ORDER BY name",
                db
            ))
            .await?;
        let (_, rows) = Self::parse_csv_response(&response);

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
                db, table
            ))
            .await?;
        let (_, rows) = Self::parse_csv_response(&response);

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

    async fn indexes(&mut self, _db: &str, _table: &str) -> Result<Vec<IndexInfo>> {
        Ok(vec![])
    }

    async fn foreign_keys(&mut self, _db: &str, _table: &str) -> Result<Vec<ForeignKeyInfo>> {
        Ok(vec![])
    }

    async fn ddl(&mut self, _db: &str, _table: &str) -> Result<DdlResult> {
        anyhow::bail!("DDL not supported for ClickHouse")
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
