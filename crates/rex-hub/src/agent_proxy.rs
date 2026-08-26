//! Hub 侧协议会话代理（v0.70.6 子任务 #7）。
//!
//! 当资源处于 agent 模式时，Hub 不再直接连目标，而是用本模块的连接器把每个协议方法
//! 经单条 WS 隧道转发给 Agent，由 Agent 在私网内终结协议并把结果回传。前端 REST 接口
//! 与直连模式完全一致（连接池里存的是实现了同一 trait 的代理），满足「前端到 Agent 由
//! Hub 中转、协议在 Agent 终结、文件数据不经浏览器」的约束。

use anyhow::Result;
use async_trait::async_trait;
use rex_common::file_transfer::{FileConnector, FileEntry, UploadResult};
use rex_common::redis::{DbInfo, KeyInfo, RedisConnector, RedisInfo, RedisValue};
use rex_common::sql::{ColumnInfo, DatabaseType, QueryResult, SqlConnector, TableInfo};
use serde_json::{json, Value};

use crate::agent_ws::agent_session_request;
use crate::app::AppState;

/// 经隧道转发一次协议子请求并解析 `data` 为 `T`。
async fn relay<T: serde::de::DeserializeOwned>(
    state: &AppState,
    channel_id: &str,
    kind: &str,
    payload: Value,
) -> Result<T> {
    let data = agent_session_request(state, channel_id, kind, payload).await?;
    serde_json::from_value(data).map_err(|e| anyhow::anyhow!("invalid session response: {e}"))
}

fn decode_bytes(data: &Value) -> Result<Vec<u8>> {
    let b64 = data
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing data field"))?;
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| anyhow::anyhow!("base64 decode failed: {e}"))
}

// ═══════════════════════════════════════════════════════════════════════════
// SQL 代理
// ═══════════════════════════════════════════════════════════════════════════

pub struct AgentSqlProxy {
    state: AppState,
    channel_id: String,
    /// 已确定的子类（mysql/postgresql/sqlite）；agent 模式探测出后由调用方传入。
    /// 仅用于 `database_type()` 报告，连接持久化走 `take_session_subtype`。
    subtype: Option<String>,
}

impl AgentSqlProxy {
    pub fn new(state: AppState, channel_id: String, subtype: Option<String>) -> Self {
        Self {
            state,
            channel_id,
            subtype,
        }
    }
}

#[async_trait]
impl SqlConnector for AgentSqlProxy {
    fn database_type(&self) -> DatabaseType {
        match self.subtype.as_deref() {
            Some("postgresql") | Some("postgres") => DatabaseType::PostgreSQL,
            Some("sqlite") => DatabaseType::SQLite,
            _ => DatabaseType::MySQL,
        }
    }

    async fn execute(&mut self, sql: &str) -> Result<QueryResult> {
        relay(
            &self.state,
            &self.channel_id,
            "query",
            json!({ "sql": sql }),
        )
        .await
    }

    async fn databases(&mut self) -> Result<Vec<String>> {
        let v: Value =
            agent_session_request(&self.state, &self.channel_id, "databases", json!({})).await?;
        Ok(v.get("databases")
            .and_then(|d| d.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn tables(&mut self, db: &str) -> Result<Vec<TableInfo>> {
        let v: Value =
            agent_session_request(&self.state, &self.channel_id, "tables", json!({ "db": db }))
                .await?;
        serde_json::from_value(v.get("tables").cloned().unwrap_or(Value::Null))
            .map_err(|e| anyhow::anyhow!("invalid tables: {e}"))
    }

    async fn columns(&mut self, db: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let v: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "columns",
            json!({ "db": db, "table": table }),
        )
        .await?;
        serde_json::from_value(v.get("columns").cloned().unwrap_or(Value::Null))
            .map_err(|e| anyhow::anyhow!("invalid columns: {e}"))
    }

    async fn close(&mut self) -> Result<()> {
        let _ = agent_session_request(&self.state, &self.channel_id, "close", json!({})).await;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Redis 代理
// ═══════════════════════════════════════════════════════════════════════════

pub struct AgentRedisProxy {
    state: AppState,
    channel_id: String,
}

impl AgentRedisProxy {
    pub fn new(state: AppState, channel_id: String) -> Self {
        Self { state, channel_id }
    }
}

#[async_trait]
impl RedisConnector for AgentRedisProxy {
    async fn info(&mut self) -> Result<RedisInfo> {
        let v: Value =
            agent_session_request(&self.state, &self.channel_id, "info", json!({})).await?;
        serde_json::from_value(v.get("info").cloned().unwrap_or(Value::Null))
            .map_err(|e| anyhow::anyhow!("invalid info: {e}"))
    }

    async fn dbs(&mut self) -> Result<Vec<DbInfo>> {
        let v: Value =
            agent_session_request(&self.state, &self.channel_id, "dbs", json!({})).await?;
        serde_json::from_value(v.get("dbs").cloned().unwrap_or(Value::Null))
            .map_err(|e| anyhow::anyhow!("invalid dbs: {e}"))
    }

    async fn select_db(&mut self, db: i32) -> Result<()> {
        let _ = agent_session_request(
            &self.state,
            &self.channel_id,
            "select_db",
            json!({ "db": db }),
        )
        .await?;
        Ok(())
    }

    async fn scan(&mut self, pattern: &str, count: u32) -> Result<Vec<KeyInfo>> {
        let v: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "scan",
            json!({ "pattern": pattern, "count": count }),
        )
        .await?;
        serde_json::from_value(v.get("keys").cloned().unwrap_or(Value::Null))
            .map_err(|e| anyhow::anyhow!("invalid keys: {e}"))
    }

    async fn get_type(&mut self, key: &str) -> Result<String> {
        let v: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "get_type",
            json!({ "key": key }),
        )
        .await?;
        Ok(v.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("none")
            .to_string())
    }

    async fn get_value(&mut self, key: &str) -> Result<RedisValue> {
        let v: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "get_value",
            json!({ "key": key }),
        )
        .await?;
        serde_json::from_value(v.get("value").cloned().unwrap_or(Value::Null))
            .map_err(|e| anyhow::anyhow!("invalid value: {e}"))
    }

    async fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        let _ = agent_session_request(
            &self.state,
            &self.channel_id,
            "set_value",
            json!({ "key": key, "value": value }),
        )
        .await?;
        Ok(())
    }

