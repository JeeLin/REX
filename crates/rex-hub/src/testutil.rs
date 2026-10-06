//! 测试夹具：内存 FileConnector + 测试用 AppState。
//!
//! transfer / sync 两个协调器的测试各需要一份可控的内存后端。历史上两份夹具
//! （`transfer_coordinator::MockConnector` 与 `sync_coordinator::TreeConnector`）
//! 主体逐行同构，故合并为这一份 `TreeConnector`——它是二者的严格超集
//! （多出目录感知的 `list` / `stat` 的目录判定 / `mkdir`）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rex_common::file_transfer::{FileConnector, FileEntry, ProgressCallback, UploadResult};

use crate::AppState;

/// 取路径末段文件名（部分 SFTP/S3 列表返回全路径）。
fn file_name(name: &str) -> String {
    name.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(name)
        .to_string()
}

/// 故障注入开关（`TreeConnector` 以 `..Default::default()` 取全 false）。
#[derive(Clone, Default)]
pub(crate) struct FailFlags {
    /// `list` 始终失败（模拟 SFTP 目录列举失败）。
    pub(crate) fail_list: bool,
    /// `upload` 始终失败。
    pub(crate) fail_upload: bool,
    /// `download_range` 始终失败。
    pub(crate) fail_download: bool,
    /// `rename` 始终失败。
    pub(crate) fail_rename: bool,
    /// `delete` 始终失败。
    pub(crate) fail_delete: bool,
    /// `stat` 始终失败。
    pub(crate) fail_stat: bool,
    /// 越界 `download_range` 返回 Err 而非空 vec —— 模拟 S3 的 416
    /// （`Range Not Satisfiable`），引擎不得依赖「越界返回空 vec」。
    pub(crate) simulate_s3_eof: bool,
}

/// 内存目录树 FileConnector：`store` 存文件，目录由文件键前缀虚拟推导。
///
/// `fail` 标志模拟底层故障；`cancel_hook` 在每次 upload 时回调（模拟
/// 并发取消，DB 由另一线程写入）。transfer / sync 两个协调器的测试共用这一份。
#[derive(Clone)]
pub(crate) struct TreeConnector {
    pub(crate) store: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    pub(crate) fail: FailFlags,
    pub(crate) cancel_hook: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Default for TreeConnector {
    fn default() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
            fail: FailFlags::default(),
            cancel_hook: None,
        }
    }
}

impl TreeConnector {
    pub(crate) fn new(store: Arc<Mutex<HashMap<String, Vec<u8>>>>) -> Self {
        Self {
            store,
            ..Default::default()
        }
    }

    /// 覆盖故障注入开关。
    pub(crate) fn with_fail(mut self, fail: FailFlags) -> Self {
        self.fail = fail;
        self
    }

    pub(crate) fn with_cancel_hook(mut self, hook: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.cancel_hook = Some(hook);
        self
    }

    /// 直接落文件（绕过引擎，用于布置测试场景）。
    pub(crate) fn put(&self, path: &str, data: &[u8]) {
        self.store
            .lock()
            .unwrap()
            .insert(path.to_string(), data.to_vec());
    }

    pub(crate) fn get(&self, path: &str) -> Option<Vec<u8>> {
        self.store.lock().unwrap().get(path).cloned()
    }

    pub(crate) fn contains(&self, path: &str) -> bool {
        self.store.lock().unwrap().contains_key(path)
    }

