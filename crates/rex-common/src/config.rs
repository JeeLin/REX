//! 可选配置文件读取。
//!
//! 数据目录下若存在配置文件（Hub: `config.yaml` / Agent: `agent.yaml`），则在启动时读取其中的
//! 键值并写入环境变量——**仅当对应环境变量尚未设置时**才写入，因此 env 变量始终优先于配置文件，
//! 且现有业务代码无需改动（仍通过 `std::env::var` 读取配置）。
//!
//! 文件不存在或解析失败时静默忽略，保持纯 env 变量的原有行为。

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::service::ServiceKind;

/// 配置文件中的可选字段（仅相关键被对应二进制使用）。
#[derive(Debug, Default, Deserialize)]
pub struct FileConfig {
    pub port: Option<u16>,
    pub data_dir: Option<PathBuf>,
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

/// 默认数据目录（无 REX_DATA_DIR 时）：
/// - Linux/macOS：`$HOME/.rex`
/// - Windows：`%LOCALAPPDATA%/rex`（无则当前目录下的 .rex）
/// - 其他平台：`.rex`
pub fn default_data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("rex"))
            .unwrap_or_else(|| PathBuf::from(".rex"))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".rex"))
            .unwrap_or_else(|| PathBuf::from(".rex"))
    }
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

/// 加载 `.env`：优先**可执行文件同目录**，不存在时回退「当前工作目录逐级向上」
/// （`dotenvy::dotenv()` 默认语义）。
///
/// exe 同目录优先保证 Windows 服务（CWD 常为 `system32`）、`--background` daemonize
/// 与跨目录启动都能读到二进制旁的 `.env`。文件不存在属正常情况，静默跳过；
/// 存在但读取/解析失败会在 stderr 打印 warning（main 阶段 tracing 尚未初始化）。
/// 不覆盖已设置的环境变量（dotenvy 语义）。
///
/// 应在 `main()` 尽早调用：supervisor 加载后 spawn 的 worker 继承其环境变量，
/// worker 自身的 `main()` 也会再执行一次（幂等）。
pub fn load_dotenv() {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    load_dotenv_from(exe_dir.as_deref());
}

/// 加载 `.env`：优先**可执行文件同目录**，未命中回退 CWD 逐级向上（[`cwd_fallback`]）。
fn load_dotenv_from(exe_dir: Option<&Path>) -> DotenvSource {
    load_dotenv_with(exe_dir, cwd_fallback)
}

/// 同 [`load_dotenv_from`]，但回退查找可注入：生产恒传 [`cwd_fallback`]，
/// 测试传临时目录 fixture，避免切换进程 CWD 与同进程其它测试并发冲突。
///
/// 返回值语义：任一段出现真实读取/解析错误 → [`DotenvSource::Failed`]（取首个错误），
/// 否则按命中来源返回。错误分支的 warning 打印与「继续回退」行为与不返回值时完全一致。
fn load_dotenv_with(
    exe_dir: Option<&Path>,
    fallback: impl FnOnce() -> Result<(), dotenvy::Error>,
) -> DotenvSource {
    let mut first_err = None;
    if let Some(dir) = exe_dir {
        let path = dir.join(".env");
        match dotenvy::from_path(&path) {
            Ok(()) => return DotenvSource::ExeDir(path),
            Err(e) if e.not_found() => {}
            Err(e) => {
                eprintln!("warning: failed to load {}: {e}", path.display());
                first_err = Some(e);
            }
        }
    }
    let cwd_result = fallback();
    if let Err(e) = &cwd_result {
        if !e.not_found() {
            eprintln!("warning: failed to load .env from working directory: {e}");
        }
    }
    if let Some(e) = first_err {
        return DotenvSource::Failed(e);
    }
    match cwd_result {
        Ok(()) => DotenvSource::CwdFallback,
        Err(e) if e.not_found() => DotenvSource::NotFound,
        Err(e) => DotenvSource::Failed(e),
    }
}

/// CWD 逐级向上查找并加载 `.env`（`dotenvy::dotenv()` 默认语义）。
fn cwd_fallback() -> Result<(), dotenvy::Error> {
    dotenvy::dotenv().map(|_| ())
}

