//! Redis 协议抽象 — 统一 Redis 连接器。

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 格式检测元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    /// 检测到的格式名（"text", "json", "msgpack", "php_serialize" 等）
    pub detected: String,
    /// 解码后的可读文本（高级格式有值）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoded: Option<String>,
    /// 压缩算法名（仅压缩格式有值）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<String>,
}

/// DB 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbInfo {
    pub index: i32,
    pub keys: u64,
    pub expires: u64,
}

/// 键信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    pub key: String,
    pub type_name: String,
}

/// Redis 值（按类型区分）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum RedisValue {
    String {
        value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<FormatInfo>,
    },
    List(Vec<String>),
    Set(Vec<String>),
    ZSet(Vec<(String, f64)>),
    Hash(Vec<(String, String)>),
}

/// Server INFO 摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisInfo {
    pub redis_version: String,
    pub os: String,
    pub process_id: String,
    pub connected_clients: String,
    pub used_memory: String,
    pub used_memory_peak: String,
    pub total_commands_processed: String,
    pub keyspace: Vec<KeyspaceInfo>,
}

/// 每个 DB 的 keyspace 统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyspaceInfo {
    pub db: String,
    pub keys: u64,
    pub expires: u64,
}

/// 连接请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConnectRequest {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub db: Option<i32>,
}

/// Redis 连接器 trait
#[async_trait]
pub trait RedisConnector: Send + Sync {
    /// 获取 Server INFO
    async fn info(&mut self) -> Result<RedisInfo>;

    /// 列出所有 DB（含键数）
    async fn dbs(&mut self) -> Result<Vec<DbInfo>>;

    /// 切换 DB
    async fn select_db(&mut self, db: i32) -> Result<()>;

    /// SCAN 遍历键（返回一批键）
    async fn scan(&mut self, pattern: &str, count: u32) -> Result<Vec<KeyInfo>>;

    /// 获取键的类型
    async fn get_type(&mut self, key: &str) -> Result<String>;

    /// 获取键值
    async fn get_value(&mut self, key: &str) -> Result<RedisValue>;

    /// 设置键值（String 类型）
    async fn set_value(&mut self, key: &str, value: &str) -> Result<()>;

    /// 删除键
    async fn del(&mut self, keys: &[String]) -> Result<u64>;

    /// 获取 TTL
    async fn ttl(&mut self, key: &str) -> Result<i64>;

    /// 设置 TTL（-1 = 永不过期，-2 = 删除）
    async fn set_ttl(&mut self, key: &str, seconds: i64) -> Result<()>;

    /// 执行任意命令（CLI 用）
    async fn command(&mut self, args: &[String]) -> Result<String>;

    /// 关闭连接
    async fn close(&mut self) -> Result<()>;
}

// ---------------------------------------------------------------------------
// 统一 Redis 操作分发
// ---------------------------------------------------------------------------

/// 统一 Redis 操作分发：按 kind 调用 connector 对应方法，返回 JSON。
/// Agent（WebSocket 隧道）和 Hub（HTTP handler）共用此函数。
pub async fn dispatch_redis(
    conn: &mut dyn RedisConnector,
    kind: &str,
    payload: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    match kind {
        "info" => {
            let info = conn.info().await?;
            Ok(serde_json::json!({ "info": info }))
        }
        "dbs" => {
            let dbs = conn.dbs().await?;
            Ok(serde_json::json!({ "dbs": dbs }))
        }
        "select_db" => {
            let db = payload.get("db").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            conn.select_db(db).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "scan" => {
            let pattern = payload
                .get("pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("*");
            let count = payload.get("count").and_then(|v| v.as_u64()).unwrap_or(100) as u32;
            let keys = conn.scan(pattern, count).await?;
            Ok(serde_json::json!({ "keys": keys }))
        }
        "get_type" => {
            let key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let t = conn.get_type(key).await?;
            Ok(serde_json::json!({ "type": t }))
        }
        "get_value" => {
            let key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let v = conn.get_value(key).await?;
            Ok(serde_json::json!({ "value": v }))
        }
        "set_value" => {
            let key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let val = payload.get("value").and_then(|v| v.as_str()).unwrap_or("");
            conn.set_value(key, val).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "del" => {
            let keys: Vec<String> = payload
                .get("keys")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let n = conn.del(&keys).await?;
            Ok(serde_json::json!({ "deleted": n }))
        }
        "ttl" => {
            let key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let ttl = conn.ttl(key).await?;
            Ok(serde_json::json!({ "ttl": ttl }))
        }
        "set_ttl" => {
            let key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let secs = payload.get("seconds").and_then(|v| v.as_i64()).unwrap_or(0);
            conn.set_ttl(key, secs).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "command" => {
            let args: Vec<String> = payload
                .get("args")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let out = conn.command(&args).await?;
            Ok(serde_json::json!({ "output": out }))
        }
        "close" => {
            let _ = conn.close().await;
            Ok(serde_json::json!({ "closed": true }))
        }
        other => anyhow::bail!("unsupported redis request kind: {other}"),
    }
}
