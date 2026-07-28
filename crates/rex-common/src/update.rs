//! 更新模型 — Hub 和 Agent 共享的更新指令和 supervisor-worker 通信协议。

use serde::{Deserialize, Serialize};

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
