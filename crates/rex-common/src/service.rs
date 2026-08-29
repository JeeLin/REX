//! 操作系统服务注册与生命周期管理（开机自启）。
//!
//! 支持 Linux (systemd) 与 macOS (launchd)。
//! 单元文件的「生成逻辑」（`render_*`）为纯函数，便于单测；
//! 「执行外部命令」（`systemctl` / `launchctl`）的部分仅在 unix 上真实调用，
//! 其他平台（如 Windows）直接返回明确的不支持错误与带外指引。

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::Args;
use clap::Subcommand;

/// 二进制种类，决定服务名、launchd label 与相关环境变量。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceKind {
    Hub,
    Agent,
}

impl ServiceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ServiceKind::Hub => "hub",
            ServiceKind::Agent => "agent",
        }
    }

    pub fn default_name(self) -> String {
        format!("rex-{}", self.as_str())
    }

    pub fn launchd_label(self) -> String {
        format!("com.rex.{}", self.as_str())
    }

    /// 安装为服务时需要从当前进程继承的相关环境变量。
    pub fn relevant_env_keys(self) -> &'static [&'static str] {
        match self {
            ServiceKind::Hub => &["REX_PORT", "REX_DATA_DIR", "REX_STATIC_DIR"],
            ServiceKind::Agent => &["REX_HUB_URL", "REX_AGENT_TOKEN"],
        }
    }
}

/// 服务安装范围：用户级（无需 root）或系统级（需 root）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceScope {
    User,
    System,
}

/// 当前运行平台，决定使用哪套服务管理机制。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServicePlatform {
    Linux,
    Macos,
    Other,
}

pub fn detect_platform() -> ServicePlatform {
    if cfg!(target_os = "linux") {
        ServicePlatform::Linux
    } else if cfg!(target_os = "macos") {
        ServicePlatform::Macos
    } else {
        ServicePlatform::Other
    }
}

/// 安装所需配置（exe 路径在 `handle` 中解析）。
pub struct InstallConfig {
    pub kind: ServiceKind,
    pub name: String,
    pub scope: ServiceScope,
    pub exe: PathBuf,
}

/// 服务子命令的共享选项。
#[derive(Args, Debug, Clone, Default)]
pub struct ServiceOpts {
    /// 系统级（默认用户级）；仅 *nix 生效
    #[arg(long)]
    pub system: bool,
    /// 服务名（默认 rex-hub / rex-agent）
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ServiceAction {
    /// 安装为系统服务并启用开机自启
    Install(ServiceOpts),
    /// 卸载系统服务
    Uninstall(ServiceOpts),
    /// 启动服务
    Start(ServiceOpts),
    /// 停止服务
    Stop(ServiceOpts),
    /// 重启服务
    Restart(ServiceOpts),
    /// 查看服务状态
    Status(ServiceOpts),
}

/// 服务子命令分组：`rex-hub service <action>`。
#[derive(Args, Debug, Clone)]
pub struct ServiceCmd {
    #[command(subcommand)]
    pub action: ServiceAction,
}

/// 收集当前进程中非空的相关环境变量。
pub fn collect_env(kind: ServiceKind) -> Vec<(String, String)> {
    kind.relevant_env_keys()
        .iter()
        .filter_map(|k| std::env::var(*k).ok().map(|v| (k.to_string(), v)))
        .collect()
}

/// CLI 入口：把 `ServiceCmd` 分发到对应平台实现。
pub fn handle(kind: ServiceKind, cmd: ServiceCmd) -> Result<()> {
    match cmd.action {
        ServiceAction::Install(o) => {
            let cfg = build_install_config(kind, &o)?;
            println!("{}", install(&cfg)?);
        }
        ServiceAction::Uninstall(o) => {
            let (name, scope) = resolve(&o, kind);
            println!("{}", uninstall(kind, &name, scope)?);
        }
        ServiceAction::Start(o) => {
            let (name, scope) = resolve(&o, kind);
            println!("{}", start(kind, &name, scope)?);
        }
        ServiceAction::Stop(o) => {
            let (name, scope) = resolve(&o, kind);
            println!("{}", stop(kind, &name, scope)?);
        }
        ServiceAction::Restart(o) => {
            let (name, scope) = resolve(&o, kind);
            println!("{}", restart(kind, &name, scope)?);
        }
        ServiceAction::Status(o) => {
            let (name, scope) = resolve(&o, kind);
            println!("{}", status(kind, &name, scope));
        }
    }
    Ok(())
}

fn resolve(opts: &ServiceOpts, kind: ServiceKind) -> (String, ServiceScope) {
    let scope = if opts.system {
        ServiceScope::System
    } else {
        ServiceScope::User
    };
    let name = opts.name.clone().unwrap_or_else(|| kind.default_name());
    (name, scope)
}

fn build_install_config(kind: ServiceKind, opts: &ServiceOpts) -> Result<InstallConfig> {
    let (name, scope) = resolve(opts, kind);
    let exe = std::env::current_exe().context("failed to resolve current executable path")?;
    Ok(InstallConfig {
        kind,
        name,
        scope,
        exe,
    })
}

// ───────────────────────────── 单元文件生成（纯函数） ─────────────────────────────

fn systemd_unit_path(name: &str, scope: ServiceScope) -> PathBuf {
    match scope {
        ServiceScope::System => {
            PathBuf::from("/etc/systemd/system").join(format!("{name}.service"))
        }
        ServiceScope::User => {
            let base = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    std::env::var("HOME")
                        .map(PathBuf::from)
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(".config")
                });
            base.join("systemd/user").join(format!("{name}.service"))
        }
    }
}

