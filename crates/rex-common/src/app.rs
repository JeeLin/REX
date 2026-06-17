use crate::cli::Cli;
use crate::supervisor::{run_supervisor_with, SupervisorConfig};
use clap::Parser;
use std::time::Duration;

/// 初始化 tracing 日志
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
}

/// 应用入口：从环境参数解析 CLI，分发到 worker 或 supervisor
pub fn run() -> anyhow::Result<()> {
    run_from(std::env::args())
}

/// 应用入口：从指定参数解析 CLI，分发到 worker 或 supervisor
pub fn run_from(args: impl IntoIterator<Item = String>) -> anyhow::Result<()> {
    run_from_with(args, default_supervisor)
}

/// 可测试的入口：接受参数和自定义 supervisor 工厂
pub fn run_from_with(
    args: impl IntoIterator<Item = String>,
    make_supervisor: fn() -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let cli = Cli::parse_from(args);

    if cli.worker {
        tracing::info!("worker started");
    } else {
        make_supervisor()?;
    }

    Ok(())
}

/// 默认 supervisor（使用当前可执行文件）
fn default_supervisor() -> anyhow::Result<()> {
    run_supervisor_with(
        || {
            let mut cmd = std::process::Command::new(
                std::env::current_exe().expect("failed to get current exe"),
            );
            cmd.arg("--worker");
            cmd
        },
        SupervisorConfig {
            restart_delay: Duration::from_secs(1),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn init_tracing_does_not_panic() {
        init_tracing();
    }

    #[test]
    fn run_from_worker_exits_cleanly() {
        let args = vec!["rex".to_string(), "--worker".to_string()];
        let result = run_from(args);
        assert!(result.is_ok());
    }

    #[test]
    fn run_from_with_worker_flag() {
        let args = vec!["rex".to_string(), "--worker".to_string()];
        let result = run_from_with(args, || Ok(()));
        assert!(result.is_ok());
    }

    #[test]
    fn run_from_with_supervisor_flag() {
        static CALLED: AtomicUsize = AtomicUsize::new(0);

        fn mock_supervisor() -> anyhow::Result<()> {
            CALLED.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        let args = vec!["rex".to_string()];
        let result = run_from_with(args, mock_supervisor);
        assert!(result.is_ok());
        assert_eq!(CALLED.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn run_from_supervisor_with_exit_zero() {
        let config = SupervisorConfig {
            restart_delay: Duration::from_millis(10),
        };
        let result = run_supervisor_with(
            || {
                let mut cmd = std::process::Command::new("sh");
                cmd.arg("-c").arg("exit 0");
                cmd
            },
            config,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn run_from_supervisor_restarts_on_nonzero() {
        let config = SupervisorConfig {
            restart_delay: Duration::from_millis(10),
        };

        let call_count = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&call_count);

        let result = run_supervisor_with(
            move || {
                let n = count.fetch_add(1, Ordering::SeqCst);
                let mut cmd = std::process::Command::new("sh");
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
}
