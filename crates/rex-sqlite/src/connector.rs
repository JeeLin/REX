use anyhow::Result;
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;

// ── 数据模型 ─────────────────────────────────────────────

/// SQLite 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    /// SQLite 数据库文件路径
    pub db_path: String,
    pub name: Option<String>,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            db_path: String::new(),
            name: None,
        }
    }
}

/// 列信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub cid: i32,
    pub name: String,
    pub r#type: String,
    pub notnull: bool,
    pub default_value: Option<String>,
    pub pk: bool,
}

/// 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub affected_rows: u64,
    pub elapsed_ms: u64,
}

// 状态
struct State {
    connection: Option<rusqlite::Connection>,
    connected: bool,
}

// ── SqliteConnector trait ─────────────────────────────────

#[async_trait]
pub trait SqliteConnector: Send + Sync {
    /// 连接到 SQLite 数据库
    async fn connect(&mut self) -> Result<()>;

    /// 执行 SQL 查询
    async fn execute(&self, sql: &str) -> Result<SqliteResult>;

    /// 列出所有表
    async fn list_tables(&self) -> Result<Vec<String>>;

    /// 获取表结构信息
    async fn get_table_info(&self, table: &str) -> Result<Vec<ColumnInfo>>;

    /// 关闭连接
    async fn close(&self) -> Result<()>;
}

// ── SqliteConnector 实现 ──────────────────────────────────

/// SQLite 连接器
pub struct SqliteConnectorImpl {
    config: SqliteConfig,
    state: Mutex<State>,
}

impl SqliteConnectorImpl {
    pub fn new(config: SqliteConfig) -> Self {
        Self {
            config,
            state: Mutex::new(State {
                connection: None,
                connected: false,
            }),
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let config: SqliteConfig = serde_json::from_str(json)?;
        Ok(Self::new(config))
    }

    pub fn config(&self) -> &SqliteConfig {
        &self.config
    }

    pub fn into_config(self) -> SqliteConfig {
        self.config
    }
}

#[async_trait]
impl SqliteConnector for SqliteConnectorImpl {
    async fn connect(&mut self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if state.connected {
            return Ok(());
        }
        let conn = rusqlite::Connection::open(&self.config.db_path)?;
        state.connection = Some(conn);
        state.connected = true;
        Ok(())
    }

    async fn execute(&self, sql: &str) -> Result<SqliteResult> {
        let start = Instant::now();
        let state = self.state.lock().unwrap();
        let conn = state
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        let mut stmt = conn.prepare(sql)?;
        let column_count = stmt.column_count();
        let columns = (0..column_count)
            .map(|i| stmt.column_name(i).unwrap().to_string())
            .collect::<Vec<String>>();

        let sql_upper = sql.trim().to_ascii_uppercase();
        let is_query = sql_upper.starts_with("SELECT")
            || sql_upper.starts_with("PRAGMA")
            || sql_upper.starts_with("WITH")
            || sql_upper.starts_with("EXPLAIN");

        let (rows, affected_rows) = if is_query {
            // Execute query and collect rows
            let rows_result = stmt.query_map([], |row| {
                Ok((0..column_count)
                    .map(|i| {
                        let value = match row.get_ref_unwrap(i) {
                            rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                            rusqlite::types::ValueRef::Integer(v) => serde_json::json!(v),
                            rusqlite::types::ValueRef::Real(v) => serde_json::json!(v),
                            rusqlite::types::ValueRef::Text(v) => {
                                serde_json::Value::String(String::from_utf8_lossy(v).into_owned())
                            }
                            rusqlite::types::ValueRef::Blob(v) => {
                                serde_json::Value::String(general_purpose::STANDARD.encode(v))
                            }
                        };
                        value
                    })
                    .collect::<Vec<serde_json::Value>>())
            })?;
            let rows: Vec<Vec<serde_json::Value>> =
                rows_result.collect::<std::result::Result<_, _>>()?;
            (rows, 0u64)
        } else {
            // Execute command (INSERT, UPDATE, DELETE, etc.) and get changes
            let changes = stmt.execute([])?;
            (Vec::new(), changes as u64)
        };

        let elapsed = start.elapsed();
        Ok(SqliteResult {
            columns,
            rows,
            affected_rows,
            elapsed_ms: elapsed.as_millis() as u64,
        })
    }

    async fn list_tables(&self) -> Result<Vec<String>> {
        let state = self.state.lock().unwrap();
        let conn = state
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut tables = Vec::new();
        for row in rows {
            tables.push(row?);
        }
        Ok(tables)
    }

