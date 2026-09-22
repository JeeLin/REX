//! 更新模型 — Hub 和 Agent 共享的更新指令和 supervisor-worker 通信协议。

use std::path::Path;

use serde::{Deserialize, Serialize};

/// 更新替换过程中产生、更新完成后应清理的二进制同目录残留文件后缀。
///
/// - `old`：Windows 上 rename 当前二进制让位（Unix 直接覆盖，不产生）
/// - `bak`：替换前对当前二进制的备份副本（全平台）
pub const UPDATE_LEFTOVER_EXTS: [&str; 2] = ["old", "bak"];

/// 清理 `current_exe` 旁的 `.old` / `.bak` 更新残留文件。
///
/// 尽力而为：单个文件删除失败仅记录 warning，不返回错误、不阻断更新流程。
/// （典型场景：Windows 上 supervisor 仍从 `.old` 镜像运行，删除会因文件锁失败，
/// 留待 supervisor 下次启动时清理。）
pub fn cleanup_update_leftovers(current_exe: &Path) {
    for ext in UPDATE_LEFTOVER_EXTS {
        let path = current_exe.with_extension(ext);
        if !path.exists() {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => {
                tracing::info!(
                    ext,
                    path = %path.display(),
                    "cleaned up leftover file from previous update"
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    ext,
                    path = %path.display(),
                    "failed to remove leftover file"
                );
            }
        }
    }
}

/// 更新指令（Hub → Agent，通过 WebSocket 心跳推送）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCommand {
    pub version: String,
    pub download_url: String,
    pub fallback_url: String,
    pub sha256: String,
}

/// 更新阶段（Agent 端进度上报 / Hub supervisor 使用）
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdatePhase {
    #[default]
    Idle,
    Downloading,
    Verifying,
    Replacing,
    Restarting,
    Error,
    /// Worker 已写入 update-state，请求 supervisor 替换二进制
    Requested,
    /// Supervisor 正在替换二进制并启动新版 worker
    StartingNew,
    /// 新版本健康检查通过，更新完成
    Committed,
    /// 新版本健康检查失败，supervisor 正在恢复旧版
    RollingBack,
    /// 回滚完成，旧版 worker 正在运行
    RolledBack,
    /// 连续 3 次启动失败，更新终止
    Failed,
}

/// 更新进度（Agent → Hub，通过 WebSocket 上报）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProgress {
    pub phase: UpdatePhase,
    #[serde(default)]
    pub progress: f64,
    #[serde(default)]
    pub error: Option<String>,
}

/// update-state.json — Worker 写入，Supervisor 读取
///
/// Worker 下载新版二进制后写此文件，然后优雅退出（exit(10)）。
/// Supervisor 检测到 exit(10) 后读取此文件，执行原子替换。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateStateFile {
    /// 当前更新阶段
    #[serde(default)]
    pub phase: UpdatePhase,
    /// 目标版本号
    pub target_version: String,
    /// 更新前的旧版本号
    #[serde(default)]
    pub old_version: String,
    /// 新二进制暂存路径
    #[serde(default)]
    pub staged_path: String,
    /// 旧二进制备份路径（用于回滚）
    #[serde(default)]
    pub rollback_path: String,
    /// SHA256 校验值（十六进制）
    #[serde(default)]
    pub sha256: String,
    /// 重启尝试次数
    #[serde(default)]
    pub attempt: u32,
}

/// 计算 SHA256 哈希值并返回十六进制字符串。
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_exe(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rex-update-leftover-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.join("rex-agent")
    }

    #[test]
    fn test_cleanup_removes_both_suffixes() {
        let exe = temp_exe("both");
        let bak = exe.with_extension("bak");
        let old = exe.with_extension("old");
        std::fs::write(&exe, b"exe").unwrap();
        std::fs::write(&bak, b"bak").unwrap();
        std::fs::write(&old, b"old").unwrap();

        cleanup_update_leftovers(&exe);

        assert!(!bak.exists(), ".bak leftover should be removed");
        assert!(!old.exists(), ".old leftover should be removed");
        assert!(exe.exists(), "current binary must not be touched");

        let _ = std::fs::remove_dir_all(exe.parent().unwrap());
    }

    #[test]
    fn test_cleanup_missing_files_is_noop() {
        let exe = temp_exe("missing");
        // 无残留时静默返回，不 panic
        cleanup_update_leftovers(&exe);
        let _ = std::fs::remove_dir_all(exe.parent().unwrap());
    }
}
