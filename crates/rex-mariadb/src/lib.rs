//! MariaDB 协议实现 — 基于 MySQL 驱动的 SqlConnector（MariaDB 与 MySQL 协议兼容）。

use anyhow::Result;
use rex_common::sql::{
    ColumnInfo, ConnectRequest, DdlResult, ForeignKeyInfo, IndexInfo, QueryResult, SqlConnector,
    TableInfo,
};

/// MariaDB 连接器（复用 MySQL 驱动）
pub struct MariaDBConnector {
    inner: rex_mysql::MySqlConnector,
}

impl MariaDBConnector {
    /// 建立 MariaDB 连接（使用 MySQL 协议）
    pub async fn connect(req: ConnectRequest) -> Result<Self> {
        let inner = rex_mysql::MySqlConnector::connect(req).await?;
        Ok(Self { inner })
    }
}

#[async_trait::async_trait]
impl SqlConnector for MariaDBConnector {
    fn database_type(&self) -> rex_common::sql::DatabaseType {
        rex_common::sql::DatabaseType::MariaDB
    }

    async fn execute(&mut self, sql: &str) -> Result<QueryResult> {
        self.inner.execute(sql).await
    }

    async fn databases(&mut self) -> Result<Vec<String>> {
        self.inner.databases().await
    }

    async fn tables(&mut self, db: &str) -> Result<Vec<TableInfo>> {
        self.inner.tables(db).await
    }

    async fn columns(&mut self, db: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        self.inner.columns(db, table).await
    }

    async fn indexes(&mut self, db: &str, table: &str) -> Result<Vec<IndexInfo>> {
        self.inner.indexes(db, table).await
    }

    async fn foreign_keys(&mut self, db: &str, table: &str) -> Result<Vec<ForeignKeyInfo>> {
        self.inner.foreign_keys(db, table).await
    }

    async fn ddl(&mut self, db: &str, table: &str) -> Result<DdlResult> {
        self.inner.ddl(db, table).await
    }

    async fn close(&mut self) -> Result<()> {
        self.inner.close().await
    }
}