/// 生成 systemd 单元文件内容（纯函数，便于单测）。
pub fn render_systemd_unit(cfg: &InstallConfig, env: &[(String, String)]) -> String {
    let env_block = if env.is_empty() {
        String::new()
    } else {
        let lines: Vec<String> = env
            .iter()
            .map(|(k, v)| format!("Environment={k}={v}"))
            .collect();
        format!("{}\n", lines.join("\n"))
    };
    let wanted = match cfg.scope {
        ServiceScope::System => "multi-user.target",
        ServiceScope::User => "default.target",
    };
    format!(
        "[Unit]\n\
         Description=REX {kind}\n\
         After=network.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         ExecStart={exe}\n\
         {env_block}\
         Restart=on-failure\n\
         RestartSec=2\n\
         \n\
         [Install]\n\
         WantedBy={wanted}\n",
        kind = cfg.kind.as_str(),
        exe = cfg.exe.display(),
        env_block = env_block,
        wanted = wanted,
    )
}

fn launchd_plist_path(label: &str, scope: ServiceScope) -> PathBuf {
    match scope {
        ServiceScope::System => Path::new("/Library/LaunchDaemons").join(format!("{label}.plist")),
        ServiceScope::User => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            Path::new(&home)
                .join("Library/LaunchAgents")
                .join(format!("{label}.plist"))
        }
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// 生成 launchd plist 内容（纯函数，便于单测）。
pub fn render_launchd_plist(cfg: &InstallConfig, env: &[(String, String)]) -> String {
    let label = cfg.kind.launchd_label();
    let env_block = if env.is_empty() {
        String::new()
    } else {
        let lines: Vec<String> = env
            .iter()
            .map(|(k, v)| format!("    <key>{k}</key>\n    <string>{}</string>", xml_escape(v)))
            .collect();
        format!("{}\n", lines.join("\n"))
    };
    let log = format!(
        "{}/Library/Logs/rex-{}.log",
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
        cfg.kind.as_str()
    );
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \x20\x20<key>Label</key>\n\x20\x20<string>{label}</string>\n\
         \x20\x20<key>ProgramArguments</key>\n\x20\x20<array>\n\x20\x20\x20\x20<string>{exe}</string>\n\x20\x20</array>\n\
         \x20\x20<key>EnvironmentVariables</key>\n\x20\x20<dict>\n{env_block}\x20\x20</dict>\n\
         \x20\x20<key>RunAtLoad</key>\n\x20\x20<true/>\n\
         \x20\x20<key>KeepAlive</key>\n\x20\x20<true/>\n\
         \x20\x20<key>StandardOutPath</key>\n\x20\x20<string>{log}</string>\n\
         \x20\x20<key>StandardErrorPath</key>\n\x20\x20<string>{log}</string>\n\
         </dict>\n\
         </plist>\n",
        label = label,
        exe = xml_escape(&cfg.exe.display().to_string()),
        env_block = env_block,
        log = xml_escape(&log),
    )
}

