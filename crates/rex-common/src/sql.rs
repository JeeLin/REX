//! SQL 连接器 trait 和共享类型，供 SQL Console 模块使用。
//!
//! 各数据库的实现分别在 `rex-mysql`、`rex-postgresql`、`rex-sqlite` crate 中。
//! `SqlConnectorFactory` 的实际连接逻辑由 `rex-hub` 在组装时注入。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 共享类型
// ---------------------------------------------------------------------------

/// 查询结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// 列定义。
    pub columns: Vec<ColumnInfo>,
    /// 行数据，每行是列值数组。
    pub rows: Vec<Vec<serde_json::Value>>,
    /// 受影响的行数（INSERT / UPDATE / DELETE）。
    pub affected_rows: u64,
    /// 查询耗时（毫秒）。
    pub elapsed_ms: u64,
}

/// 列信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
}

/// 表 / 视图信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    /// `"BASE TABLE"` 或 `"VIEW"`。
    pub table_type: String,
}

/// 索引信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    /// `BTREE` / `HASH` / `FULLTEXT` / `GIN` / `GIST` 等。
    pub index_type: String,
}

/// 外键信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
    pub on_delete: String,
    pub on_update: String,
}

/// DDL 预览结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdlResult {
    pub ddl: String,
}

/// 建立连接时的请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectRequest {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub database: Option<String>,
}

/// 方言探测结果。
///
/// v0.70.7 连接入口 dialect 探测：当 `db_type` 缺省时，按端口预判 → 双线缆协议握手
/// 回退 → `SELECT VERSION()` 确认，最终解析出 [`DatabaseType`] 的 [`String`] 表示
/// （mysql / postgresql / sqlite）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedDialect {
    MySQL,
    PostgreSQL,
    SQLite,
    ClickHouse,
    SqlServer,
    MariaDB,
    Oracle,
}

impl DetectedDialect {
    /// 解析为协议层使用的 db_type 字符串。
    pub fn as_str(&self) -> &'static str {
        match self {
            DetectedDialect::MySQL => "mysql",
            DetectedDialect::PostgreSQL => "postgresql",
            DetectedDialect::SQLite => "sqlite",
            DetectedDialect::ClickHouse => "clickhouse",
            DetectedDialect::SqlServer => "sqlserver",
            DetectedDialect::MariaDB => "mariadb",
            DetectedDialect::Oracle => "oracle",
        }
    }

    /// 从连接返回的连接器里读取方言（每个连接器已实现 `database_type()`）。
    pub fn from_connector(conn: &dyn SqlConnector) -> Option<DetectedDialect> {
        match conn.database_type() {
            DatabaseType::MySQL => Some(DetectedDialect::MySQL),
            DatabaseType::MariaDB => Some(DetectedDialect::MariaDB),
            DatabaseType::PostgreSQL => Some(DetectedDialect::PostgreSQL),
            DatabaseType::SQLite => Some(DetectedDialect::SQLite),
            DatabaseType::ClickHouse => Some(DetectedDialect::ClickHouse),
            DatabaseType::SqlServer => Some(DetectedDialect::SqlServer),
            DatabaseType::Oracle => Some(DetectedDialect::Oracle),
        }
    }
}

/// 按端口返回候选方言列表（已知端口优先排对应方言，未知端口全量尝试）。
///
/// 调用方应依次对候选列表做协议握手，首个成功的即为正确方言。
pub fn candidates_for_port(port: u16) -> &'static [DatabaseType] {
    match port {
        3306 => &[DatabaseType::MySQL, DatabaseType::PostgreSQL],
        5432 => &[DatabaseType::PostgreSQL, DatabaseType::MySQL],
        2883 | 1521 => &[
            DatabaseType::Oracle,
            DatabaseType::MySQL,
            DatabaseType::PostgreSQL,
        ],
        1433 => &[
            DatabaseType::SqlServer,
            DatabaseType::MySQL,
            DatabaseType::PostgreSQL,
        ],
        8123 | 9000 => &[
            DatabaseType::ClickHouse,
            DatabaseType::MySQL,
            DatabaseType::PostgreSQL,
        ],
        _ => &[
            DatabaseType::MySQL,
            DatabaseType::PostgreSQL,
            DatabaseType::Oracle,
            DatabaseType::SqlServer,
            DatabaseType::ClickHouse,
        ],
    }
}

// ---------------------------------------------------------------------------
// SqlConnector trait
// ---------------------------------------------------------------------------