/// `.env` 加载来源或首个真实错误，供测试断言与诊断。
///
/// [`load_dotenv`] 忽略该返回值，加载/回退/告警行为保持不变。
#[derive(Debug)]
pub enum DotenvSource {
    /// 命中 `<exe_dir>/.env`，路径为该文件
    ExeDir(PathBuf),
    /// exe 同目录未命中（或未提供），由 CWD 逐级向上回退命中
    CwdFallback,
    /// 两段查找都未找到 `.env`（正常情况，静默跳过）
    NotFound,
    /// 存在 `.env` 但读取/解析失败（warning 已打印到 stderr）
    Failed(dotenvy::Error),
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

    #[test]
    fn test_load_dotenv_prefers_exe_dir() {
        let exe_dir = tempfile::tempdir().unwrap();
        std::fs::write(exe_dir.path().join(".env"), "REX_TEST_DOTENV_EXE=exe-dir\n").unwrap();
        // 回退目录里放同名键的 fixture：exe 命中时回退不应被触发
        let cwd_dir = tempfile::tempdir().unwrap();
        std::fs::write(cwd_dir.path().join(".env"), "REX_TEST_DOTENV_EXE=cwd\n").unwrap();
        assert!(std::env::var("REX_TEST_DOTENV_EXE").is_err());

        let src = load_dotenv_with(Some(exe_dir.path()), || {
            dotenvy::from_path(cwd_dir.path().join(".env"))
        });

        match src {
            DotenvSource::ExeDir(path) => assert_eq!(path, exe_dir.path().join(".env")),
            other => panic!("expected ExeDir, got {other:?}"),
        }
        assert_eq!(
            std::env::var("REX_TEST_DOTENV_EXE").as_deref(),
            Ok("exe-dir")
        );
        std::env::remove_var("REX_TEST_DOTENV_EXE");
    }

    #[test]
    fn test_load_dotenv_missing_exe_dir_falls_back() {
        let exe_dir = tempfile::tempdir().unwrap(); // 无 .env
        let cwd_dir = tempfile::tempdir().unwrap();
        std::fs::write(
            cwd_dir.path().join(".env"),
            "REX_TEST_DOTENV_FALLBACK=cwd\n",
        )
        .unwrap();
        assert!(std::env::var("REX_TEST_DOTENV_FALLBACK").is_err());

        let src = load_dotenv_with(Some(exe_dir.path()), || {
            dotenvy::from_path(cwd_dir.path().join(".env"))
        });

        assert!(
            matches!(src, DotenvSource::CwdFallback),
            "exe 目录无 .env 必须回退到 CWD 查找，got {src:?}"
        );
        assert_eq!(
            std::env::var("REX_TEST_DOTENV_FALLBACK").as_deref(),
            Ok("cwd")
        );
        std::env::remove_var("REX_TEST_DOTENV_FALLBACK");
    }

    #[test]
    fn test_load_dotenv_none_exe_dir_uses_cwd_lookup() {
        let cwd_dir = tempfile::tempdir().unwrap();
        std::fs::write(cwd_dir.path().join(".env"), "REX_TEST_DOTENV_NONE=cwd\n").unwrap();
        assert!(std::env::var("REX_TEST_DOTENV_NONE").is_err());

        let hit = load_dotenv_with(None, || dotenvy::from_path(cwd_dir.path().join(".env")));
        assert!(
            matches!(hit, DotenvSource::CwdFallback),
            "无 exe 目录时必须走 CWD 查找并命中，got {hit:?}"
        );
        assert_eq!(std::env::var("REX_TEST_DOTENV_NONE").as_deref(), Ok("cwd"));

        let missing = load_dotenv_with(None, || {
            dotenvy::from_path(cwd_dir.path().join("no-such.env"))
        });
        assert!(
            matches!(missing, DotenvSource::NotFound),
            "CWD 查找未命中应返回 NotFound（静默），got {missing:?}"
        );
        std::env::remove_var("REX_TEST_DOTENV_NONE");
    }

    #[test]
    fn test_load_dotenv_broken_file_returns_failed_but_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".env"), "not a valid dotenv line ===\n").unwrap();

        let src = load_dotenv_from(Some(dir.path()));

        assert!(
            matches!(src, DotenvSource::Failed(_)),
            "解析失败必须落到 Failed 分支，got {src:?}"
        );
        assert!(std::env::var("REX_TEST_DOTENV_BROKEN").is_err());
    }
}