// ───────────────────────────── 平台无关分发 ─────────────────────────────

pub fn install(cfg: &InstallConfig) -> Result<String> {
    match detect_platform() {
        ServicePlatform::Linux => systemd_install(cfg),
        ServicePlatform::Macos => launchd_install(cfg),
        ServicePlatform::Other => unsupported(),
    }
}

pub fn uninstall(kind: ServiceKind, name: &str, scope: ServiceScope) -> Result<String> {
    match detect_platform() {
        ServicePlatform::Linux => systemd_uninstall(name, scope),
        ServicePlatform::Macos => launchd_uninstall(&kind.launchd_label(), scope),
        ServicePlatform::Other => unsupported(),
    }
}

pub fn start(kind: ServiceKind, name: &str, scope: ServiceScope) -> Result<String> {
    match detect_platform() {
        ServicePlatform::Linux => systemd_run(scope, "start", name),
        ServicePlatform::Macos => launchd_run(&kind.launchd_label(), "start", scope),
        ServicePlatform::Other => unsupported(),
    }
}

pub fn stop(kind: ServiceKind, name: &str, scope: ServiceScope) -> Result<String> {
    match detect_platform() {
        ServicePlatform::Linux => systemd_run(scope, "stop", name),
        ServicePlatform::Macos => launchd_run(&kind.launchd_label(), "stop", scope),
        ServicePlatform::Other => unsupported(),
    }
}

pub fn restart(kind: ServiceKind, name: &str, scope: ServiceScope) -> Result<String> {
    match detect_platform() {
        ServicePlatform::Linux => systemd_run(scope, "restart", name),
        ServicePlatform::Macos => launchd_run(&kind.launchd_label(), "restart", scope),
        ServicePlatform::Other => unsupported(),
    }
}

/// 查询状态：不崩溃，返回人类可读字符串。
pub fn status(kind: ServiceKind, name: &str, scope: ServiceScope) -> String {
    match detect_platform() {
        ServicePlatform::Linux => systemd_status(name, scope),
        ServicePlatform::Macos => launchd_status(&kind.launchd_label()),
        ServicePlatform::Other => {
            "service management is not supported on this platform".to_string()
        }
    }
}

fn unsupported() -> Result<String> {
    bail!(
        "automatic service management is not supported on this platform.\n\
         On Windows, register the binary with nssm or Task Scheduler.\n\
         The binary runs supervisor + worker when started directly."
    )
}

// ───────────────────────────── *nix 实际执行 ─────────────────────────────

#[cfg(unix)]
fn systemd_install(cfg: &InstallConfig) -> Result<String> {
    let path = systemd_unit_path(&cfg.name, cfg.scope);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir {}", parent.display()))?;
    }
    let env = collect_env(cfg.kind);
    std::fs::write(&path, render_systemd_unit(cfg, &env))
        .with_context(|| format!("write unit file {}", path.display()))?;

    let user = user_flag(cfg.scope);
    run_systemctl(&["daemon-reload"], user)?;
    run_systemctl(&["enable", &format!("{}.service", cfg.name)], user)?;
    run_systemctl(&["start", &format!("{}.service", cfg.name)], user)?;
    Ok(format!(
        "installed and started systemd service: {}",
        path.display()
    ))
}

#[cfg(unix)]
fn systemd_uninstall(name: &str, scope: ServiceScope) -> Result<String> {
    let user = user_flag(scope);
    // best-effort：服务可能未运行
    let _ = run_systemctl(&["disable", "--now", &format!("{name}.service")], user);
    let path = systemd_unit_path(name, scope);
    remove_file_if_exists(&path);
    Ok(format!(
        "uninstalled systemd service (removed {})",
        path.display()
    ))
}

#[cfg(unix)]
fn systemd_run(scope: ServiceScope, action: &str, name: &str) -> Result<String> {
    let user = user_flag(scope);
    run_systemctl(&[action, &format!("{name}.service")], user)?;
    Ok(format!("service {name} {action}"))
}

