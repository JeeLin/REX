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
mod http_server;
mod supervisor;
mod updater;

#[cfg(unix)]
use std::os::unix::io::IntoRawFd;
use std::path::PathBuf;

use rex_common::cli::{self, RunOpts, ServiceKind};

fn main() {
    let cli = cli::parse();
    let kind = ServiceKind::Agent;

    // Windows 服务模式：走 SCM 协议
    #[cfg(target_os = "windows")]
    {
        if matches!(&cli.command, Some(cli::Commands::Run(o)) if o.windows_service) {
            if let Err(e) = run_service_as_windows_service(&RunOpts::default()) {
                eprintln!("Error: {e:#}");
                std::process::exit(1);
            }
            return;
        }
    }

    if let Err(e) = cli::dispatch(cli, kind, run_service) {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

/// Windows SCM 服务入口。由 `--windows-service` 触发。
///
/// 调用链：main → StartServiceCtrlDispatcher → scm_service_main → 注册 handler → 上报 RUNNING → 实际业务
#[cfg(target_os = "windows")]
fn run_service_as_windows_service(_opts: &RunOpts) -> anyhow::Result<()> {
    use windows_sys::Win32::System::Services::*;

    // SERVICE_TABLE_ENTRY 需要静态生命周期
    let mut service_name: Vec<u16> = "rex-agent\0".encode_utf16().collect();
    let mut table = [
        SERVICE_TABLE_ENTRYW {
            lpServiceName: service_name.as_mut_ptr(),
            lpServiceProc: Some(scm_service_main),
        },
        SERVICE_TABLE_ENTRYW {
            lpServiceName: std::ptr::null_mut(),
            lpServiceProc: None,
        },
    ];

    // StartServiceCtrlDispatcher 会阻塞，直到服务主函数返回
    let ok = unsafe { StartServiceCtrlDispatcherW(table.as_ptr()) };
    if ok == 0 {
        // 失败（可能不是从 SCM 启动的），直接按普通模式运行
        let opts = RunOpts::default();
        run_service(&opts)?;
    }
    Ok(())
}

/// SCM 回调的服务主函数：注册 handler → 上报状态 → 执行业务 → 等待 shutdown
#[cfg(target_os = "windows")]
unsafe extern "system" fn scm_service_main(_argc: u32, _argv: *mut *mut u16) {
    use windows_sys::Win32::System::Services::*;

    // 1. 注册控制处理器
    let _handle = RegisterServiceCtrlHandlerExW(
        b"rex-agent\0".as_ptr().cast(),
        Some(scm_control_handler),
        std::ptr::null_mut(),
    );

    // 2. 上报 SERVICE_START_PENDING
    scm_report_status(SERVICE_START_PENDING, 0, 10000);

    // 3. 执行实际业务（在新线程中，因为 scm_service_main 本身要阻塞等 shutdown）
    let handle = std::thread::spawn(|| {
        scm_report_status(SERVICE_RUNNING, 0, 0);
        let opts = RunOpts::default();
        let _ = run_service(&opts);
    });

    // 4. 阻塞等待 shutdown 请求
    while !crate::supervisor::is_shutdown_requested() {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    // 5. 上报 SERVICE_STOP_PENDING
    scm_report_status(SERVICE_STOP_PENDING, 0, 30000);

    // 6. 等待业务线程退出（最多 30 秒）
    let _ = handle.join();

    // 7. 上报 SERVICE_STOPPED
    scm_report_status(SERVICE_STOPPED, 0, 0);
}

/// SCM 控制处理器：响应 Stop / Shutdown
#[cfg(target_os = "windows")]
unsafe extern "system" fn scm_control_handler(
    control: u32,
    _event_type: u32,
    _event_data: *mut core::ffi::c_void,
    _context: *mut core::ffi::c_void,
) -> u32 {
    use windows_sys::Win32::System::Services::*;

    match control {
        SERVICE_CONTROL_STOP | SERVICE_CONTROL_SHUTDOWN => {
            crate::supervisor::request_shutdown();
            0 // NO_ERROR
        }
        _ => 0,
    }
}

/// 上报服务状态给 SCM
#[cfg(target_os = "windows")]
fn scm_report_status(status: u32, exit_code: u32, wait_hint: u32) {
    use windows_sys::Win32::System::Services::*;

    static mut HANDLE: SERVICE_STATUS_HANDLE = std::ptr::null_mut();

    unsafe {
        if HANDLE.is_null() {
            HANDLE = RegisterServiceCtrlHandlerExW(
                b"rex-agent\0".as_ptr().cast(),
                Some(scm_control_handler),
                std::ptr::null_mut(),
            );
        }

        let service_status = SERVICE_STATUS {
            dwServiceType: SERVICE_WIN32_OWN_PROCESS,
            dwCurrentState: status,
            dwControlsAccepted: SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN,
            dwWin32ExitCode: exit_code,
            dwServiceSpecificExitCode: 0,
            dwCheckPoint: 0,
            dwWaitHint: wait_hint,
        };

        SetServiceStatus(HANDLE, &service_status);
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
        .unwrap_or_else(|_| rex_common::config::default_data_dir())
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
    let timer = tracing_subscriber::fmt::time::ChronoLocal::rfc_3339();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .with_timer(timer)
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
        // 创建 API 请求挂起映射
        let api_pending: agent_ws::ApiPendingMap =
            std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

        // 启动 Agent WebSocket 连接（内部会启动 HTTP server）
        agent_ws::run_agent(config, api_pending).await;
    });
}