/// SQL 查询配置。
#[derive(Debug, Clone)]
pub struct QueryConfig {
    /// 查询超时（秒），默认 30 秒。
    pub timeout_secs: u64,
    /// 最大返回行数，默认 10000 行。
    pub max_rows: usize,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_rows: 10000,
        }
    }
}

/// SQL 连接器的统一接口，由各数据库 crate 分别实现。
#[async_trait]
pub trait SqlConnector: Send {
    /// 当前连接器对应的数据库类型（v0.70.7：探测后回写 dialect 用）。
    fn database_type(&self) -> DatabaseType;

    /// 执行 SQL 语句并返回结果。
    async fn execute(&mut self, sql: &str) -> anyhow::Result<QueryResult>;

    /// 列出所有数据库。
    async fn databases(&mut self) -> anyhow::Result<Vec<String>>;

    /// 列出指定数据库中的所有表。
    async fn tables(&mut self, db: &str) -> anyhow::Result<Vec<TableInfo>>;

    /// 列出指定表的列信息。
    async fn columns(&mut self, db: &str, table: &str) -> anyhow::Result<Vec<ColumnInfo>>;

    /// 列出指定表的索引信息。
    async fn indexes(&mut self, _db: &str, _table: &str) -> anyhow::Result<Vec<IndexInfo>> {
        Ok(vec![])
    }

    /// 列出指定表的外键信息。
    async fn foreign_keys(
        &mut self,
        _db: &str,
        _table: &str,
    ) -> anyhow::Result<Vec<ForeignKeyInfo>> {
        Ok(vec![])
    }

    /// 获取指定表的 DDL（CREATE TABLE 语句）。
    async fn ddl(&mut self, _db: &str, _table: &str) -> anyhow::Result<DdlResult> {
        anyhow::bail!("DDL not supported")
    }

    /// 关闭连接，释放资源。
    async fn close(&mut self) -> anyhow::Result<()>;
}

// ---------------------------------------------------------------------------
// SqlConnectorFactory
// ---------------------------------------------------------------------------

/// 数据库类型。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DatabaseType {
    MySQL,
    PostgreSQL,
    SQLite,
    ClickHouse,
    SqlServer,
    MariaDB,
    Oracle,
}

impl DatabaseType {
    /// 转为持久化用的小写字符串。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MySQL => "mysql",
            Self::PostgreSQL => "postgresql",
            Self::SQLite => "sqlite",
            Self::ClickHouse => "clickhouse",
            Self::SqlServer => "sqlserver",
            Self::MariaDB => "mariadb",
            Self::Oracle => "oracle",
        }
    }
}

/// 连接器工厂，根据数据库类型创建对应的 [`SqlConnector`] 实现。
pub struct SqlConnectorFactory {
    db_type: DatabaseType,
}

impl SqlConnectorFactory {
    pub fn new(db_type: DatabaseType) -> Self {
        Self { db_type }
    }

    pub fn db_type(&self) -> DatabaseType {
        self.db_type
    }

    /// 根据连接请求创建并返回一个 [`SqlConnector`] 实现。
    ///
    /// 由 rex-hub 层提供实际分发，此处仅作类型封装。
    pub async fn connect(&self, _req: ConnectRequest) -> anyhow::Result<Box<dyn SqlConnector>> {
        anyhow::bail!("connect must be wired in rex-hub")
    }
}

// ---------------------------------------------------------------------------
// 共享方言探测
// ---------------------------------------------------------------------------

/// 方言探测结果：已连接的连接器 + 持久化用的方言字符串。
pub struct DetectResult {
    pub conn: Box<dyn SqlConnector>,
    pub dialect: &'static str,
}