#[cfg(unix)]
fn systemd_status(name: &str, scope: ServiceScope) -> String {
    let user = match scope {
        ServiceScope::System => "--system",
        ServiceScope::User => "--user",
    };
    match Command::new("systemctl")
        .arg(user)
        .arg("is-active")
        .arg(format!("{name}.service"))
        .output()
    {
        Ok(o) if o.status.success() => format!("{name}: active"),
        Ok(o) => format!("{name}: {}", String::from_utf8_lossy(&o.stdout).trim()),
        Err(e) => format!("{name}: unable to query ({e})"),
    }
}

#[cfg(unix)]
fn launchd_install(cfg: &InstallConfig) -> Result<String> {
    let label = cfg.kind.launchd_label();
    let path = launchd_plist_path(&label, cfg.scope);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir {}", parent.display()))?;
    }
    let env = collect_env(cfg.kind);
    std::fs::write(&path, render_launchd_plist(cfg, &env))
        .with_context(|| format!("write plist {}", path.display()))?;
    let status = Command::new("launchctl")
        .arg("load")
        .arg(&path)
        .status()
        .context("failed to run launchctl load")?;
    if !status.success() {
        bail!("launchctl load failed ({status})");
    }
    Ok(format!(
        "installed and loaded launchd service: {path}",
        path = path.display()
    ))
}

#[cfg(unix)]
fn launchd_uninstall(label: &str, scope: ServiceScope) -> Result<String> {
    let path = launchd_plist_path(label, scope);
    let _ = Command::new("launchctl").arg("unload").arg(&path).status();
    remove_file_if_exists(&path);
    Ok(format!(
        "uninstalled launchd service (removed {path})",
        path = path.display()
    ))
}

#[cfg(unix)]
fn launchd_run(label: &str, action: &str, scope: ServiceScope) -> Result<String> {
    let path = launchd_plist_path(label, scope);
    let status = match action {
        "restart" => Command::new("launchctl")
            .arg("unload")
            .arg(&path)
            .status()
            .and_then(|_| Command::new("launchctl").arg("load").arg(&path).status()),
        other => Command::new("launchctl").arg(other).arg(label).status(),
    }
    .context("failed to run launchctl")?;
    if !status.success() {
        bail!("launchctl {action} failed ({status})");
    }
    Ok(format!("service {label} {action}"))
}

#[cfg(unix)]
fn launchd_status(label: &str) -> String {
    match Command::new("launchctl").arg("list").output() {
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stdout);
            if out.lines().any(|l| l.contains(label)) {
                format!("{label}: loaded")
            } else {
                format!("{label}: not loaded")
            }
        }
        Err(e) => format!("{label}: unable to query ({e})"),
    }
}

#[cfg(unix)]
fn user_flag(scope: ServiceScope) -> Option<&'static str> {
    match scope {
        ServiceScope::System => None,
        ServiceScope::User => Some("--user"),
    }
}

#[cfg(unix)]
fn run_systemctl(args: &[&str], user: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("systemctl");
    if let Some(u) = user {
        cmd.arg(u);
    }
    cmd.args(args);
    let status = cmd.status().context("failed to run systemctl")?;
    if !status.success() {
        bail!("systemctl {:?} failed ({status})", args);
    }
    Ok(())
}

fn remove_file_if_exists(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}

// ───────────────────────────── 非 unix 平台桩实现 ─────────────────────────────
// 上面「按 detect_platform 分发」的 match 在全部平台都必须可编译；非 unix（如 Windows）
// 上 detect_platform() 返回 Other，这些分支运行时不会被命中，仍需存在以满足编译。
// 真实执行仅由上方 #[cfg(unix)] 的实现提供。

#[cfg(not(unix))]
fn systemd_install(_cfg: &InstallConfig) -> Result<String> {
    unsupported()
}

#[cfg(not(unix))]
fn systemd_uninstall(_name: &str, _scope: ServiceScope) -> Result<String> {
    unsupported()
}

#[cfg(not(unix))]
fn systemd_run(_scope: ServiceScope, _action: &str, _name: &str) -> Result<String> {
    unsupported()
}

#[cfg(not(unix))]
fn systemd_status(_name: &str, _scope: ServiceScope) -> String {
    "service management is not supported on this platform".to_string()
}