    async fn del(&mut self, keys: &[String]) -> Result<u64> {
        let v: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "del",
            json!({ "keys": keys }),
        )
        .await?;
        Ok(v.get("deleted").and_then(|d| d.as_u64()).unwrap_or(0))
    }

    async fn ttl(&mut self, key: &str) -> Result<i64> {
        let v: Value =
            agent_session_request(&self.state, &self.channel_id, "ttl", json!({ "key": key }))
                .await?;
        Ok(v.get("ttl").and_then(|t| t.as_i64()).unwrap_or(-2))
    }

    async fn set_ttl(&mut self, key: &str, seconds: i64) -> Result<()> {
        let _ = agent_session_request(
            &self.state,
            &self.channel_id,
            "set_ttl",
            json!({ "key": key, "seconds": seconds }),
        )
        .await?;
        Ok(())
    }

    async fn command(&mut self, args: &[String]) -> Result<String> {
        let v: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "command",
            json!({ "args": args }),
        )
        .await?;
        Ok(v.get("output")
            .and_then(|o| o.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn close(&mut self) -> Result<()> {
        let _ = agent_session_request(&self.state, &self.channel_id, "close", json!({})).await;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 文件代理（SFTP / S3）
// ═══════════════════════════════════════════════════════════════════════════

pub struct AgentFileProxy {
    state: AppState,
    channel_id: String,
}

impl AgentFileProxy {
    pub fn new(state: AppState, channel_id: String) -> Self {
        Self { state, channel_id }
    }
}

#[async_trait]
impl FileConnector for AgentFileProxy {
    async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>> {
        let v: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "list",
            json!({ "path": path }),
        )
        .await?;
        serde_json::from_value(v.get("entries").cloned().unwrap_or(Value::Null))
            .map_err(|e| anyhow::anyhow!("invalid entries: {e}"))
    }

    async fn stat(&mut self, path: &str) -> Result<FileEntry> {
        let v: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "stat",
            json!({ "path": path }),
        )
        .await?;
        serde_json::from_value(v.get("entry").cloned().unwrap_or(Value::Null))
            .map_err(|e| anyhow::anyhow!("invalid entry: {e}"))
    }

    async fn upload(
        &mut self,
        remote_path: &str,
        data: Vec<u8>,
        offset: u64,
        _progress: Option<&rex_common::file_transfer::ProgressCallback>,
    ) -> Result<UploadResult> {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
        let _: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "upload",
            json!({ "path": remote_path, "offset": offset, "data": b64 }),
        )
        .await?;
        Ok(UploadResult { upload_id: None })
    }

    async fn download(&mut self, path: &str) -> Result<Vec<u8>> {
        let v: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "download",
            json!({ "path": path }),
        )
        .await?;
        decode_bytes(&v)
    }

    async fn download_range(
        &mut self,
        path: &str,
        offset: u64,
        limit: Option<u64>,
    ) -> Result<Vec<u8>> {
        let v: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "download",
            json!({ "path": path, "offset": offset, "limit": limit }),
        )
        .await?;
        decode_bytes(&v)
    }

    async fn delete(&mut self, path: &str) -> Result<()> {
        let _ = agent_session_request(
            &self.state,
            &self.channel_id,
            "delete",
            json!({ "path": path }),
        )
        .await?;
        Ok(())
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        let _ = agent_session_request(
            &self.state,
            &self.channel_id,
            "rename",
            json!({ "from": from, "to": to }),
        )
        .await?;
        Ok(())
    }

    async fn mkdir(&mut self, path: &str) -> Result<()> {
        let _ = agent_session_request(
            &self.state,
            &self.channel_id,
            "mkdir",
            json!({ "path": path }),
        )
        .await?;
        Ok(())
    }

    async fn read_for_edit(&mut self, path: &str) -> Result<Vec<u8>> {
        let v: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "read_for_edit",
            json!({ "path": path }),
        )
        .await?;
        decode_bytes(&v)
    }

    async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> Result<()> {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
        let _: Value = agent_session_request(
            &self.state,
            &self.channel_id,
            "save_from_edit",
            json!({ "path": path, "data": b64 }),
        )
        .await?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        let _ = agent_session_request(&self.state, &self.channel_id, "close", json!({})).await;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
