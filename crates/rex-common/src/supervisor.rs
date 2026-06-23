use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub struct SupervisorConfig {
    pub restart_delay: Duration,
}

/// 更新相关的 supervisor 配置
pub struct UpdateSupervisorConfig {
    pub restart_delay: Duration,
    pub data_dir: PathBuf,
    pub health_check_timeout: Duration,
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

/// 退出码语义
pub const EXIT_NORMAL: i32 = 0;
pub const EXIT_UPDATE_REQUESTED: i32 = 10;
pub const EXIT_HEALTH_FAILED: i32 = 11;
pub const EXIT_CRASHED: i32 = 12;
pub const MAX_ATTEMPTS: u32 = 3;

/// 第二阶段 supervisor：支持自动更新、健康检查、回滚。
///
/// 退出码语义：
/// - 0: 正常退出
/// - 10: 请求更新
/// - 11: 健康检查失败
/// - 12: 崩溃/异常退出
pub fn run_update_supervisor(
    mut make_cmd: impl FnMut() -> Command,
    config: UpdateSupervisorConfig,
) -> anyhow::Result<()> {
    use crate::update_state::{UpdatePhase, UpdateState};

    let state_path = config.data_dir.join("update-state.json");
    let mut state = UpdateState::read(&state_path);

    loop {
        // 构建 worker 命令
        let mut cmd = make_cmd();

        // 如果是更新后的首次启动，传入 REX_UPDATE_PENDING
        if state.phase == UpdatePhase::StartingNew {
            tracing::info!(version = %state.target_version, "starting new worker with REX_UPDATE_PENDING");
            cmd.env("REX_UPDATE_PENDING", "1");
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "failed to spawn worker");
                thread::sleep(config.restart_delay);
                continue;
            }
        };

        let status = child.wait()?;
        let code = status.code().unwrap_or(EXIT_CRASHED);

        tracing::info!(exit_code = code, phase = ?state.phase, attempt = state.attempt, "worker exited");

        match code {
            EXIT_NORMAL => {
                match state.phase {
                    UpdatePhase::Requested => {
                        // 替换二进制
                        tracing::info!("performing binary replacement");
                        if let Err(e) = perform_replacement(&state, &config.data_dir) {
                            tracing::error!(error = %e, "binary replacement failed");
                            state.phase = UpdatePhase::Failed;
                            state.write(&state_path)?;
                            break;
                        }
                        state.phase = UpdatePhase::StartingNew;
                        state.write(&state_path)?;
                        // 继续循环，启动新 worker
                    }
                    UpdatePhase::StartingNew => {
                        // 新版本健康检查通过
                        tracing::info!("update committed successfully");
                        state.phase = UpdatePhase::Committed;
                        state.write(&state_path)?;
                        // 清理 rollback
                        if !state.rollback_path.is_empty() {
                            let _ = std::fs::remove_file(&state.rollback_path);
                        }
                        state.phase = UpdatePhase::Idle;
                        state.write(&state_path)?;
                        break;
                    }
                    _ => {
                        // 正常退出，不再重启
                        break;
                    }
                }
            }
            EXIT_UPDATE_REQUESTED => {
                if state.attempt >= MAX_ATTEMPTS {
                    tracing::warn!("max attempts reached, rolling back");
                    rollback(&mut state, &state_path, &config.data_dir)?;
                    break;
                }
                state.attempt += 1;
                if state.phase != UpdatePhase::Requested {
                    state.phase = UpdatePhase::Requested;
                }
                state.write(&state_path)?;
                // 继续循环，执行替换
            }
            EXIT_HEALTH_FAILED | EXIT_CRASHED => {
                state.attempt += 1;
                if state.attempt >= MAX_ATTEMPTS {
                    tracing::warn!(exit_code = code, "max attempts reached, rolling back");
                    rollback(&mut state, &state_path, &config.data_dir)?;
                    break;
                }
                state.write(&state_path)?;
                tracing::warn!(
                    delay_ms = config.restart_delay.as_millis() as u64,
                    attempt = state.attempt,
                    "restarting worker"
                );
                thread::sleep(config.restart_delay);
            }
            _ => {
                // 未知退出码，等待后重启
                tracing::warn!(exit_code = code, "unknown exit code, restarting");
                thread::sleep(config.restart_delay);
            }
        }
    }

    Ok(())
}

/// 执行二进制替换：staged → 当前二进制
fn perform_replacement(
    state: &crate::update_state::UpdateState,
    _data_dir: &PathBuf,
) -> anyhow::Result<()> {
    let staged = std::path::Path::new(&state.staged_path);
    if !staged.exists() {
        anyhow::bail!("staged binary not found: {}", state.staged_path);
    }

    let current_exe = std::env::current_exe()?;

    // 备份当前二进制（如果还没有备份）
    if state.rollback_path.is_empty() || !std::path::Path::new(&state.rollback_path).exists() {
        let rollback_dir = current_exe
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("rollback");
        std::fs::create_dir_all(&rollback_dir)?;
        let rollback_path = rollback_dir.join(
            current_exe
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("rex")),
        );
        std::fs::copy(&current_exe, &rollback_path)?;
        tracing::info!(path = %rollback_path.display(), "backed up current binary");
    }

    // 替换：在 Unix 上 rename 是原子的
    // 先写入临时文件再 rename
    let tmp_path = current_exe.with_extension("tmp");
    std::fs::copy(staged, &tmp_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp_path, perms)?;
    }

    std::fs::rename(&tmp_path, &current_exe)?;
    tracing::info!("binary replacement completed");

    Ok(())
}

/// 回滚到旧版本
fn rollback(
    state: &mut crate::update_state::UpdateState,
    state_path: &std::path::Path,
    _data_dir: &PathBuf,
) -> anyhow::Result<()> {
    state.phase = crate::update_state::UpdatePhase::RollingBack;
    state.write(state_path)?;

    let rollback = std::path::Path::new(&state.rollback_path);
    if !rollback.exists() {
        tracing::error!("rollback binary not found, cannot rollback");
        state.phase = crate::update_state::UpdatePhase::Failed;
        state.write(state_path)?;
        return Ok(());
    }

    let current_exe = std::env::current_exe()?;
    let tmp_path = current_exe.with_extension("tmp");
    std::fs::copy(rollback, &tmp_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp_path, perms)?;
    }

    std::fs::rename(&tmp_path, &current_exe)?;
    state.phase = crate::update_state::UpdatePhase::RolledBack;
    state.write(state_path)?;

    tracing::info!("rollback completed");

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

    #[test]
    fn exit_code_constants_are_correct() {
        assert_eq!(EXIT_NORMAL, 0);
        assert_eq!(EXIT_UPDATE_REQUESTED, 10);
        assert_eq!(EXIT_HEALTH_FAILED, 11);
        assert_eq!(EXIT_CRASHED, 12);
        assert_eq!(MAX_ATTEMPTS, 3);
    }

    #[test]
    fn update_supervisor_config_holds_restart_delay() {
        let config = UpdateSupervisorConfig {
            restart_delay: Duration::from_secs(3),
            data_dir: PathBuf::from("/tmp"),
            health_check_timeout: Duration::from_secs(30),
        };
        assert_eq!(config.restart_delay, Duration::from_secs(3));
        assert_eq!(config.data_dir, PathBuf::from("/tmp"));
        assert_eq!(config.health_check_timeout, Duration::from_secs(30));
    }
}
