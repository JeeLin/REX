//! 文件传输引擎 — `FileConnector` trait 定义与共享传输逻辑。
//!
//! 各协议（SFTP、S3、本地）各自实现 [`rex_common::file_transfer::FileConnector`]。
//! 本 crate 承载跨协议的调度、并发与续传逻辑（里程碑 0.70.3 起逐步落地）。

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use rex_common::file_transfer::{FileConnector, FileEntry, ProgressCallback, UploadResult};

/// 默认分片大小（5MB），与 S3 multipart 最小分片对齐。
pub const DEFAULT_PART_SIZE: u64 = 5 * 1024 * 1024;

/// 根据总大小计算分片数（向上取整，至少 1 片）。
pub fn part_count(total: u64, part_size: u64) -> u64 {
    if total == 0 {
        1
    } else {
        total.div_ceil(part_size)
    }
}

/// 内存实现，用于传输调度逻辑的单元测试，不接触真实后端。
#[derive(Default)]
pub struct MemConnector {
    files: Mutex<HashMap<String, Vec<u8>>>,
}

#[async_trait]
impl FileConnector for MemConnector {
    async fn list(&mut self, path: &str) -> anyhow::Result<Vec<FileEntry>> {
        let prefix = path.trim_start_matches('/');
        let entries: Vec<FileEntry> = self
            .files
            .lock()
            .unwrap()
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| FileEntry {
                name: k.clone(),
                path: k.clone(),
                is_dir: false,
                size: v.len() as u64,
                modified: None,
                permissions: None,
                storage_class: None,
                acl: None,
            })
            .collect();
        Ok(entries)
    }

    async fn stat(&mut self, path: &str) -> anyhow::Result<FileEntry> {
        let files = self.files.lock().unwrap();
        match files.get(path) {
            Some(v) => Ok(FileEntry {
                name: path.to_string(),
                path: path.to_string(),
                is_dir: false,
                size: v.len() as u64,
                modified: None,
                permissions: None,
                storage_class: None,
                acl: None,
            }),
            None => anyhow::bail!("not found: {path}"),
        }
    }

    async fn upload(
        &mut self,
        remote_path: &str,
        data: Vec<u8>,
        _offset: u64,
        progress: Option<&ProgressCallback>,
    ) -> anyhow::Result<UploadResult> {
        let total = data.len() as u64;
        self.files
            .lock()
            .unwrap()
            .insert(remote_path.to_string(), data);
        if let Some(cb) = progress {
            cb(total, total);
        }
        Ok(UploadResult::default())
    }

    async fn download(&mut self, path: &str) -> anyhow::Result<Vec<u8>> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("not found: {path}"))
    }

    async fn download_range(
        &mut self,
        path: &str,
        offset: u64,
        limit: Option<u64>,
    ) -> anyhow::Result<Vec<u8>> {
        let data = self.download(path).await?;
        let end = limit
            .map(|l| (offset + l).min(data.len() as u64))
            .unwrap_or(data.len() as u64);
        Ok(data
            .into_iter()
            .skip(offset as usize)
            .take((end - offset) as usize)
            .collect())
    }

    async fn delete(&mut self, path: &str) -> anyhow::Result<()> {
        self.files.lock().unwrap().remove(path);
        Ok(())
    }

    async fn rename(&mut self, from: &str, to: &str) -> anyhow::Result<()> {
        let mut files = self.files.lock().unwrap();
        let data = files
            .remove(from)
            .ok_or_else(|| anyhow::anyhow!("not found: {from}"))?;
        files.insert(to.to_string(), data);
        Ok(())
    }

    async fn mkdir(&mut self, path: &str) -> anyhow::Result<()> {
        self.files
            .lock()
            .unwrap()
            .insert(format!("{path}/"), Vec::new());
        Ok(())
    }

    async fn read_for_edit(&mut self, path: &str) -> anyhow::Result<Vec<u8>> {
        self.download(path).await
    }

    async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> anyhow::Result<()> {
        self.files.lock().unwrap().insert(path.to_string(), data);
        Ok(())
    }

    async fn close(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_count_basic() {
        assert_eq!(part_count(0, DEFAULT_PART_SIZE), 1);
        assert_eq!(part_count(1, DEFAULT_PART_SIZE), 1);
        assert_eq!(part_count(DEFAULT_PART_SIZE, DEFAULT_PART_SIZE), 1);
        assert_eq!(part_count(DEFAULT_PART_SIZE + 1, DEFAULT_PART_SIZE), 2);
        assert_eq!(part_count(DEFAULT_PART_SIZE * 3, DEFAULT_PART_SIZE), 3);
    }

    #[tokio::test]
    async fn mem_connector_upload_download_roundtrip() {
        let mut c = MemConnector::default();
        let data = vec![1u8, 2, 3, 4];
        c.upload("a.txt", data.clone(), 0, None).await.unwrap();
        let got = c.download("a.txt").await.unwrap();
        assert_eq!(got, data);
    }

    #[tokio::test]
    async fn mem_connector_download_range() {
        let mut c = MemConnector::default();
        c.upload("a.bin", vec![10, 20, 30, 40, 50], 0, None)
            .await
            .unwrap();
        let mid = c.download_range("a.bin", 1, Some(2)).await.unwrap();
        assert_eq!(mid, vec![20, 30]);
        let tail = c.download_range("a.bin", 3, None).await.unwrap();
        assert_eq!(tail, vec![40, 50]);
    }

    #[tokio::test]
    async fn mem_connector_rename_and_delete() {
        let mut c = MemConnector::default();
        c.upload("old", vec![9], 0, None).await.unwrap();
        c.rename("old", "new").await.unwrap();
        assert!(c.download("old").await.is_err());
        assert_eq!(c.download("new").await.unwrap(), vec![9]);
        c.delete("new").await.unwrap();
        assert!(c.download("new").await.is_err());
    }

    #[tokio::test]
    async fn mem_connector_list_filters_by_prefix() {
        let mut c = MemConnector::default();
        c.upload("dir/a", vec![1], 0, None).await.unwrap();
        c.upload("dir/b", vec![2], 0, None).await.unwrap();
        c.upload("other/c", vec![3], 0, None).await.unwrap();
        let entries = c.list("dir/").await.unwrap();
        let mut names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["dir/a", "dir/b"]);
    }

    #[test]
    fn upload_result_default_has_no_upload_id() {
        assert!(UploadResult::default().upload_id.is_none());
    }
}
