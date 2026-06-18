use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::info;

// ── 传输端点 ──────────────────────────────────────────────

/// 传输端点：描述源/目标连接信息和路径
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferEndpoint {
    /// 连接类型："local" | "sftp"
    pub connector_type: String,
    /// 资源 ID（关联 DB 中的资源记录）
    pub resource_id: Option<String>,
    /// SFTP 连接参数（仅 connector_type="sftp" 时使用）
    pub sftp_host: Option<String>,
    pub sftp_port: Option<u16>,
    pub sftp_username: Option<String>,
    /// 文件路径
    pub path: PathBuf,
}

// ── 传输状态 ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "reason")]
pub enum TransferStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

impl TransferStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransferStatus::Pending => "pending",
            TransferStatus::Running => "running",
            TransferStatus::Completed => "completed",
            TransferStatus::Failed(_) => "failed",
            TransferStatus::Cancelled => "cancelled",
        }
    }
}

// ── 传输进度 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferProgress {
    pub total_bytes: u64,
    pub transferred_bytes: u64,
}

impl TransferProgress {
    pub fn percent(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0
        }
    }
}

// ── 传输任务 ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferTask {
    pub id: String,
    pub source: TransferEndpoint,
    pub target: TransferEndpoint,
    pub status: TransferStatus,
    pub progress: TransferProgress,
    pub created_at: String,
    pub updated_at: String,
}

impl TransferTask {
    /// 创建新的传输任务（初始状态为 Pending）
    pub fn new(id: String, source: TransferEndpoint, target: TransferEndpoint) -> Self {
        let now = Self::now_iso();
        Self {
            id,
            source,
            target,
            status: TransferStatus::Pending,
            progress: TransferProgress {
                total_bytes: 0,
                transferred_bytes: 0,
            },
            created_at: now.clone(),
            updated_at: now,
        }
    }

    fn now_iso() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("{secs:010}")
    }
}

// ── 传输事件（WebSocket 推送） ───────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransferEvent {
    /// 任务状态变更
    StatusChanged {
        task_id: String,
        status: TransferStatus,
    },
    /// 进度更新
    Progress {
        task_id: String,
        progress: TransferProgress,
    },
    /// 任务完成
    Completed {
        task_id: String,
        progress: TransferProgress,
    },
    /// 任务失败
    Failed { task_id: String, error: String },
}

// ── 传输管理器 ────────────────────────────────────────────

/// 内存中的传输任务管理器
pub struct TransferManager {
    tasks: RwLock<Vec<Arc<RwLock<TransferTask>>>>,
}

impl TransferManager {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(Vec::new()),
        }
    }

    /// 创建新的传输任务，返回任务 ID
    pub async fn create_task(&self, source: TransferEndpoint, target: TransferEndpoint) -> String {
        let id = format!("xfer_{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let task = TransferTask::new(id.clone(), source, target);
        info!(task_id = %id, "transfer task created");
        self.tasks.write().await.push(Arc::new(RwLock::new(task)));
        id
    }

    /// 获取任务列表（只读快照）
    pub async fn list_tasks(&self) -> Vec<TransferTask> {
        let tasks = self.tasks.read().await;
        let mut result = Vec::with_capacity(tasks.len());
        for task in tasks.iter() {
            result.push(task.read().await.clone());
        }
        result
    }

    /// 获取单个任务
    pub async fn get_task(&self, task_id: &str) -> Option<TransferTask> {
        let tasks = self.tasks.read().await;
        for task in tasks.iter() {
            let t = task.read().await;
            if t.id == task_id {
                return Some(t.clone());
            }
        }
        None
    }

    /// 更新任务状态
    pub async fn set_status(&self, task_id: &str, status: TransferStatus) -> Result<()> {
        let tasks = self.tasks.read().await;
        for task in tasks.iter() {
            let mut t = task.write().await;
            if t.id == task_id {
                t.status = status;
                t.updated_at = TransferTask::now_iso();
                return Ok(());
            }
        }
        anyhow::bail!("task not found: {}", task_id)
    }

    /// 更新任务进度
    pub async fn set_progress(&self, task_id: &str, total: u64, transferred: u64) -> Result<()> {
        let tasks = self.tasks.read().await;
        for task in tasks.iter() {
            let mut t = task.write().await;
            if t.id == task_id {
                t.progress = TransferProgress {
                    total_bytes: total,
                    transferred_bytes: transferred,
                };
                t.updated_at = TransferTask::now_iso();
                return Ok(());
            }
        }
        anyhow::bail!("task not found: {}", task_id)
    }

    /// 取消任务（仅 Pending 和 Running 可取消）
    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        let tasks = self.tasks.read().await;
        for task in tasks.iter() {
            let mut t = task.write().await;
            if t.id == task_id {
                match &t.status {
                    TransferStatus::Pending | TransferStatus::Running => {
                        t.status = TransferStatus::Cancelled;
                        t.updated_at = TransferTask::now_iso();
                        info!(task_id = %task_id, "transfer task cancelled");
                        return Ok(());
                    }
                    _ => {
                        anyhow::bail!("cannot cancel task in status: {}", t.status.as_str())
                    }
                }
            }
        }
        anyhow::bail!("task not found: {}", task_id)
    }

    /// 移除任务（已完成/失败/已取消的）
    pub async fn remove_task(&self, task_id: &str) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        let before = tasks.len();
        // try_read 安全：持有外层写锁时不会有其他方法持有内层锁
        tasks.retain(|t| match t.try_read() {
            Ok(t) => t.id != task_id,
            Err(_) => true, // 无法读取时保留任务（防御性编程）
        });
        if tasks.len() < before {
            info!(task_id = %task_id, "transfer task removed");
            Ok(())
        } else {
            anyhow::bail!("task not found: {}", task_id)
        }
    }
}

