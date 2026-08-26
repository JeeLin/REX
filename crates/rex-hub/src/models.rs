//! 持久化模型：环境、资源、Agent、审计日志等数据库行的 Rust 结构定义，
//! 与 `migrations.sql` 中的表结构一一对应。

use serde::{Deserialize, Serialize};

// --- Environment ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub description: String,
    pub connection_mode: String,
    pub registration_token: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewEnvironment {
    pub name: String,
    pub description: Option<String>,
    pub connection_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateEnvironment {
    pub name: Option<String>,
    pub description: Option<String>,
    pub connection_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentDetail {
    #[serde(flatten)]
    pub environment: Environment,
    pub resource_count: i64,
    pub agent_status: Option<String>,
}

// --- Resource ---

#[derive(Debug, Clone, Deserialize)]
pub struct NewResource {
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub config_json: Option<String>,
    /// 资源子类型（子类）。合并里程碑（v0.70.7）引入：SQL 资源用其存探测出的方言
    /// （mysql/postgresql/sqlite）；其他资源类型暂未使用，为 None。通用可空列，
    /// 后续任何资源类型需要细分变体时都可复用，避免「db_type」这类协议专有字段的歧义。
    #[serde(default)]
    pub subtype: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub config_json: String,
    /// 资源子类型（子类）。合并里程碑（v0.70.7）引入：SQL 资源用其存探测出的方言
    /// （mysql/postgresql/sqlite）；其他资源类型暂未使用，为 None。通用可空列。
    #[serde(default)]
    pub subtype: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

// --- Agent ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub ip: String,
    pub status: String,
    pub last_seen_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// --- Audit ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub time: String,
    pub action: String,
    pub target: Option<String>,
    pub environment_id: Option<String>,
    pub resource_id: Option<String>,
    pub agent_id: Option<String>,
    pub result: String,
    pub detail: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NewAuditEntry {
    pub action: String,
    pub target: Option<String>,
    pub environment_id: Option<String>,
    pub resource_id: Option<String>,
    pub agent_id: Option<String>,
    pub result: String,
    pub detail: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub time_from: Option<String>,
    pub time_to: Option<String>,
    pub action: Option<String>,
    pub environment_id: Option<String>,
    pub agent_id: Option<String>,
    pub result: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditStats {
    pub total: i64,
    pub success_count: i64,
    pub failure_count: i64,
}

/// 命名 SQL 查询（用户保存的可复用查询片段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedQuery {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub sql: String,
    #[serde(default)]
    pub db_type: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- SIP CDR (Call Detail Record) ---

/// 通话记录（持久化到 SQLite，前端表格展示 + 关联录音/抓包）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdrRecord {
    pub id: String,
    pub resource_id: String,
    pub peer: String,
    pub call_id: String,
    #[serde(default)]
    pub start_time: String,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub duration_sec: i64,
    #[serde(default = "default_direction")]
    pub direction: String,
    #[serde(default = "default_cdr_state")]
    pub state: String,
    #[serde(default)]
    pub recording_url: String,
    #[serde(default)]
    pub pcap_url: String,
}

fn default_direction() -> String {
    "out".into()
}
fn default_cdr_state() -> String {
    "ended".into()
}

/// 新建/更新 CDR 的入参（Hub 通话状态机驱动写入）。
#[derive(Debug, Clone, Default)]
pub struct NewCdr {
    pub id: String,
    pub resource_id: String,
    pub peer: String,
    pub call_id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration_sec: i64,
    pub direction: String,
    pub state: String,
    pub recording_url: String,
    pub pcap_url: String,
}

/// CDR 列表查询过滤 + 分页 + 排序。
#[derive(Debug, Clone, Default)]
pub struct CdrFilter {
    pub resource_id: Option<String>,
    pub direction: Option<String>,
    pub state: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub sort: Option<String>, // start_desc | start_asc
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// 单条 SIP 抓包报文（前端回看/下载用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SipCaptureRecord {
    /// Unix 微秒时间戳。
    pub ts_us: u64,
    /// 方向：ua1_out / ua1_in / ua2_in。
    pub direction: String,
    /// 原始 SIP 报文文本（JSON 线格式）。
    pub raw: String,
}