#[cfg(not(unix))]
fn launchd_install(_cfg: &InstallConfig) -> Result<String> {
    unsupported()
}

#[cfg(not(unix))]
fn launchd_uninstall(_label: &str, _scope: ServiceScope) -> Result<String> {
    unsupported()
}

#[cfg(not(unix))]
fn launchd_run(_label: &str, _action: &str, _scope: ServiceScope) -> Result<String> {
    unsupported()
}

#[cfg(not(unix))]
fn launchd_status(_label: &str) -> String {
    "service management is not supported on this platform".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(kind: ServiceKind, scope: ServiceScope) -> InstallConfig {
        InstallConfig {
            kind,
            name: kind.default_name(),
            scope,
            exe: PathBuf::from("/usr/local/bin/rex-hub"),
        }
    }

    #[test]
    fn test_detect_platform_not_other() {
        // 本仓库 CI 仅跑 Linux/macOS；断言不会误报 Other。
        assert_ne!(detect_platform(), ServicePlatform::Other);
    }

    #[test]
    fn test_render_systemd_unit_system_scope() {
        let unit = render_systemd_unit(&cfg(ServiceKind::Hub, ServiceScope::System), &[]);
        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("ExecStart=/usr/local/bin/rex-hub"));
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("WantedBy=multi-user.target"));
        assert!(!unit.contains("default.target"));
    }

    #[test]
    fn test_render_systemd_unit_user_scope() {
        let unit = render_systemd_unit(&cfg(ServiceKind::Agent, ServiceScope::User), &[]);
        assert!(unit.contains("WantedBy=default.target"));
        assert!(!unit.contains("multi-user.target"));
    }

    #[test]
    fn test_render_systemd_unit_env_injection() {
        let env = vec![
            ("REX_PORT".to_string(), "3000".to_string()),
            ("REX_DATA_DIR".to_string(), "/home/u/.rex".to_string()),
        ];
        let unit = render_systemd_unit(&cfg(ServiceKind::Hub, ServiceScope::System), &env);
        assert!(unit.contains("Environment=REX_PORT=3000"));
        assert!(unit.contains("Environment=REX_DATA_DIR=/home/u/.rex"));
    }

    #[test]
    fn test_render_launchd_plist_structure() {
        let env = vec![("REX_PORT".to_string(), "3000".to_string())];
        let plist = render_launchd_plist(&cfg(ServiceKind::Hub, ServiceScope::User), &env);
        assert!(plist.contains("<?xml"));
        assert!(plist.contains("<plist version=\"1.0\">"));
        assert!(plist.contains("<dict>"));
        assert!(plist.ends_with("</plist>\n"));
        assert!(plist.contains("<key>Label</key>"));
        assert!(plist.contains("<string>com.rex.hub</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<true/>"));
        assert!(plist.contains("<key>KeepAlive</key>"));
        assert!(plist.contains("<key>EnvironmentVariables</key>"));
        assert!(plist.contains("REX_PORT"));
        // 标签配对：<dict> 与 </dict> 数量一致，<plist> 与 </plist> 数量一致
        assert_eq!(
            plist.matches("<dict>").count(),
            plist.matches("</dict>").count()
        );
        assert_eq!(
            plist.matches("<plist").count(),
            plist.matches("</plist>").count()
        );
        assert_eq!(
            plist.matches("<array>").count(),
            plist.matches("</array>").count()
        );
    }

    #[test]
    fn test_render_launchd_plist_xml_escape() {
        let env = vec![("REX_HUB_URL".to_string(), "wss://h&a<b>.com".to_string())];
        let plist = render_launchd_plist(&cfg(ServiceKind::Agent, ServiceScope::User), &env);
        assert!(plist.contains("wss://h&amp;a&lt;b&gt;.com"));
    }

    #[test]
    fn test_service_kind_names() {
        assert_eq!(ServiceKind::Hub.default_name(), "rex-hub");
        assert_eq!(ServiceKind::Agent.default_name(), "rex-agent");
        assert_eq!(ServiceKind::Hub.launchd_label(), "com.rex.hub");
        assert_eq!(
            ServiceKind::Hub.relevant_env_keys(),
            &["REX_PORT", "REX_DATA_DIR", "REX_STATIC_DIR"]
        );
    }
}
