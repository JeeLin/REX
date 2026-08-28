//! 可选配置文件读取。
//!
//! 数据目录下若存在配置文件（Hub: `config.yaml` / Agent: `agent.yaml`），则在启动时读取其中的
//! 键值并写入环境变量——**仅当对应环境变量尚未设置时**才写入，因此 env 变量始终优先于配置文件，
//! 且现有业务代码无需改动（仍通过 `std::env::var` 读取配置）。
//!
//! 文件不存在或解析失败时静默忽略，保持纯 env 变量的原有行为。

use std::path::PathBuf;

use serde::Deserialize;

use crate::service::ServiceKind;

/// 配置文件中的可选字段（仅相关键被对应二进制使用）。
#[derive(Debug, Default, Deserialize)]
pub struct FileConfig {
    pub port: Option<u16>,
    pub data_dir: Option<PathBuf>,
    pub static_dir: Option<PathBuf>,
    pub hub_url: Option<String>,
    pub token: Option<String>,
}

/// 默认配置文件路径：`<data_dir>/config.yaml`（Hub）或 `<data_dir>/agent.yaml`（Agent）。
pub fn default_config_path(kind: ServiceKind) -> PathBuf {
    let data_dir = std::env::var("REX_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_data_dir());
    match kind {
        ServiceKind::Hub => data_dir.join("config.yaml"),
        ServiceKind::Agent => data_dir.join("agent.yaml"),
    }
}

fn default_data_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".rex"))
        .unwrap_or_else(|| PathBuf::from(".rex"))
}

/// 从配置文件加载并写入 env（env 已设置的键不被覆盖）。
///
/// 应在 supervisor 进程启动早期、spawn worker 之前调用：worker 会继承 supervisor 的环境变量。
/// 必须在创建 tokio runtime 之前调用（`std::env::set_var` 非线程安全）。
pub fn apply_config_env(kind: ServiceKind) {
    let path = default_config_path(kind);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(cfg) = serde_yaml::from_str::<FileConfig>(&text) else {
        tracing::warn!(path = %path.display(), "failed to parse config file, ignored");
        return;
    };
    set_if_unset("REX_PORT", cfg.port.map(|p| p.to_string()));
    set_if_unset(
        "REX_DATA_DIR",
        cfg.data_dir.as_ref().map(|p| p.display().to_string()),
    );
    set_if_unset(
        "REX_STATIC_DIR",
        cfg.static_dir.as_ref().map(|p| p.display().to_string()),
    );
    set_if_unset("REX_HUB_URL", cfg.hub_url);
    set_if_unset("REX_AGENT_TOKEN", cfg.token);
}

fn set_if_unset(key: &str, value: Option<String>) {
    if let Some(v) = value {
        if std::env::var(key).is_err() {
            std::env::set_var(key, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_parse_file_config() {
        let cfg: FileConfig =
            serde_yaml::from_str("port: 3000\ndata_dir: /tmp/x\nhub_url: wss://h\n").unwrap();
        assert_eq!(cfg.port, Some(3000));
        assert_eq!(cfg.data_dir.as_deref(), Some(Path::new("/tmp/x")));
        assert_eq!(cfg.hub_url.as_deref(), Some("wss://h"));
        assert!(cfg.token.is_none());
    }

    #[test]
    fn test_default_config_path() {
        std::env::set_var("REX_DATA_DIR", "/data/rex");
        assert_eq!(
            default_config_path(ServiceKind::Hub),
            PathBuf::from("/data/rex/config.yaml")
        );
        assert_eq!(
            default_config_path(ServiceKind::Agent),
            PathBuf::from("/data/rex/agent.yaml")
        );
        std::env::remove_var("REX_DATA_DIR");
    }
}
