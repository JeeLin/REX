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
}

impl DetectedDialect {
    /// 解析为协议层使用的 db_type 字符串。
    pub fn as_str(&self) -> &'static str {
        match self {
            DetectedDialect::MySQL => "mysql",
            DetectedDialect::PostgreSQL => "postgresql",
            DetectedDialect::SQLite => "sqlite",
        }
    }

    /// 从连接返回的连接器里读取方言（每个连接器已实现 `database_type()`）。
    pub fn from_connector(conn: &dyn SqlConnector) -> Option<DetectedDialect> {
        match conn.database_type() {
            DatabaseType::MySQL => Some(DetectedDialect::MySQL),
            DatabaseType::PostgreSQL => Some(DetectedDialect::PostgreSQL),
            DatabaseType::SQLite => Some(DetectedDialect::SQLite),
        }
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
