//! Redis 协议抽象 — 统一 Redis 连接器。

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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
    String(String),
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
