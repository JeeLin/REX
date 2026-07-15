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

/// 建立连接时的请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectRequest {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub database: Option<String>,
}

// ---------------------------------------------------------------------------
// SqlConnector trait
// ---------------------------------------------------------------------------

/// SQL 连接器的统一接口，由各数据库 crate 分别实现。
#[async_trait]
pub trait SqlConnector: Send {
    /// 执行 SQL 语句并返回结果。
    async fn execute(&mut self, sql: &str) -> anyhow::Result<QueryResult>;

    /// 列出所有数据库。
    async fn databases(&mut self) -> anyhow::Result<Vec<String>>;

    /// 列出指定数据库中的所有表。
    async fn tables(&mut self, db: &str) -> anyhow::Result<Vec<TableInfo>>;

    /// 列出指定表的列信息。
    async fn columns(&mut self, db: &str, table: &str) -> anyhow::Result<Vec<ColumnInfo>>;

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

/// 连接器工厂，根据数据库类型创建对应的 [`SqlConnector`] 实例。
///
/// 实际的 `connect` 实现在 `rex-hub` 层通过注册表注入，
/// 此处仅提供类型枚举和 trait 签名。
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
    /// 默认返回 `Err`——调用方应在 `rex-hub` 中提供具体的工厂函数
    /// 并通过 [`Self::new_with_connector`] 注入。
    pub async fn connect(&self, _req: ConnectRequest) -> anyhow::Result<Box<dyn SqlConnector>> {
        anyhow::bail!(
            "no connector registered for {:?}; wire the factory in rex-hub",
            self.db_type
        )
    }
}
