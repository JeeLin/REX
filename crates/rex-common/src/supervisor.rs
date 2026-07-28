//! Supervisor 进程管理 — 监控 worker 子进程，处理更新信号、健康检查和回滚。
//!
//! supervisor 是 Hub/Agent 的父进程，在 `main()` 中启动。
//! 它 spawn worker 子进程并监控其退出码，根据退出码和 update-state.json 决定下一步。

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::Duration;

use crate::update::{UpdatePhase, UpdateStateFile};

/// Supervisor 配置
pub struct SupervisorConfig {
    /// 数据目录（update-state.json 所在位置）
    pub data_dir: PathBuf,
    /// worker 健康检查 URL
    pub health_url: String,
    /// 最大重启尝试次数
    pub max_restart_attempts: u32,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("."),
            health_url: "http://127.0.0.1:3000/api/health".into(),
            max_restart_attempts: 3,
        }
    }
}

/// Exit code: 正常退出
pub const EXIT_NORMAL: i32 = 0;
/// Exit code: 请求更新（worker 写入 update-state.json 后退出）
pub const EXIT_UPDATE_REQUEST: i32 = 10;
/// Exit code: 健康检查失败
pub const EXIT_HEALTH_FAILURE: i32 = 11;
/// Exit code: 崩溃/异常退出
pub const EXIT_CRASH: i32 = 12;

/// 启动 supervisor 主循环。
///
/// 不应直接调用此函数；应由 `main()` 根据进程角色决定调用。
pub fn run_supervisor(config: SupervisorConfig, worker_args: &[String]) -> ! {
    let mut attempt: u32 = 0;

    loop {
        tracing::info!(
            action = "SUPERVISOR_SPAWN",
            attempt,
            "spawning worker process"
        );

        let status = spawn_worker(worker_args);
        let code = extract_exit_code(&status);

        tracing::info!(
            action = "SUPERVISOR_WORKER_EXIT",
            exit_code = code,
            "worker process exited"
        );

        let state_path = config.data_dir.join("update-state.json");
        let state = read_update_state(&state_path);

        match code {
            EXIT_NORMAL => {
                // 正常退出：检查是否有待处理的更新
                if let Some(ref s) = state {
                    if s.phase == UpdatePhase::Requested {
                        tracing::info!(
                            action = "SUPERVISOR_UPDATE",
                            target = %s.target_version,
                            "worker exited normally with pending update, replacing binary"
                        );
                        if replace_binary(s) {
                            attempt = 0;
                            write_update_phase(&state_path, UpdatePhase::StartingNew, 0);
                            continue; // 启动新版
                        }
                    }
                }
                // 无待处理更新，短暂等待后重启
                attempt = 0;
                std::thread::sleep(Duration::from_secs(1));
            }
            EXIT_UPDATE_REQUEST => {
                // Worker 请求更新
                if let Some(ref s) = state {
                    tracing::info!(
                        action = "SUPERVISOR_UPDATE",
                        target = %s.target_version,
                        "worker requested update, replacing binary"
                    );
                    if replace_binary(s) {
                        attempt = 0;
                        write_update_phase(&state_path, UpdatePhase::StartingNew, 0);
                        continue; // 启动新版
                    }
                    // 替换失败，回滚
                    tracing::error!(action = "SUPERVISOR_UPDATE_FAILED", "binary replacement failed");
                }
                attempt += 1;
            }
            EXIT_HEALTH_FAILURE | EXIT_CRASH => {
                attempt += 1;
                tracing::warn!(
                    action = "SUPERVISOR_FAILURE",
                    exit_code = code,
                    attempt,
                    max = config.max_restart_attempts,
                    "worker exited with failure"
                );
                if attempt >= config.max_restart_attempts {
                    tracing::error!(
                        action = "SUPERVISOR_ROLLBACK",
                        "max restart attempts reached, rolling back"
                    );
                    if let Some(ref s) = state {
                        rollback(s, &state_path);
                    }
                    attempt = 0;
                }
            }
            _ => {
                // 未知退出码，短暂等待后重启
                attempt += 1;
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

/// Spawn worker 子进程并等待其退出。
fn spawn_worker(args: &[String]) -> ExitStatus {
    let current_exe = std::env::current_exe().expect("failed to get current executable path");

    let mut cmd = Command::new(current_exe);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.env("REX_WORKER", "1");

    cmd.status().expect("failed to spawn worker process")
}

/// 从退出状态提取退出码。
fn extract_exit_code(status: &ExitStatus) -> i32 {
    status.code().unwrap_or(EXIT_CRASH)
}

/// 从 update-state.json 读取更新状态。
fn read_update_state(path: &Path) -> Option<UpdateStateFile> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// 原子写入 update-state.json。
fn write_update_state(path: &Path, state: &UpdateStateFile) {
    let tmp_path = path.with_extension("json.tmp");
    if let Ok(json) = serde_json::to_string_pretty(state) {
        if std::fs::write(&tmp_path, json).is_ok() {
            let _ = std::fs::rename(&tmp_path, path);
        }
    }
}

/// 更新 update-state.json 中的 phase 和 attempt。
fn write_update_phase(path: &Path, phase: UpdatePhase, attempt: u32) {
    if let Some(mut state) = read_update_state(path) {
        state.phase = phase;
        state.attempt = attempt;
        write_update_state(path, &state);
    }
}

/// 执行二进制替换：备份旧版 → rename staging → 当前路径。
fn replace_binary(state: &UpdateStateFile) -> bool {
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "failed to get current exe path");
            return false;
        }
    };

    let staging = PathBuf::from(&state.staged_path);
    if !staging.exists() {
        tracing::error!(path = %staging.display(), "staged binary not found");
        return false;
    }

    // 备份当前二进制
    let backup = PathBuf::from(&state.rollback_path);
    if let Some(parent) = backup.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::copy(&current_exe, &backup) {
        tracing::error!(error = %e, "failed to backup current binary");
        return false;
    }

    // 原子替换：rename staging → current
    if let Err(e) = std::fs::rename(&staging, &current_exe) {
        tracing::error!(error = %e, "failed to replace binary");
        // 尝试恢复备份
        let _ = std::fs::copy(&backup, &current_exe);
        return false;
    }

    // 设置执行权限（Unix）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755));
    }

    tracing::info!(
        action = "SUPERVISOR_REPLACE",
        "binary replaced successfully"
    );
    true
}