impl Default for TransferManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_endpoint(path: &str) -> TransferEndpoint {
        TransferEndpoint {
            connector_type: "local".to_string(),
            resource_id: None,
            sftp_host: None,
            sftp_port: None,
            sftp_username: None,
            path: PathBuf::from(path),
        }
    }

    #[test]
    fn transfer_status_as_str() {
        assert_eq!(TransferStatus::Pending.as_str(), "pending");
        assert_eq!(TransferStatus::Running.as_str(), "running");
        assert_eq!(TransferStatus::Completed.as_str(), "completed");
        assert_eq!(TransferStatus::Failed("err".into()).as_str(), "failed");
        assert_eq!(TransferStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn transfer_progress_percent() {
        let p = TransferProgress {
            total_bytes: 200,
            transferred_bytes: 50,
        };
        assert_eq!(p.percent(), 25.0);

        let p = TransferProgress {
            total_bytes: 0,
            transferred_bytes: 0,
        };
        assert_eq!(p.percent(), 0.0);
    }

    #[test]
    fn transfer_task_new_has_pending_status() {
        let task = TransferTask::new("xfer_0001".into(), make_endpoint("/a"), make_endpoint("/b"));
        assert_eq!(task.status, TransferStatus::Pending);
        assert_eq!(task.progress.transferred_bytes, 0);
        assert!(!task.created_at.is_empty());
    }

    #[test]
    fn transfer_status_json_roundtrip() {
        let statuses = vec![
            TransferStatus::Pending,
            TransferStatus::Running,
            TransferStatus::Completed,
            TransferStatus::Failed("timeout".into()),
            TransferStatus::Cancelled,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: TransferStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.as_str(), status.as_str());
        }
    }

    #[tokio::test]
    async fn manager_create_and_list() {
        let mgr = TransferManager::new();
        let id = mgr
            .create_task(make_endpoint("/a"), make_endpoint("/b"))
            .await;
        assert!(id.starts_with("xfer_"));

        let tasks = mgr.list_tasks().await;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, id);
    }

    #[tokio::test]
    async fn manager_get_task() {
        let mgr = TransferManager::new();
        let id = mgr
            .create_task(make_endpoint("/a"), make_endpoint("/b"))
            .await;

        let task = mgr.get_task(&id).await;
        assert!(task.is_some());
        assert_eq!(task.unwrap().id, id);

        let missing = mgr.get_task("nonexistent").await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn manager_set_status_and_progress() {
        let mgr = TransferManager::new();
        let id = mgr
            .create_task(make_endpoint("/a"), make_endpoint("/b"))
            .await;

        mgr.set_status(&id, TransferStatus::Running).await.unwrap();
        mgr.set_progress(&id, 100, 50).await.unwrap();

        let task = mgr.get_task(&id).await.unwrap();
        assert_eq!(task.status, TransferStatus::Running);
        assert_eq!(task.progress.total_bytes, 100);
        assert_eq!(task.progress.transferred_bytes, 50);
    }

    #[tokio::test]
    async fn manager_cancel_pending_task() {
        let mgr = TransferManager::new();
        let id = mgr
            .create_task(make_endpoint("/a"), make_endpoint("/b"))
            .await;

        mgr.cancel_task(&id).await.unwrap();
        let task = mgr.get_task(&id).await.unwrap();
        assert_eq!(task.status, TransferStatus::Cancelled);
    }

    #[tokio::test]
    async fn manager_cancel_completed_task_fails() {
        let mgr = TransferManager::new();
        let id = mgr
            .create_task(make_endpoint("/a"), make_endpoint("/b"))
            .await;
        mgr.set_status(&id, TransferStatus::Completed)
            .await
            .unwrap();

        let result = mgr.cancel_task(&id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn manager_remove_task() {
        let mgr = TransferManager::new();
        let id = mgr
            .create_task(make_endpoint("/a"), make_endpoint("/b"))
            .await;
        mgr.set_status(&id, TransferStatus::Completed)
            .await
            .unwrap();

        mgr.remove_task(&id).await.unwrap();
        assert!(mgr.get_task(&id).await.is_none());
    }

    #[tokio::test]
    async fn manager_remove_nonexistent_fails() {
        let mgr = TransferManager::new();
        let result = mgr.remove_task("nonexistent").await;
        assert!(result.is_err());
    }

    #[test]
    fn transfer_endpoint_json_roundtrip() {
        let ep = TransferEndpoint {
            connector_type: "sftp".into(),
            resource_id: Some("res_001".into()),
            sftp_host: Some("192.168.1.1".into()),
            sftp_port: Some(22),
            sftp_username: Some("root".into()),
            path: PathBuf::from("/data/file.tar.gz"),
        };
        let json = serde_json::to_string(&ep).unwrap();
        let parsed: TransferEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(ep, parsed);
    }

    #[test]
    fn transfer_event_json_roundtrip() {
        let events = vec![
            TransferEvent::StatusChanged {
                task_id: "xfer_001".into(),
                status: TransferStatus::Running,
            },
            TransferEvent::Progress {
                task_id: "xfer_001".into(),
                progress: TransferProgress {
                    total_bytes: 100,
                    transferred_bytes: 50,
                },
            },
            TransferEvent::Completed {
                task_id: "xfer_001".into(),
                progress: TransferProgress {
                    total_bytes: 100,
                    transferred_bytes: 100,
                },
            },
            TransferEvent::Failed {
                task_id: "xfer_001".into(),
                error: "connection lost".into(),
            },
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let _parsed: TransferEvent = serde_json::from_str(&json).unwrap();
        }
    }
}
