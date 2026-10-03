//! 传输任务模型 — v0.91.0 落地架构文档 (`docs/architecture/file-transfer.md`
//! §TransferCoordinator) 中承诺的 `TransferTask` / `TransferStatus` / `ConflictPolicy`。
//!
//! 模型层面供 rex-hub 持久化 + 前端进度查询 API 共享；跨连接直连调度/执行面交由
//! T2 的 TransferCoordinator 落地。

use rex_common::file_transfer::FileConnectRequest;
use serde::{Deserialize, Serialize};

/// 目标路径已存在时的冲突处理策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictPolicy {
    #[default]
    Overwrite,
    Skip,
    Rename,
    Fail,
}

impl std::fmt::Display for ConflictPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ConflictPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Overwrite => "overwrite",
            Self::Skip => "skip",
            Self::Rename => "rename",
            Self::Fail => "fail",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "overwrite" => Self::Overwrite,
            "skip" => Self::Skip,
            "rename" => Self::Rename,
            "fail" => Self::Fail,
            _ => return None,
        })
    }
}

/// 传输任务生命周期状态。
///
/// `Failed(String)` 携带错误信息；持久化时 state 存为 `as_str_lossy()` 对应字符串，
/// 错误文本存于 `transfer_task.error` 列（详见 `rex-hub/src/db.rs`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransferStatus {
    Pending,
    Running,
    Paused,
    Canceling,
    Verifying,
    Completed,
    Failed(String),
    Canceled,
}

impl Default for TransferStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl TransferStatus {
    pub fn as_str_lossy(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Canceling => "canceling",
            Self::Verifying => "verifying",
            Self::Completed => "completed",
            Self::Failed(_) => "failed",
            Self::Canceled => "canceled",
        }
    }

    /// 从状态字符串恢复（`Failed` 的错误文本单独回填）。
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            "paused" => Self::Paused,
            "canceling" => Self::Canceling,
            "verifying" => Self::Verifying,
            "completed" => Self::Completed,
            "failed" => Self::Failed(String::new()),
            "canceled" => Self::Canceled,
            _ => Self::Pending,
        }
    }
}

/// 传输进度快照。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct TransferProgress {
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub speed_bytes_per_sec: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_seconds: Option<u64>,
}

/// 可序列化的传输端点描述 — 复用连接请求（含协议专属字段）。
pub type TransferEndpoint = FileConnectRequest;

/// 单次跨连接传输任务。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferTask {
    pub id: String,
    pub source: TransferEndpoint,
    pub target: TransferEndpoint,
    pub source_path: String,
    pub target_path: String,
    pub conflict_policy: ConflictPolicy,
    pub status: TransferStatus,
    pub progress: TransferProgress,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_policy_round_trip() {
        assert_eq!(ConflictPolicy::default(), ConflictPolicy::Overwrite);
        assert_eq!(ConflictPolicy::Overwrite.as_str(), "overwrite");
        assert_eq!(ConflictPolicy::Rename.as_str(), "rename");
        assert_eq!(
            ConflictPolicy::from_str("rename"),
            Some(ConflictPolicy::Rename)
        );
        assert_eq!(ConflictPolicy::from_str("nope"), None);
        assert_eq!(ConflictPolicy::Fail.to_string(), "fail");
    }

    #[test]
    fn transfer_status_round_trip() {
        assert_eq!(TransferStatus::default(), TransferStatus::Pending);
        assert_eq!(
            TransferStatus::Failed("boom".into()).as_str_lossy(),
            "failed"
        );
        assert_eq!(
            TransferStatus::from_str_lossy("failed"),
            TransferStatus::Failed(String::new())
        );
        assert_eq!(
            TransferStatus::from_str_lossy("completed"),
            TransferStatus::Completed
        );
        assert_eq!(
            TransferStatus::from_str_lossy("unknown"),
            TransferStatus::Pending
        );
    }

    #[test]
    fn progress_default_and_serde() {
        let p = TransferProgress::default();
        assert_eq!(p.total_bytes, 0);
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("transferred_bytes"));
        let back: TransferProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }
}
