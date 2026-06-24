use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use tracing::{info, warn};

use crate::FileConnector;
use crate::task::{TransferManager, TransferStatus};

/// 执行单个传输任务
///
/// 从 source connector 读取文件，写入 target connector，通过 manager 更新进度。
/// 本实现为全量读写（先读取全部内容到内存再写入），适用于中小文件。
pub async fn execute_transfer(
    manager: Arc<TransferManager>,
    task_id: String,
    source: Arc<dyn FileConnector>,
    target: Arc<dyn FileConnector>,
    source_path: PathBuf,
    target_path: PathBuf,
) {
    info!(
        task_id = %task_id,
        source = %source_path.display(),
        target = %target_path.display(),
        "transfer started"
    );

    // 设为 Running
    if let Err(e) = manager.set_status(&task_id, TransferStatus::Running).await {
        warn!(task_id = %task_id, error = %e, "failed to set status to Running");
        let _ = manager.set_status(&task_id, TransferStatus::Failed(format!("状态更新失败: {e}"))).await;
        return;
    }

    match do_transfer(&source, &target, &source_path, &target_path, &manager, &task_id).await {
        Ok(total) => {
            let _ = manager.set_progress(&task_id, total, total).await;
            let _ = manager.set_status(&task_id, TransferStatus::Completed).await;
            info!(task_id = %task_id, bytes = total, "transfer completed");
        }
        Err(e) => {
            let msg = format!("{e:#}");
            warn!(task_id = %task_id, error = %msg, "transfer failed");
            let _ = manager.set_status(&task_id, TransferStatus::Failed(msg)).await;
        }
    }
}

/// 实际执行传输逻辑
async fn do_transfer(
    source: &Arc<dyn FileConnector>,
    target: &Arc<dyn FileConnector>,
    source_path: &std::path::Path,
    target_path: &std::path::Path,
    manager: &Arc<TransferManager>,
    task_id: &str,
) -> Result<u64> {
    // 1. 获取源文件元数据
    let meta = source.metadata(source_path).await
        .map_err(|e| anyhow::anyhow!("读取源文件元数据失败: {e}"))?;

    let total = meta.size.unwrap_or(0);

    // 2. 读取源文件内容
    let read_result = source.read(source_path).await
        .map_err(|e| anyhow::anyhow!("读取源文件失败: {e}"))?;

    let bytes = &read_result.bytes;
    let transferred = bytes.len() as u64;

    // 3. 更新进度（读取完成）
    let _ = manager.set_progress(task_id, total.max(transferred), transferred).await;

    // 4. 写入目标
    target.write(target_path, bytes).await
        .map_err(|e| anyhow::anyhow!("写入目标文件失败: {e}"))?;

    // 5. 更新进度（写入完成）
    let _ = manager.set_progress(task_id, transferred, transferred).await;

    Ok(transferred)
}

// ── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    use crate::local::LocalConnector;
    use crate::task::TransferManager;

    fn make_ep(path: &str) -> crate::task::TransferEndpoint {
        crate::task::TransferEndpoint {
            connector_type: "local".into(),
            resource_id: None,
            sftp_host: None,
            sftp_port: None,
            sftp_username: None,
            path: PathBuf::from(path),
        }
    }

    #[tokio::test]
    async fn transfer_file_between_local_connectors() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();
        let src_conn = Arc::new(LocalConnector::new(src_dir.path().to_path_buf()).unwrap());
        let dst_conn = Arc::new(LocalConnector::new(dst_dir.path().to_path_buf()).unwrap());

        let manager = Arc::new(TransferManager::new());
        let task_id = manager.create_task(make_ep("/a.txt"), make_ep("/a.txt")).await;

        // 写入源文件（通过 connector）
        src_conn.write(Path::new("/a.txt"), b"hello world").await.unwrap();

        execute_transfer(
            manager.clone(), task_id.clone(),
            src_conn, dst_conn,
            "/a.txt".into(), "/a.txt".into(),
        )
        .await;

        // 验证目标目录有文件
        let dest_file = dst_dir.path().join("a.txt");
        assert!(dest_file.exists());
        assert_eq!(tokio::fs::read(&dest_file).await.unwrap(), b"hello world");

        // 验证任务状态
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, crate::task::TransferStatus::Completed);
        assert_eq!(task.progress.transferred_bytes, 11);
    }

    #[tokio::test]
    async fn transfer_nonexistent_source_fails() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();
        let src_conn = Arc::new(LocalConnector::new(src_dir.path().to_path_buf()).unwrap());
        let dst_conn = Arc::new(LocalConnector::new(dst_dir.path().to_path_buf()).unwrap());

        let manager = Arc::new(TransferManager::new());
        let task_id = manager.create_task(make_ep("/missing.txt"), make_ep("/missing.txt")).await;

        execute_transfer(
            manager.clone(), task_id.clone(),
            src_conn, dst_conn,
            "/missing.txt".into(), "/missing.txt".into(),
        )
        .await;

        let task = manager.get_task(&task_id).await.unwrap();
        assert!(matches!(task.status, crate::task::TransferStatus::Failed(_)));
    }

    #[tokio::test]
    async fn transfer_empty_file() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();
        let src_conn = Arc::new(LocalConnector::new(src_dir.path().to_path_buf()).unwrap());
        let dst_conn = Arc::new(LocalConnector::new(dst_dir.path().to_path_buf()).unwrap());

        let manager = Arc::new(TransferManager::new());
        let task_id = manager.create_task(make_ep("/e.txt"), make_ep("/e.txt")).await;

        src_conn.write(Path::new("/e.txt"), b"").await.unwrap();

        execute_transfer(
            manager.clone(), task_id.clone(),
            src_conn, dst_conn,
            "/e.txt".into(), "/e.txt".into(),
        )
        .await;

        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, crate::task::TransferStatus::Completed);
        assert_eq!(task.progress.transferred_bytes, 0);
    }

    #[tokio::test]
    async fn progress_updates_during_transfer() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();
        let src_conn = Arc::new(LocalConnector::new(src_dir.path().to_path_buf()).unwrap());
        let dst_conn = Arc::new(LocalConnector::new(dst_dir.path().to_path_buf()).unwrap());

        let manager = Arc::new(TransferManager::new());
        let task_id = manager.create_task(make_ep("/d.bin"), make_ep("/d.bin")).await;

        let data = vec![0u8; 1024];
        src_conn.write(Path::new("/d.bin"), &data).await.unwrap();

        execute_transfer(
            manager.clone(), task_id.clone(),
            src_conn, dst_conn,
            "/d.bin".into(), "/d.bin".into(),
        )
        .await;

        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, crate::task::TransferStatus::Completed);
        assert_eq!(task.progress.total_bytes, 1024);
        assert_eq!(task.progress.transferred_bytes, 1024);
    }
}
