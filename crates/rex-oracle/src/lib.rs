//! Oracle 数据库实现 — stub 实现（真实驱动待集成）。
//!
//! 当前为桩实现，所有操作返回 "not yet implemented" 错误。
//! 后续可对接 oracle-rs 或 ODBC 驱动。

use anyhow::{bail, Result};
use rex_common::sql::{
    ColumnInfo, ConnectRequest, DdlResult, ForeignKeyInfo, IndexInfo, QueryResult, SqlConnector,
    TableInfo,
};

/// Oracle 连接器（stub）
pub struct OracleConnector {
    _host: String,
    _port: u16,
    _database: String,
}

impl OracleConnector {
    /// 建立 Oracle 连接（stub）
    pub async fn connect(req: ConnectRequest) -> Result<Self> {
        bail!(
            "Oracle driver not yet integrated. Connect requested: {}:{}",
            req.host,
            req.port
        );
    }
}

#[async_trait::async_trait]
impl SqlConnector for OracleConnector {
    fn database_type(&self) -> rex_common::sql::DatabaseType {
        rex_common::sql::DatabaseType::Oracle
    }

    async fn execute(&mut self, _sql: &str) -> Result<QueryResult> {
        bail!("Oracle driver not yet integrated")
    }

    async fn databases(&mut self) -> Result<Vec<String>> {
        bail!("Oracle driver not yet integrated")
    }

    async fn tables(&mut self, _db: &str) -> Result<Vec<TableInfo>> {
        bail!("Oracle driver not yet integrated")
    }

    async fn columns(&mut self, _db: &str, _table: &str) -> Result<Vec<ColumnInfo>> {
        bail!("Oracle driver not yet integrated")
    }

    async fn indexes(&mut self, _db: &str, _table: &str) -> Result<Vec<IndexInfo>> {
        bail!("Oracle driver not yet integrated")
    }

    async fn foreign_keys(&mut self, _db: &str, _table: &str) -> Result<Vec<ForeignKeyInfo>> {
        bail!("Oracle driver not yet integrated")
    }

    async fn ddl(&mut self, _db: &str, _table: &str) -> Result<DdlResult> {
        bail!("Oracle driver not yet integrated")
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