/// 共享方言探测算法：端口预判 → 协议握手 → Oracle 跳过 → SELECT VERSION() 确认。
///
/// `connect_fn` 负责按 `DatabaseType` 创建连接器（Agent/Hub 各自注入实现）。
pub async fn detect_dialect<F, Fut>(
    req: ConnectRequest,
    connect_fn: F,
) -> anyhow::Result<DetectResult>
where
    F: Fn(DatabaseType, ConnectRequest) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<Box<dyn SqlConnector>>>,
{
    // SQLite：无 host 或 port 为 0 视为本地文件库。
    if req.host.is_empty() || req.port == 0 {
        let conn = connect_fn(DatabaseType::SQLite, req.clone()).await?;
        return Ok(DetectResult {
            conn,
            dialect: "sqlite",
        });
    }

    let candidates = candidates_for_port(req.port);

    for &dt in candidates {
        let label = format!("{:?}", dt);
        match connect_fn(dt, req.clone()).await {
            Ok(mut conn) => {
                // Oracle 不支持 SELECT VERSION()，协议握手成功即确认。
                if dt == DatabaseType::Oracle {
                    tracing::info!(port = req.port, dialect = ?dt, "dialect detected (Oracle, protocol handshake OK)");
                    return Ok(DetectResult {
                        conn,
                        dialect: dt.as_str(),
                    });
                }
                tracing::debug!(dialect = %label, "protocol handshake succeeded, trying SELECT VERSION()");
                match conn.execute("SELECT VERSION()").await {
                    Ok(result) => {
                        let version = result
                            .rows
                            .first()
                            .and_then(|r| r.first())
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        let confirmed = if version.to_uppercase().contains("POSTGRESQL") {
                            DatabaseType::PostgreSQL
                        } else {
                            dt
                        };
                        tracing::info!(
                            port = req.port,
                            version = %version,
                            dialect = ?confirmed,
                            "dialect detected"
                        );
                        // 确认结果与握手类型不同，重新连接。
                        let final_conn = if confirmed == dt {
                            conn
                        } else {
                            connect_fn(confirmed, req.clone()).await?
                        };
                        return Ok(DetectResult {
                            conn: final_conn,
                            dialect: confirmed.as_str(),
                        });
                    }
                    Err(e) => {
                        tracing::warn!(dialect = %label, error = %e, "SELECT VERSION() failed");
                        continue;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(dialect = %label, error = %e, "connection failed");
                continue;
            }
        }
    }

    anyhow::bail!(
        "unrecognized dialect (host={}, port={}). Please specify the subtype explicitly.",
        req.host,
        req.port,
    )
}

// ---------------------------------------------------------------------------
// 统一 SQL 操作分发
// ---------------------------------------------------------------------------

/// 统一 SQL 操作分发：按 kind 调用 connector 对应方法，返回 JSON。
/// Agent（WebSocket 隧道）和 Hub（HTTP handler）共用此函数。
pub async fn dispatch_sql(
    conn: &mut dyn SqlConnector,
    kind: &str,
    payload: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    match kind {
        "query" | "exec" => {
            let sql = payload.get("sql").and_then(|v| v.as_str()).unwrap_or("");
            let res: QueryResult = conn.execute(sql).await?;
            Ok(serde_json::json!({
                "columns": res.columns,
                "rows": res.rows,
                "affected_rows": res.affected_rows,
                "elapsed_ms": res.elapsed_ms,
            }))
        }
        "databases" => {
            let dbs = conn.databases().await?;
            Ok(serde_json::json!({ "databases": dbs }))
        }
        "tables" => {
            let db = payload.get("db").and_then(|v| v.as_str()).unwrap_or("");
            let t = conn.tables(db).await?;
            Ok(serde_json::json!({ "tables": t }))
        }
        "columns" => {
            let db = payload.get("db").and_then(|v| v.as_str()).unwrap_or("");
            let table = payload.get("table").and_then(|v| v.as_str()).unwrap_or("");
            let c = conn.columns(db, table).await?;
            Ok(serde_json::json!({ "columns": c }))
        }
        "indexes" => {
            let db = payload.get("db").and_then(|v| v.as_str()).unwrap_or("");
            let table = payload.get("table").and_then(|v| v.as_str()).unwrap_or("");
            let idx = conn.indexes(db, table).await?;
            Ok(serde_json::json!({ "indexes": idx }))
        }
        "foreign_keys" => {
            let db = payload.get("db").and_then(|v| v.as_str()).unwrap_or("");
            let table = payload.get("table").and_then(|v| v.as_str()).unwrap_or("");
            let fks = conn.foreign_keys(db, table).await?;
            Ok(serde_json::json!({ "foreign_keys": fks }))
        }
        "ddl" => {
            let db = payload.get("db").and_then(|v| v.as_str()).unwrap_or("");
            let table = payload.get("table").and_then(|v| v.as_str()).unwrap_or("");
            let d = conn.ddl(db, table).await?;
            Ok(serde_json::json!({ "ddl": d }))
        }
        "close" => {
            let _ = conn.close().await;
            Ok(serde_json::json!({ "closed": true }))
        }
        other => anyhow::bail!("unsupported sql request kind: {other}"),
    }
}