    async fn get_table_info(&self, table: &str) -> Result<Vec<ColumnInfo>> {
        // Validate table name to prevent SQL injection (PRAGMA doesn't support parameterized queries)
        if !table.chars().all(|c| c.is_alphanumeric() || c == '_') || table.is_empty() {
            anyhow::bail!("invalid table name: {}", table);
        }

        let state = self.state.lock().unwrap();
        let conn = state
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        let pragma_sql = format!("PRAGMA table_info({})", table);
        let mut stmt = conn.prepare(&pragma_sql)?;
        let rows = stmt.query_map([], |row| {
            let raw_default = row.get::<_, Option<String>>(4)?;
            let default_value = raw_default.map(|s| {
                if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
                    s[1..s.len() - 1].to_string()
                } else {
                    s
                }
            });
            Ok(ColumnInfo {
                cid: row.get::<_, i32>(0)?,
                name: row.get::<_, String>(1)?,
                r#type: row.get::<_, String>(2)?,
                notnull: row.get::<_, bool>(3)?,
                default_value,
                pk: row.get::<_, i32>(5)? != 0,
            })
        })?;
        let mut info = Vec::new();
        for row in rows {
            info.push(row?);
        }
        Ok(info)
    }

    async fn close(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if let Some(conn) = state.connection.take() {
            drop(conn); // 显式关闭连接
        }
        state.connected = false;
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn sqlite_config_default() {
        let config = SqliteConfig::default();
        assert!(config.db_path.is_empty());
        assert!(config.name.is_none());
    }

    #[test]
    fn sqlite_config_deserializes() {
        let json = r#"{"db_path":"/data/app.db","name":"app-db"}"#;
        let config: SqliteConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.db_path, "/data/app.db");
        assert_eq!(config.name, Some("app-db".into()));
    }

