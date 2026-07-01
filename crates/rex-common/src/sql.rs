use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── SQL 查询结果列 ──────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlColumn {
    pub name: String,
    pub data_type: String,
}

// ── SQL 查询结果 ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlResult {
    pub columns: Vec<SqlColumn>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub affected_rows: u64,
    pub elapsed_ms: u64,
}

// ── 数据库信息 ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub name: String,
}

// ── 表信息 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub row_count: Option<u64>,
}

// ── 列信息 ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
}

// ── SqlConnector trait ──────────────────────────────────

#[async_trait]
pub trait SqlConnector: Send + Sync {
    /// 连接数据库
    async fn connect(&mut self) -> Result<()>;

    /// 执行 SQL 查询
    async fn execute(&self, sql: &str) -> Result<SqlResult>;

    /// 列出所有数据库
    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>>;

    /// 列出指定数据库中的所有表
    async fn list_tables(&self, database: &str) -> Result<Vec<TableInfo>>;

    /// 列出指定表的列信息
    async fn list_columns(&self, database: &str, table: &str) -> Result<Vec<ColumnInfo>>;

    /// 获取 SQL 执行计划（EXPLAIN）
    async fn explain(&self, sql: &str) -> Result<ExplainResult>;

    /// 关闭连接
    async fn close(&self) -> Result<()>;
}

// ── 执行计划结果 ────────────────────────────────────────

/// 通用的 SQL 执行计划结果，使用列式结构适配不同方言
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainResult {
    /// 列名（如 id, select_type, table, type, key, rows, extra 等）
    pub columns: Vec<String>,
    /// 行数据（每行为 Vec<serde_json::Value>）
    pub rows: Vec<Vec<serde_json::Value>>,
    /// 原始文本输出（作为 fallback 展示）
    pub raw_output: String,
}

// ── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_column_serializes() {
        let col = SqlColumn {
            name: "id".into(),
            data_type: "INT".into(),
        };
        let json = serde_json::to_string(&col).unwrap();
        assert!(json.contains("\"name\":\"id\""));
        assert!(json.contains("\"data_type\":\"INT\""));
    }

    #[test]
    fn sql_result_roundtrips() {
        let result = SqlResult {
            columns: vec![SqlColumn {
                name: "id".into(),
                data_type: "INT".into(),
            }],
            rows: vec![vec![serde_json::json!(1), serde_json::json!("hello")]],
            affected_rows: 0,
            elapsed_ms: 5,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: SqlResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.columns.len(), 1);
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.affected_rows, 0);
        assert_eq!(parsed.elapsed_ms, 5);
    }

    #[test]
    fn database_info_roundtrips() {
        let info = DatabaseInfo {
            name: "mydb".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: DatabaseInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, info);
    }

    #[test]
    fn table_info_roundtrips() {
        let info = TableInfo {
            name: "users".into(),
            row_count: Some(42),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: TableInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "users");
        assert_eq!(parsed.row_count, Some(42));
    }

    #[test]
    fn column_info_roundtrips() {
        let info = ColumnInfo {
            name: "id".into(),
            data_type: "INT".into(),
            is_nullable: false,
            is_primary_key: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: ColumnInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, info);
    }

    #[test]
    fn sql_connector_trait_is_object_safe() {
        fn _assert_object_safe(_: &dyn SqlConnector) {}
    }

    #[test]
    fn explain_result_roundtrips() {
        let result = ExplainResult {
            columns: vec!["id".into(), "select_type".into(), "table".into(), "rows".into()],
            rows: vec![
                vec![serde_json::json!(1), serde_json::json!("SIMPLE"), serde_json::json!("users"), serde_json::json!(100)],
            ],
            raw_output: "id\tselect_type\ttable\trows\n1\tSIMPLE\tusers\t100".into(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ExplainResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.columns.len(), 4);
        assert_eq!(parsed.rows.len(), 1);
        assert!(parsed.raw_output.contains("SIMPLE"));
    }
}
