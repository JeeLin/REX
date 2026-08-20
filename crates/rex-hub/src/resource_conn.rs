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

/// 从 `ResourceConnInfo` 解析 SIP 配置。
///
/// `config_json` 为 `SipProfile` 形状（`{ accounts[], activeAccount }`）：
/// 选取 `activeAccount` 对应账户（不存在则回退 `accounts[0]`），该账户自带
/// `server`/`port`/`transport` 与登录凭据，直接构造生效的 [`rex_sip::SipConfig`]。
///
/// 资源顶层 `host`/`port` 不再作为 server 来源（server 已下沉到账户层）；
/// 仅当账户 `server` 缺省时回退资源顶层 `host`（兼容直连场景下的缺省写法）。
/// `password` 已在 `load_resource_config` 中由 crypto 解密，此处直接读取明文。
pub fn load_sip_conn(info: &ResourceConnInfo) -> Result<rex_sip::SipConfig, String> {
    let cfg = &info.config;
    let profile: rex_sip::SipProfile =
        serde_json::from_value(cfg.clone()).map_err(|e| format!("sip: invalid profile: {e}"))?;
    let active = profile
        .accounts
        .iter()
        .find(|a| a.id == profile.active_account)
        .or_else(|| profile.accounts.first())
        .ok_or_else(|| "sip: no account available".to_string())?;

    // server 优先取账户自带；缺省时回退资源顶层 host。
    let server = if !active.server.is_empty() {
        active.server.clone()
    } else {
        info.host.clone()
    };
    if server.is_empty() {
        return Err("sip: missing server".to_string());
    }
    // 端口取账户自带（`SipAccount` serde 默认 5060；字段缺省时生效）。
    // 字段显式传 0 不会被 default 覆盖，故此处显式拒绝非法端口。
    // 资源顶层 port 对 SIP 不生效——server/port 已完全下沉到账户层。
    let port = active.port;
    if port == 0 {
        return Err("sip: invalid port (must be > 0)".to_string());
    }
    if active.username.is_empty() {
        return Err("sip: missing username".to_string());
    }

    Ok(rex_sip::SipConfig {
        server,
        port,
        username: active.username.clone(),
        password: active.password.clone(),
        display_name: active.display_name.clone(),
        transport: active.transport,
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
    fn load_sip_conn_resolves_active_account_from_profile() {
        let cfg = r#"{
            "accounts":[
                {"id":"a1","server":"pbx.example.com","port":5061,"transport":"tcp","username":"alice","password":"pa","displayName":"Alice"},
                {"id":"a2","server":"pbx2.example.com","port":5062,"transport":"tls","username":"bob","password":"pb","displayName":"Bob"}
            ],
            "activeAccount":"a2"
        }"#;
        let sip = load_sip_conn(&info_with_config(cfg)).unwrap();
        // 生效账户应为 a2，且 server/port/transport 取自身携带值。
        assert_eq!(sip.server, "pbx2.example.com");
        assert_eq!(sip.port, 5062);
        assert_eq!(sip.transport, rex_sip::SipTransport::Tls);
        assert_eq!(sip.username, "bob");
        assert_eq!(sip.password.as_deref(), Some("pb"));
        assert_eq!(sip.display_name.as_deref(), Some("Bob"));
    }

    #[test]
    fn load_sip_conn_active_account_fallback_to_first() {
        let cfg = r#"{
            "accounts":[
                {"id":"a1","server":"pbx.example.com","username":"alice","password":"pa"},
                {"id":"a2","server":"pbx2.example.com","username":"bob","password":"pb"}
            ],
            "activeAccount":"does-not-exist"
        }"#;
        let sip = load_sip_conn(&info_with_config(cfg)).unwrap();
        // activeAccount 不存在 → 回退 accounts[0]
        assert_eq!(sip.username, "alice");
        assert_eq!(sip.server, "pbx.example.com");
        assert_eq!(sip.port, 5060); // 默认端口
        assert_eq!(sip.transport, rex_sip::SipTransport::Udp);
    }

    #[test]
    fn load_sip_conn_account_server_defaults_to_top_level_host() {
        // 账户 server 缺省时回退资源顶层 host。
        let mut info = info_with_config(
            r#"{"accounts":[{"id":"a1","username":"alice"}],"activeAccount":"a1"}"#,
        );
        info.host = "top.example.com".into();
        let sip = load_sip_conn(&info).unwrap();
        assert_eq!(sip.server, "top.example.com");
        assert_eq!(sip.username, "alice");
    }

    #[test]
    fn load_sip_conn_anonymous_password_optional() {
        // 匿名注册（无 password）也是合法的 SIP 配置。
        let cfg =
            r#"{"accounts":[{"id":"a1","server":"sip.x","username":"u"}],"activeAccount":"a1"}"#;
        let sip = load_sip_conn(&info_with_config(cfg)).unwrap();
        assert!(sip.password.is_none());
        assert_eq!(sip.transport, rex_sip::SipTransport::Udp);
    }

    #[test]
    fn load_sip_conn_missing_account_is_error() {
        let cfg = r#"{"accounts":[],"activeAccount":"a1"}"#;
        let res = load_sip_conn(&info_with_config(cfg));
        assert!(res.is_err());
    }

    #[test]
    fn load_sip_conn_missing_username_is_error() {
        let cfg = r#"{"accounts":[{"id":"a1","server":"sip.x"}],"activeAccount":"a1"}"#;
        let res = load_sip_conn(&info_with_config(cfg));
        assert!(res.is_err());
    }
}
