/// 通用资源连接参数加载器
///
/// 所有协议共用此模块从 DB 读取资源记录并解密 config_json。
/// SSH 的 `load_resource_conn` (terminal_ws.rs) 是最早实现的版本，
/// 包含 SSH 特有字段（use_agent, agent_id, keepalive_interval）。
/// 本模块提供更通用的版本，适用于 MySQL/PostgreSQL/Redis/SFTP/SQLite/S3/SIP。
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

/// 从 `ResourceConnInfo` 解析 SIP 配置（`config_json` 内字段）。
///
/// SIP 协议特有字段：`server`、`port?`、`username`、`password`、`displayName?`、
/// `transport?`（udp/tcp/tls，默认 udp）。`password` 已在 `load_resource_config`
/// 中由 crypto 解密，此处直接读取明文。
///
/// 资源顶层 `host` 缺省时回退到 `config.server`；顶层 `port` 缺省时回退到
/// `config.port` 或默认 5060。
pub fn load_sip_conn(info: &ResourceConnInfo) -> Result<rex_sip::SipConfig, String> {
    let cfg = &info.config;
    let server = if !info.host.is_empty() {
        Some(info.host.clone())
    } else {
        cfg.get("server").and_then(|v| v.as_str()).map(String::from)
    }
    .ok_or_else(|| "sip: missing server".to_string())?;
    let port = info
        .port
        .or_else(|| cfg.get("port").and_then(|v| v.as_u64()).map(|p| p as u16))
        .unwrap_or(5060);
    let username = if !info.username.is_empty() {
        Some(info.username.clone())
    } else {
        cfg.get("username")
            .and_then(|v| v.as_str())
            .map(String::from)
    }
    .ok_or_else(|| "sip: missing username".to_string())?;
    let password = cfg
        .get("password")
        .and_then(|v| v.as_str())
        .map(String::from);
    let display_name = cfg
        .get("displayName")
        .and_then(|v| v.as_str())
        .map(String::from);
    let transport = match cfg
        .get("transport")
        .and_then(|v| v.as_str())
        .unwrap_or("udp")
    {
        "tcp" => rex_sip::SipTransport::Tcp,
        "tls" => rex_sip::SipTransport::Tls,
        _ => rex_sip::SipTransport::Udp,
    };
    Ok(rex_sip::SipConfig {
        server,
        port,
        username,
        password,
        display_name,
        transport,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info_with_config(config: &str) -> ResourceConnInfo {
        ResourceConnInfo {
            resource_id: "r1".into(),
            name: "sip".into(),
            protocol: "sip".into(),
            host: String::new(),
            port: None,
            username: String::new(),
            config: serde_json::from_str(config).unwrap(),
        }
    }

    #[test]
    fn load_sip_conn_parses_full_config() {
        let cfg = r#"{"server":"sip.example.com","port":5061,"username":"1000","password":"secret","displayName":"Alice","transport":"tls"}"#;
        let sip = load_sip_conn(&info_with_config(cfg)).unwrap();
        assert_eq!(sip.server, "sip.example.com");
        assert_eq!(sip.port, 5061);
        assert_eq!(sip.username, "1000");
        assert_eq!(sip.password.as_deref(), Some("secret"));
        assert_eq!(sip.display_name.as_deref(), Some("Alice"));
        assert_eq!(sip.transport, rex_sip::SipTransport::Tls);
    }

    #[test]
    fn load_sip_conn_defaults_port_and_transport() {
        let cfg = r#"{"server":"sip.x","username":"u","password":"p"}"#;
        let sip = load_sip_conn(&info_with_config(cfg)).unwrap();
        assert_eq!(sip.port, 5060);
        assert_eq!(sip.transport, rex_sip::SipTransport::Udp);
        assert!(sip.display_name.is_none());
    }

    #[test]
    fn load_sip_conn_anonymous_password_optional() {
        // 匿名注册（无 password）也是合法的 SIP 配置。
        let cfg = r#"{"server":"sip.x","username":"u"}"#;
        let sip = load_sip_conn(&info_with_config(cfg)).unwrap();
        assert!(sip.password.is_none());
    }

    #[test]
    fn load_sip_conn_falls_back_to_top_level_host() {
        let mut info = info_with_config(r#"{"username":"u","password":"p"}"#);
        info.host = "top.example.com".into();
        let sip = load_sip_conn(&info).unwrap();
        assert_eq!(sip.server, "top.example.com");
    }
}
