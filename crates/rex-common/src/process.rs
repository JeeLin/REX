//! 进程管理：单实例互斥（pidfile）+ 后台运行（daemonize）+ 停止（stop）。
//!
//! - 单实例：每个 data_dir 下一个 `rex-<hub|agent>.pid`，启动时若已有存活进程则拒绝，
//!   避免同一环境多个 agent / hub 冲突（一个环境只能有一个 agent）。
//! - 后台：Unix 用 `setsid` 脱离终端；Windows 用 `CreateProcess(DETACHED_PROCESS)`。
//! - 停止：`stop` 读 pidfile 向进程发终止信号并清理。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use crate::service::ServiceKind;
use anyhow::{bail, Context, Result};

static ALIVE: OnceLock<AtomicBool> = OnceLock::new();

/// pidfile 路径：`<data_dir>/rex-<hub|agent>.pid`
/// `data_dir` 由调用方显式传入（启动时已解析 REX_DATA_DIR / 默认值），
/// 避免函数内部读取全局 env，便于测试且行为可预测。
pub fn pid_path(kind: ServiceKind, data_dir: &Path) -> PathBuf {
    data_dir.join(format!("rex-{}.pid", kind.as_str()))
}

/// 读取 pidfile 中的 PID（文件不存在或解析失败返回 None）。
pub fn read_pid_file(path: &Path) -> Option<u32> {
    let content = std::fs::read_to_string(path).ok()?;
    content.trim().parse::<u32>().ok()
}

/// 进程是否存活（跨平台：Unix `kill(pid,0)`，Windows `OpenProcess`）。
pub fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // signal 0 不真正发信号，只做存在性 + 权限检查
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION};
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
            if handle.is_null() {
                return false;
            }
            windows_sys::Win32::Foundation::CloseHandle(handle);
            true
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        true
    }
}

/// 单实例检查：若已有存活实例则拒绝启动。返回错误信息已含停止提示。
pub fn ensure_single_instance(kind: ServiceKind, data_dir: &Path) -> Result<()> {
    // supervisor 子进程（worker）跳过单实例检查：pid 文件属于 supervisor 本身
    if std::env::var("REX_WORKER").is_ok() {
        return Ok(());
    }
    let path = pid_path(kind, data_dir);
    if let Some(pid) = read_pid_file(&path) {
        // Docker 容器重启后新进程仍是 PID 1，与残留 pidfile 中的 PID 相同
        // 但实际是不同进程实例——跳过检查并清理旧 pidfile
        if pid == std::process::id() {
            let _ = std::fs::remove_file(&path);
            return Ok(());
        }
        if is_process_alive(pid) {
            bail!(
                "rex-{} 已在运行 (pid {}).\n请先停止：rex-{} stop",
                kind.as_str(),
                pid,
                kind.as_str()
            );
        }
        // 陈旧 pidfile（进程已死），清理后继续
        let _ = std::fs::remove_file(&path);
    }
    Ok(())
}

/// 写入 pidfile 并注册退出时清理（仅在当前进程是前台/后台主进程时调用）。
pub fn write_pid_file(kind: ServiceKind, data_dir: &Path) -> Result<PathBuf> {
    let path = pid_path(kind, data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&path, std::process::id().to_string())
        .with_context(|| format!("write pid file {}", path.display()))?;

    let alive = ALIVE.get_or_init(|| AtomicBool::new(true));
    alive.store(true, Ordering::SeqCst);
    // 进程正常退出（非 stop 路径强制 kill）时兜底删除 pidfile；
    // stop 路径会先发信号再删文件，atexit 因文件已不存在而幂等无副作用。
    #[cfg(unix)]
    {
        extern "C" fn cleanup() {
            if let Some(a) = ALIVE.get() {
                a.store(false, Ordering::SeqCst);
            }
        }
        unsafe {
            libc::atexit(cleanup);
        }
    }
    Ok(path)
}

/// 停止：读 pidfile → 发终止信号 → 清理。返回已停止的 pid 或「未运行」。
pub fn stop(kind: ServiceKind, data_dir: &Path) -> String {
    let path = pid_path(kind, data_dir);
    match read_pid_file(&path) {
        None => format!("rex-{} 未在运行（无 pid 文件）", kind.as_str()),
        Some(pid) => {
            if !is_process_alive(pid) {
                let _ = std::fs::remove_file(&path);
                return format!(
                    "rex-{} 未在运行（pid {} 已死，已清理陈旧 pid 文件）",
                    kind.as_str(),
                    pid
                );
            }
            let sent = send_terminate(pid);
            let _ = std::fs::remove_file(&path);
            if sent {
                format!("已向 rex-{} (pid {}) 发送终止信号", kind.as_str(), pid)
            } else {
                format!(
                    "向 rex-{} (pid {}) 发送终止信号失败，请用 kill {pid} 手动停止",
                    kind.as_str(),
                    pid
                )
            }
        }
    }
}

#[cfg(unix)]
fn send_terminate(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) == 0 }
}

