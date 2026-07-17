//! Agent Supervisor — 启动 Worker 进程、监控退出、处理二进制更新替换。
//!
//! 进程模型：
//!   PID 1: supervisor（本模块）
//!     └── worker（实际业务逻辑：WebSocket 连接、资源代理）
//!
//! Worker 退出码约定：
//!   0    — 正常退出，supervisor 也退出
//!   42   — 有待应用的更新，supervisor 读取 update-state.json 后替换并重启
//!   其他 — 异常崩溃，检查 update-state.json 是否有未完成的更新

use std::path::{Path, PathBuf};
use std::process::Command;

const EXIT_CODE_UPDATE: i32 = 42;
const MAX_RESTARTS: u32 = 3;

pub fn run_supervisor() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    let current_exe = std::env::current_exe().expect("failed to get current exe path");
    tracing::info!(
        name = "REX Agent",
        version = env!("CARGO_PKG_VERSION"),
        exe = %current_exe.display(),
        status = "supervisor starting"
    );

    let mut restart_count: u32 = 0;

    loop {
        // 启动 worker 子进程
        tracing::info!(restart = restart_count, "spawning worker");

        let status = Command::new(&current_exe)
            .env("REX_WORKER", "1")
            .status()
            .expect("failed to spawn worker");

        let code = status.code().unwrap_or(-1);
        tracing::info!(exit_code = code, "worker exited");

        match code {
            0 => {
                // 正常退出，supervisor 也退出
                tracing::info!("worker exited normally, supervisor shutting down");
                std::process::exit(0);
            }
            EXIT_CODE_UPDATE => {
                // Worker 准备了更新，supervisor 应用并重启
                tracing::info!("worker prepared update, applying...");
                match apply_update(&current_exe) {
                    Ok(version) => {
                        tracing::info!(version = %version, "update applied, restarting worker");
                        restart_count = 0; // 更新成功，重置计数
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to apply update");
                        // 更新失败，用旧版本重试
                        restart_count += 1;
                        if restart_count >= MAX_RESTARTS {
                            tracing::error!("worker crashed {MAX_RESTARTS} times, giving up");
                            std::process::exit(1);
                        }
                        // 删除可能损坏的 update-state.json
                        let _ = std::fs::remove_file(state_file_path(&current_exe));
                    }
                }
            }
            _ => {
                // 异常崩溃
                tracing::warn!(exit_code = code, "worker crashed");

                // 检查是否有未完成的更新
                if let Ok(true) = apply_update_if_pending(&current_exe) {
                    tracing::info!("applied pending update after crash, restarting worker");
                    restart_count = 0;
                    continue;
                }

                restart_count += 1;
                if restart_count >= MAX_RESTARTS {
                    tracing::error!("worker crashed {MAX_RESTARTS} times, giving up");
                    std::process::exit(1);
                }
                tracing::info!(
                    remaining = MAX_RESTARTS - restart_count,
                    "restarting worker"
                );
            }
        }
    }
}

/// 读取 update-state.json 并应用更新（rename tmp → current）
fn apply_update(current_exe: &Path) -> Result<String, String> {
    let state_path = state_file_path(current_exe);
    let state_json =
        std::fs::read_to_string(&state_path).map_err(|e| format!("read update-state.json: {e}"))?;

    let state: rex_common::update::UpdateStateFile =
        serde_json::from_str(&state_json).map_err(|e| format!("parse update-state.json: {e}"))?;

    let tmp_path = PathBuf::from(&state.tmp_path);
    if !tmp_path.exists() {
        return Err(format!("tmp file not found: {}", state.tmp_path));
    }

    // 校验 SHA256（如果提供了）
    if !state.sha256.is_empty() {
        let data = std::fs::read(&tmp_path).map_err(|e| format!("read tmp file: {e}"))?;
        let hash = sha256_hex(&data);
        if hash != state.sha256 {
            return Err(format!(
                "SHA256 mismatch in supervisor: expected {}, got {}",
                state.sha256, hash
            ));
        }
    }

    // 备份当前二进制
    let backup = current_exe.with_extension("bak");
    if let Err(e) = std::fs::copy(current_exe, &backup) {
        tracing::warn!(error = %e, "failed to backup current binary");
    }

    // rename tmp → current（原子操作）
    std::fs::rename(&tmp_path, current_exe).map_err(|e| format!("rename tmp to current: {e}"))?;

    // 清理 update-state.json
    let _ = std::fs::remove_file(&state_path);

    Ok(state.target_version)
}

/// 检查是否有未完成的更新并应用
fn apply_update_if_pending(current_exe: &Path) -> Result<bool, String> {
    let state_path = state_file_path(current_exe);
    if !state_path.exists() {
        return Ok(false);
    }

    match apply_update(current_exe) {
        Ok(_) => Ok(true),
        Err(e) => {
            tracing::error!(error = %e, "failed to apply pending update");
            let _ = std::fs::remove_file(&state_path);
            Err(e)
        }
    }
}

fn state_file_path(current_exe: &Path) -> PathBuf {
    current_exe.with_extension("update-state.json")
}

fn sha256_hex(data: &[u8]) -> String {
    rex_common::update::sha256_hex(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_file_path() {
        let exe = PathBuf::from("/usr/local/bin/rex-agent");
        let state = state_file_path(&exe);
        assert_eq!(
            state,
            PathBuf::from("/usr/local/bin/rex-agent.update-state.json")
        );
    }

    #[test]
    fn test_sha256_hex_matches() {
        let hash = sha256_hex(b"hello");
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
