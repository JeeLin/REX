use std::process::Command;
use std::thread;
use std::time::Duration;

pub struct SupervisorConfig {
    pub restart_delay: Duration,
}

/// 第一阶段 supervisor：启动 worker 子进程，监控退出并重启。
///
/// - worker exit 0 → supervisor 退出（正常关闭）
/// - worker exit 非 0 → 等待 restart_delay 后重启
pub fn run_supervisor(config: SupervisorConfig) -> anyhow::Result<()> {
    run_supervisor_with(
        || {
            let mut cmd = Command::new(std::env::current_exe().expect("failed to get current exe"));
            cmd.arg("--worker");
            cmd
        },
        config,
    )
}

/// 可测试的 supervisor：接受自定义命令构建器
pub fn run_supervisor_with(
    mut make_cmd: impl FnMut() -> Command,
    config: SupervisorConfig,
) -> anyhow::Result<()> {
    loop {
        let mut child = make_cmd().spawn()?;

        let status = child.wait()?;
        let code = status.code().unwrap_or(1);

        tracing::info!(exit_code = code, "worker exited");

        if code == 0 {
            break;
        }

        tracing::warn!(
            delay_ms = config.restart_delay.as_millis() as u64,
            "restarting worker"
        );
        thread::sleep(config.restart_delay);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supervisor_config_holds_restart_delay() {
        let config = SupervisorConfig {
            restart_delay: Duration::from_secs(2),
        };
        assert_eq!(config.restart_delay, Duration::from_secs(2));
    }

    #[test]
    fn supervisor_exits_when_worker_returns_zero() {
        let config = SupervisorConfig {
            restart_delay: Duration::from_millis(10),
        };
        let result = run_supervisor_with(|| Command::new("true"), config);
        assert!(result.is_ok());
    }

    #[test]
    fn supervisor_restarts_when_worker_returns_nonzero() {
        let config = SupervisorConfig {
            restart_delay: Duration::from_millis(10),
        };

        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&call_count);

        let result = run_supervisor_with(
            move || {
                let n = count.fetch_add(1, Ordering::SeqCst);
                let mut cmd = Command::new("sh");
                cmd.arg("-c");
                if n == 0 {
                    cmd.arg("exit 1");
                } else {
                    cmd.arg("exit 0");
                }
                cmd
            },
            config,
        );

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn supervisor_stops_after_multiple_restarts() {
        let config = SupervisorConfig {
            restart_delay: Duration::from_millis(10),
        };

        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&call_count);

        let result = run_supervisor_with(
            move || {
                let n = count.fetch_add(1, Ordering::SeqCst);
                let mut cmd = Command::new("sh");
                cmd.arg("-c");
                if n < 3 {
                    cmd.arg("exit 1");
                } else {
                    cmd.arg("exit 0");
                }
                cmd
            },
            config,
        );

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 4);
    }
}