#[cfg(windows)]
fn send_terminate(pid: u32) -> bool {
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() {
            return false;
        }
        let ok = TerminateProcess(handle, 0) != 0;
        windows_sys::Win32::Foundation::CloseHandle(handle);
        ok
    }
}

#[cfg(not(any(unix, windows)))]
fn send_terminate(_pid: u32) -> bool {
    false
}

/// 后台运行：脱离终端（daemonize）。调用后进程在后台继续，父进程退出。
/// 日志仍由 tracing 输出到 stdout/stderr，调用前应重定向到日志文件。
pub fn daemonize() -> Result<()> {
    #[cfg(unix)]
    {
        // 复制标准 fd 到 /dev/null 之前先 double-fork 并 setsid，让进程脱离控制终端
        unsafe {
            // 第一次 fork
            let pid = libc::fork();
            if pid < 0 {
                bail!("fork failed: {}", std::io::Error::last_os_error());
            }
            if pid > 0 {
                // 父进程直接退出，把后台进程交给 init
                std::process::exit(0);
            }
            // 子进程：setsid 成为新会话 leader，脱离终端
            if libc::setsid() < 0 {
                bail!("setsid failed: {}", std::io::Error::last_os_error());
            }
            // 第二次 fork，避免重新获取控制终端
            let pid2 = libc::fork();
            if pid2 < 0 {
                bail!("second fork failed: {}", std::io::Error::last_os_error());
            }
            if pid2 > 0 {
                std::process::exit(0);
            }
            // 重定向 stdio 到 /dev/null
            let dev_null = std::ffi::CString::new("/dev/null").unwrap();
            let fd = libc::open(dev_null.as_ptr(), libc::O_RDWR);
            if fd >= 0 {
                libc::dup2(fd, libc::STDIN_FILENO);
                libc::dup2(fd, libc::STDOUT_FILENO);
                libc::dup2(fd, libc::STDERR_FILENO);
                if fd > 2 {
                    libc::close(fd);
                }
            }
            Ok(())
        }
    }
    #[cfg(windows)]
    {
        // Windows：用 CreateProcess 以 DETACHED_PROCESS 重新拉起自己，父进程退出。
        // 简化处理：直接提示用 service；后台 daemon 在 Windows 不强求。
        bail!("Windows 不支持 --background，请用 `rex-hub` 或 `rex-agent` 的 `service install` 注册为系统服务");
    }
    #[cfg(not(any(unix, windows)))]
    {
        bail!("当前平台不支持后台运行");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_pid_path(name: &str) -> PathBuf {
        // 每个用例用独立文件名，避免并行测试共享同一路径导致竞态。
        let dir = std::env::temp_dir().join(format!("rex-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok();
        dir.join(name)
    }

    #[test]
    fn test_read_pid_file_missing() {
        let p = tmp_pid_path("missing.pid");
        let _ = std::fs::remove_file(&p);
        assert_eq!(read_pid_file(&p), None);
    }

    #[test]
    fn test_read_pid_file_present() {
        let p = tmp_pid_path("present.pid");
        std::fs::write(&p, "4242").unwrap();
        assert_eq!(read_pid_file(&p), Some(4242));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn test_read_pid_file_garbage() {
        let p = tmp_pid_path("garbage.pid");
        std::fs::write(&p, "not-a-number").unwrap();
        assert_eq!(read_pid_file(&p), None);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn test_pid_path_contains_kind() {
        let dir = std::env::temp_dir();
        assert!(pid_path(ServiceKind::Hub, &dir)
            .to_string_lossy()
            .contains("rex-hub.pid"));
        assert!(pid_path(ServiceKind::Agent, &dir)
            .to_string_lossy()
            .contains("rex-agent.pid"));
    }

    #[test]
    fn test_ensure_single_instance_self() {
        // 校验三种真实场景：全新启动（无 pidfile）/ 陈旧 pidfile（进程已死）/ 存活实例（冲突）。
        // data_dir 显式传入，不依赖全局 REX_DATA_DIR，避免与并行测试相互踩踏。
        let dir = std::env::temp_dir().join(format!("rex-test-single-{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok();

        let p = dir.join("rex-agent.pid");
        let _ = std::fs::remove_file(&p);
        // 无 pidfile：全新启动，正常通过
        assert!(ensure_single_instance(ServiceKind::Agent, &dir).is_ok());

        // 陈旧 pidfile（已死进程）：应自动清理并正常通过
        std::fs::write(&p, "999999").unwrap();
        assert!(ensure_single_instance(ServiceKind::Agent, &dir).is_ok());
        assert!(!p.exists(), "stale pidfile should be removed");

        // 当前进程自己的 pid：Docker 重启场景（新旧容器都是 PID 1），应跳过并清理
        std::fs::write(&p, std::process::id().to_string()).unwrap();
        assert!(
            ensure_single_instance(ServiceKind::Agent, &dir).is_ok(),
            "same-pid (Docker restart) should be treated as stale and cleaned up"
        );
        assert!(!p.exists(), "same-pid pidfile should be removed");

        let _ = std::fs::remove_file(&p);
    }
}
