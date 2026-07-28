/// 通用资源连接参数加载器
///
/// 所有协议共用此模块从 DB 读取资源记录并解密 config_json。
/// SSH 的 `load_resource_conn` (terminal_ws.rs) 是最早实现的版本，
/// 包含 SSH 特有字段（use_agent, agent_id, keepalive_interval）。
/// 本模块提供更通用的版本，适用于 MySQL/PostgreSQL/Redis/SFTP/SQLite/S3。
use serde_json::Value as JsonValue;

use crate::app::AppState;

/// 从 DB 加载的资源连接信息（host/port/username + 解密后的 config_json）
///
/// 所有协议的 connect handler 通过此结构获取连接参数。
/// `host`/`port`/`username` 来自 Resource 顶层字段；
/// `config` 是解密后的 config_json，各协议从中提取特有参数：
/// - MySQL/PostgreSQL: `password`, `database_name`
/// - Redis: `password`, `db`
/// - SFTP: `password`, `private_key`
/// - SQLite: `file_path`
/// - S3: `endpoint`, `access_key`, `secret_key`, `bucket`, `region`
#[derive(Debug)]
pub struct ResourceConnInfo {
    pub resource_id: String,
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    /// 解密后的 config_json，各协议从中提取特有参数
    pub config: JsonValue,
}

/// 从 DB 读取资源连接信息（含 config_json 解密）
///
/// 所有协议共用此函数，确保连接参数从 DB 而非前端获取。
/// 前端仅传递 resource_id，后端负责读取和解密。
pub fn load_resource_config(
    state: &AppState,
    resource_id: &str,
) -> Result<ResourceConnInfo, String> {
    let resource = state
        .db
        .get_resource(resource_id)
        .map_err(|e| format!("db error: {e}"))?
        .ok_or_else(|| format!("resource not found: {resource_id}"))?;

    // 解密 config_json
    let config = if !resource.config_json.is_empty() && resource.config_json != "{}" {
        let decrypted = state
            .crypto
            .decrypt(&resource.config_json)
            .map_err(|e| format!("decrypt failed for resource {resource_id}: {e}"))?;
        serde_json::from_str(&decrypted)
            .map_err(|e| format!("invalid config json for resource {resource_id}: {e}"))?
    } else {
        JsonValue::Null
    };

    Ok(ResourceConnInfo {
        resource_id: resource.id,
        name: resource.name,
        protocol: resource.protocol,
        host: resource.host,
        port: resource.port,
        username: resource.username,
        config,
    })
}
