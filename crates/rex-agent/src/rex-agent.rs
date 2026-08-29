//! REX Agent 入口 — supervisor + worker 进程模型。
//!
//! PID 1 = supervisor（启动 worker、监控退出、处理更新替换）
//! Worker = 实际业务逻辑（WebSocket 连接、资源代理）
//!
//! REX_WORKER=1 → 运行 worker
//! 否则 → 运行 supervisor

mod agent_file;
mod agent_redis;
mod agent_sql;
mod agent_ssh;
mod agent_ws;
mod supervisor;
mod updater;

#[cfg(unix)]
use std::os::unix::io::IntoRawFd;
use std::path::PathBuf;

use rex_common::cli::{self, RunOpts, ServiceKind};

fn main() {
    let cli = cli::parse();
    let kind = ServiceKind::Agent;
    if let Err(e) = cli::dispatch(cli, kind, run_service) {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

/// 启动逻辑：`run` 子命令（及无子命令默认）。
///
/// 1. 读取可选配置文件（env 优先）；2. 单实例互斥（pid 文件，一个环境只能有一个 agent）；
/// 3. 把命令行参数写入 env（worker / supervisor 子进程继承）；
/// 4. `--single` 直接跑 worker（无 supervisor，无法自动更新，禁用更新检查）；
/// 5. `--background` 脱离终端后台运行。
fn run_service(opts: &RunOpts) -> anyhow::Result<()> {
    // 配置文件（env 优先）— 必须在读任何 env 之前
    rex_common::config::apply_config_env(ServiceKind::Agent);

    // 解析 data_dir（可能被配置文件设置），供单实例/pid 逻辑与后续复用
    let data_dir = data_dir_or_default();

    // 单实例互斥：同一 data_dir 只允许一个 Agent（一个环境只能有一个 agent）
    rex_common::process::ensure_single_instance(ServiceKind::Agent, &data_dir)?;

    // 命令行参数 > env：把相关字段写回 env，供 worker / supervisor 子进程继承
    if let Some(hub_url) = &opts.hub_url {
        std::env::set_var("REX_HUB_URL", hub_url);
    }
    if let Some(token) = &opts.token {
        std::env::set_var("REX_AGENT_TOKEN", token);
    }
    if let Some(data_dir) = &opts.data_dir {
        std::env::set_var("REX_DATA_DIR", data_dir);
    }

    // 后台模式：脱离终端（daemonize），日志重定向到数据目录 rex-agent.log
    #[cfg(unix)]
    if opts.background {
        let log_path = data_dir.join("rex-agent.log");
        redirect_stdio(&log_path)?;
        rex_common::process::daemonize()?;
    }

    // 写 pid 文件（前台 / 后台主进程）
    rex_common::process::write_pid_file(ServiceKind::Agent, &data_dir)?;

    // 单进程模式：直接 worker，无 supervisor → 无法自动更新，禁用更新检查
    if opts.single {
        tracing::warn!(status = "single-process mode; auto-update disabled (no supervisor)");
        std::env::set_var("REX_AUTO_UPDATE", "false");
        worker_main();
        return Ok(());
    }

    if std::env::var("REX_WORKER").is_ok() {
        worker_main();
    } else {
        crate::supervisor::run_supervisor();
    }
    Ok(())
}

fn data_dir_or_default() -> PathBuf {
    std::env::var("REX_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_data_dir())
}

/// 默认数据目录（无 REX_DATA_DIR 时）：
/// - Linux/macOS：`$HOME/.rex`
/// - Windows：`%LOCALAPPDATA%/rex`（无则当前目录下的 .rex）
/// - 其他平台：`.rex`
fn default_data_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("rex"))
            .unwrap_or_else(|| PathBuf::from(".rex"))
    } else {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".rex"))
            .unwrap_or_else(|| PathBuf::from(".rex"))
    }
}

/// 把 stdout / stderr 重定向到日志文件（后台模式用）。
#[cfg(unix)]
fn redirect_stdio(log_path: &std::path::Path) -> anyhow::Result<()> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| anyhow::anyhow!("open log file {}: {e}", log_path.display()))?;
    // into_raw_fd 把文件 fd 的所有权移出 File，关闭 File 不会关闭该 fd；
    // dup2 把其复制到 stdout/stderr，原 fd 随后必须关闭，避免泄漏。
    let fd = file.into_raw_fd();
    unsafe {
        libc::dup2(fd, libc::STDOUT_FILENO);
        libc::dup2(fd, libc::STDERR_FILENO);
        libc::close(fd);
    }
    Ok(())
}

fn worker_main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    tracing::info!(
        name = "REX Agent",
        version = env!("CARGO_PKG_VERSION"),
        status = "worker starting"
    );

    let config = match agent_ws::AgentConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to load agent config");
            eprintln!("Error: {e}");
            eprintln!("Required environment variables: REX_HUB_URL, REX_AGENT_TOKEN");
            std::process::exit(1);
        }
    };

    tracing::info!(
        hub_url = %config.hub_url,
        auto_update = config.auto_update,
        "agent configured"
    );

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        agent_ws::run_agent(config).await;
    });
}