    fn children(&self, dir: &str) -> Vec<FileEntry> {
        let prefix = format!("{}/", dir.trim_end_matches('/'));
        let store = self.store.lock().unwrap();
        let mut seen: HashMap<String, bool> = HashMap::new();
        for key in store.keys() {
            let Some(rest) = key.strip_prefix(&prefix) else {
                continue;
            };
            let (name, rest) = match rest.split_once('/') {
                Some((n, tail)) => (n.to_string(), Some(tail)),
                None => (rest.to_string(), None),
            };
            if name.is_empty() {
                continue;
            }
            let is_dir = rest.is_some();
            seen.entry(name)
                .and_modify(|d| *d |= is_dir)
                .or_insert(is_dir);
        }
        let mut out: Vec<FileEntry> = seen
            .into_iter()
            .map(|(name, is_dir)| {
                let path = format!("{prefix}{name}");
                let size = if is_dir {
                    0
                } else {
                    // 列表条目必须携带真实尺寸，否则 diff 的「按大小比较」会被
                    // 全 0 尺寸误导（引擎按 list 结果判定 Copy/Conflict）。
                    // 注意复用上方已持有的 store 锁（std::sync::Mutex 不可重入）。
                    store.get(&path).map(|v| v.len() as u64).unwrap_or(0)
                };
                FileEntry {
                    name,
                    path,
                    is_dir,
                    size,
                    modified: None,
                    permissions: None,
                    storage_class: None,
                    acl: None,
                }
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }
}

#[async_trait]
impl FileConnector for TreeConnector {
    async fn list(&mut self, path: &str) -> anyhow::Result<Vec<FileEntry>> {
        if self.fail.fail_list {
            anyhow::bail!("list failed (simulated)");
        }
        Ok(self.children(path))
    }

    async fn stat(&mut self, path: &str) -> anyhow::Result<FileEntry> {
        if self.fail.fail_stat {
            anyhow::bail!("stat failed (simulated)");
        }
        let store = self.store.lock().unwrap();
        if let Some(v) = store.get(path) {
            return Ok(FileEntry {
                name: file_name(path),
                path: path.to_string(),
                is_dir: false,
                size: v.len() as u64,
                modified: None,
                permissions: None,
                storage_class: None,
                acl: None,
            });
        }
        let prefix = format!("{}/", path.trim_end_matches('/'));
        if store.keys().any(|k| k.starts_with(&prefix)) {
            return Ok(FileEntry {
                name: file_name(path),
                path: path.to_string(),
                is_dir: true,
                size: 0,
                modified: None,
                permissions: None,
                storage_class: None,
                acl: None,
            });
        }
        anyhow::bail!("not found: {path}")
    }

    async fn upload(
        &mut self,
        remote_path: &str,
        data: Vec<u8>,
        offset: u64,
        _progress: Option<&ProgressCallback>,
    ) -> anyhow::Result<UploadResult> {
        if let Some(hook) = &self.cancel_hook {
            hook();
        }
        if self.fail.fail_upload {
            anyhow::bail!("upload failed (simulated)");
        }
        let mut store = self.store.lock().unwrap();
        let buf = store.entry(remote_path.to_string()).or_default();
        let start = offset as usize;
        if buf.len() < start + data.len() {
            buf.resize(start + data.len(), 0);
        }
        buf[start..start + data.len()].copy_from_slice(&data);
        Ok(UploadResult::default())
    }

    async fn download(&mut self, path: &str) -> anyhow::Result<Vec<u8>> {
        self.store
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
        if self.fail.fail_download {
            anyhow::bail!("download failed (simulated)");
        }
        let data = self.download(path).await?;
        if self.fail.simulate_s3_eof && offset >= data.len() as u64 {
            anyhow::bail!("HTTP 416 Range Not Satisfiable");
        }
        let start = (offset as usize).min(data.len());
        let end = match limit {
            Some(l) => ((offset + l) as usize).min(data.len()),
            None => data.len(),
        };
        Ok(data[start..end].to_vec())
    }

    async fn delete(&mut self, path: &str) -> anyhow::Result<()> {
        if self.fail.fail_delete {
            anyhow::bail!("delete failed (simulated)");
        }
        self.store.lock().unwrap().remove(path);
        Ok(())
    }

    async fn rename(&mut self, from: &str, to: &str) -> anyhow::Result<()> {
        if self.fail.fail_rename {
            anyhow::bail!("rename failed (simulated)");
        }
        let mut store = self.store.lock().unwrap();
        let data = store
            .remove(from)
            .ok_or_else(|| anyhow::anyhow!("not found: {from}"))?;
        store.insert(to.to_string(), data);
        Ok(())
    }

    async fn mkdir(&mut self, _path: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn read_for_edit(&mut self, path: &str) -> anyhow::Result<Vec<u8>> {
        self.download(path).await
    }

    async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> anyhow::Result<()> {
        self.store.lock().unwrap().insert(path.to_string(), data);
        Ok(())
    }

    async fn close(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// 建一份临时 DB + AppState（任务记录落在这里）。
pub(crate) fn make_state() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::resource_conn::build_test_state(dir.path());
    (dir, state)
}
