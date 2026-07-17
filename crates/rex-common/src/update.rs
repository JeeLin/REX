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

/// 更新阶段（Agent 端进度上报使用）
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
/// Worker 收到更新指令后，下载二进制到 tmp_path，写此文件，然后 exit(42)。
/// Supervisor 检测到 exit(42) 后读取此文件，rename tmp_path → 当前二进制路径。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStateFile {
    pub target_version: String,
    pub tmp_path: String,
    pub sha256: String,
}

/// 计算 SHA256 哈希值并返回十六进制字符串。
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}