/// 回滚到旧版本二进制。
fn rollback(state: &UpdateStateFile, state_path: &Path) {
    let rollback = PathBuf::from(&state.rollback_path);
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "failed to get current exe for rollback");
            return;
        }
    };

    if !rollback.exists() {
        tracing::error!(path = %rollback.display(), "rollback binary not found");
        write_update_phase(state_path, UpdatePhase::Failed, state.attempt);
        return;
    }

    if let Err(e) = std::fs::copy(&rollback, &current_exe) {
        tracing::error!(error = %e, "failed to restore rollback binary");
        write_update_phase(state_path, UpdatePhase::Failed, state.attempt);
        return;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755));
    }

    tracing::info!(action = "SUPERVISOR_ROLLBACK", "rolled back to previous version");
    write_update_phase(state_path, UpdatePhase::RolledBack, 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update::UpdateStateFile;

    #[test]
    fn test_extract_exit_code() {
        // 无法直接构造 ExitStatus，测试常量值
        assert_eq!(EXIT_NORMAL, 0);
        assert_eq!(EXIT_UPDATE_REQUEST, 10);
        assert_eq!(EXIT_HEALTH_FAILURE, 11);
        assert_eq!(EXIT_CRASH, 12);
    }

    #[test]
    fn test_update_state_file_roundtrip() {
        let state = UpdateStateFile {
            phase: UpdatePhase::Requested,
            target_version: "0.45.0".into(),
            old_version: "0.44.0".into(),
            staged_path: "/tmp/rex-hub".into(),
            rollback_path: "/tmp/rex-hub.old".into(),
            sha256: "abc123".into(),
            attempt: 0,
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: UpdateStateFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.phase, UpdatePhase::Requested);
        assert_eq!(parsed.target_version, "0.45.0");
        assert_eq!(parsed.attempt, 0);
    }

    #[test]
    fn test_update_state_file_default() {
        let state = UpdateStateFile::default();
        assert_eq!(state.phase, UpdatePhase::Idle);
        assert_eq!(state.attempt, 0);
        assert!(state.target_version.is_empty());
    }

    #[test]
    fn test_update_state_file_backward_compat() {
        // 旧格式只有 target_version, tmp_path, sha256
        let old_json = r#"{"target_version":"0.44.0","tmp_path":"/tmp/rex","sha256":"abc"}"#;
        let state: UpdateStateFile = serde_json::from_str(old_json).unwrap();
        assert_eq!(state.target_version, "0.44.0");
        assert_eq!(state.phase, UpdatePhase::Idle); // default
        assert_eq!(state.attempt, 0); // default
    }
}