    #[test]
    fn sqlite_config_optional_fields() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let config: SqliteConfig = serde_json::from_str(json).unwrap();
        assert!(config.name.is_none());
    }

    #[test]
    fn sqlite_connector_from_json() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let connector = SqliteConnectorImpl::from_json(json).unwrap();
        assert_eq!(connector.config().db_path, "/data/app.db");
        let state = connector.state.lock().unwrap();
        assert!(!state.connected);
    }

    #[test]
    fn sqlite_connector_is_object_safe() {
        fn _assert_object_safe(_: &dyn SqliteConnector) {}
    }

    #[tokio::test]
    async fn sqlite_connect_sets_connected() {
        let temp_file = NamedTempFile::new().unwrap();
        let json = format!(r#"{{"db_path":"{}"}}"#, temp_file.path().to_string_lossy());
        let mut connector = SqliteConnectorImpl::from_json(&json).unwrap();
        let state = connector.state.lock().unwrap();
        assert!(!state.connected);
        drop(state);
        connector.connect().await.unwrap();
        let state = connector.state.lock().unwrap();
        assert!(state.connected);
        assert!(state.connection.is_some());
    }

    #[tokio::test]
    async fn sqlite_execute_fails_when_not_connected() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let connector = SqliteConnectorImpl::from_json(json).unwrap();
        let result = connector.execute("SELECT 1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn sqlite_list_tables_fails_when_not_connected() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let connector = SqliteConnectorImpl::from_json(json).unwrap();
        let result = connector.list_tables().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn sqlite_get_table_info_fails_when_not_connected() {
        let json = r#"{"db_path":"/data/app.db"}"#;
        let connector = SqliteConnectorImpl::from_json(json).unwrap();
        let result = connector.get_table_info("users").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn sqlite_execute_returns_empty_when_connected() {
        let temp_file = NamedTempFile::new().unwrap();
        let json = format!(r#"{{"db_path":"{}"}}"#, temp_file.path().to_string_lossy());
        let mut connector = SqliteConnectorImpl::from_json(&json).unwrap();
        connector.connect().await.unwrap();
        let result = connector.execute("SELECT 1").await.unwrap();
        assert_eq!(result.columns, vec!["1".to_string()]);
        assert_eq!(result.rows, vec![vec![serde_json::json!(1)]]);
        assert_eq!(result.affected_rows, 0);
    }

    #[tokio::test]
    async fn sqlite_list_tables_returns_empty_when_connected() {
        let temp_file = NamedTempFile::new().unwrap();
        let json = format!(r#"{{"db_path":"{}"}}"#, temp_file.path().to_string_lossy());
        let mut connector = SqliteConnectorImpl::from_json(&json).unwrap();
        connector.connect().await.unwrap();
        let tables = connector.list_tables().await.unwrap();
        assert!(tables.is_empty());
    }

    #[tokio::test]
    async fn sqlite_create_and_query_table() {
        let temp_file = NamedTempFile::new().unwrap();
        let json = format!(r#"{{"db_path":"{}"}}"#, temp_file.path().to_string_lossy());
        let mut connector = SqliteConnectorImpl::from_json(&json).unwrap();
        connector.connect().await.unwrap();

        // 创建表
        connector
            .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();

        // 插入数据
        connector
            .execute("INSERT INTO users (name) VALUES ('Alice')")
            .await
            .unwrap();
        connector
            .execute("INSERT INTO users (name) VALUES ('Bob')")
            .await
            .unwrap();

        // 查询数据
        let result = connector
            .execute("SELECT id, name FROM users")
            .await
            .unwrap();
        assert_eq!(result.columns, vec!["id", "name"]);
        assert_eq!(result.rows.len(), 2);
        assert_eq!(
            result.rows[0],
            vec![serde_json::json!(1), serde_json::json!("Alice")]
        );
        assert_eq!(
            result.rows[1],
            vec![serde_json::json!(2), serde_json::json!("Bob")]
        );
        assert_eq!(result.affected_rows, 0);
    }

    #[tokio::test]
    async fn sqlite_list_tables_returns_created_table() {
        let temp_file = NamedTempFile::new().unwrap();
        let json = format!(r#"{{"db_path":"{}"}}"#, temp_file.path().to_string_lossy());
        let mut connector = SqliteConnectorImpl::from_json(&json).unwrap();
        connector.connect().await.unwrap();

        // 创建表
        connector
            .execute("CREATE TABLE test_table (id INTEGER)")
            .await
            .unwrap();

        let tables = connector.list_tables().await.unwrap();
        assert_eq!(tables, vec!["test_table"]);
    }

    #[tokio::test]
    async fn sqlite_get_table_info_returns_correct_info() {
        let temp_file = NamedTempFile::new().unwrap();
        let json = format!(r#"{{"db_path":"{}"}}"#, temp_file.path().to_string_lossy());
        let mut connector = SqliteConnectorImpl::from_json(&json).unwrap();
        connector.connect().await.unwrap();

        // 创建表
        connector
            .execute("CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT NOT NULL DEFAULT 'unknown')")
            .await
            .unwrap();

        let info = connector.get_table_info("test_table").await.unwrap();
        assert_eq!(info.len(), 2);
        // id 列
        assert_eq!(info[0].cid, 0);
        assert_eq!(info[0].name, "id");
        assert_eq!(info[0].r#type, "INTEGER");
        assert!(!info[0].notnull); // INTEGER PRIMARY KEY 默认允许 NULL
        assert_eq!(info[0].default_value, None);
        assert!(info[0].pk);
        // name 列
        assert_eq!(info[1].cid, 1);
        assert_eq!(info[1].name, "name");
        assert_eq!(info[1].r#type, "TEXT");
        assert!(info[1].notnull); // NOT NULL 约束
        assert_eq!(info[1].default_value, Some("unknown".to_string()));
        assert!(!info[1].pk);
    }

    #[tokio::test]
    async fn sqlite_close_succeeds() {
        let temp_file = NamedTempFile::new().unwrap();
        let json = format!(r#"{{"db_path":"{}"}}"#, temp_file.path().to_string_lossy());
        let mut connector = SqliteConnectorImpl::from_json(&json).unwrap();
        let state = connector.state.lock().unwrap();
        assert!(!state.connected);
        drop(state);
        connector.connect().await.unwrap();
        let state = connector.state.lock().unwrap();
        assert!(state.connected);
        drop(state);
        connector.close().await.unwrap();
        let state = connector.state.lock().unwrap();
        assert!(!state.connected);
        assert!(state.connection.is_none());
    }

    #[test]
    fn sqlite_result_serializes() {
        let result = SqliteResult {
            columns: vec!["id".into(), "name".into()],
            rows: vec![
                vec![serde_json::json!(1), serde_json::json!("Alice")],
                vec![serde_json::json!(2), serde_json::json!("Bob")],
            ],
            affected_rows: 0,
            elapsed_ms: 1,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("Alice"));
    }

    #[test]
    fn column_info_serializes() {
        let col = ColumnInfo {
            cid: 0,
            name: "id".into(),
            r#type: "INTEGER".into(),
            notnull: true,
            default_value: None,
            pk: true,
        };
        let json = serde_json::to_string(&col).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("INTEGER"));
    }
}
