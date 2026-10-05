//! 目录同步引擎 — v0.92.0 子任务 2（compare → diff → apply）。
//!
//! 引擎在服务端进程内驱动两侧目录树：`FileConnector::list` 递归扫描 → `rex_transfer::diff`
//! 生成 [`SyncPlan`] → 逐 action 直连搬运/删除。**文件数据只在 source-connector 与
//! target-connector 之间流式传输，不经过浏览器**（与 `TransferCoordinator` 同一纪律）。
//!
//! 状态机：`pending → scanning → planning → running → verifying → completed / failed`，
//! 取消沿袭 v0.91.0 规范——每阶段先轮询 DB `canceled` 标记，错误落 `failed`
//! 但绝不覆盖已被取消的任务（Bug 1/2 修复范式）。
//!
//! 掩码（include / exclude）在扫描期即生效：`exclude` 命中的子树不再下钻，
//! 被排除的文件既不复制也不参与孤儿判定。

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use rex_common::file_transfer::FileConnector;
use rex_transfer::{
    ancestor_dirs, diff, mask_matches, SyncActionDir, SyncActionKind, SyncEntry, SyncOptions,
    SyncPlan, TransferStatus,
};

use crate::app::{AppState, TransferProgressEvent};
use crate::transfer_coordinator::{CHUNK_SIZE, TEMP_SUFFIX};

/// 扫描时的最大递归深度，防御环形目录/软链导致的无限下钻。
const MAX_SCAN_DEPTH: usize = 32;

/// 同步引擎错误。持久化时写入 `transfer_task.error`（状态 `failed`）。
#[derive(Debug)]
pub enum SyncError {
    /// 任务记录不存在。
    TaskNotFound,
    /// DB 读写失败。
    Db(String),
    /// `sync_options` 列不是合法 `SyncOptions` JSON。
    InvalidOptions(String),
    /// `conflict_policy` 列非法。
    InvalidConflictPolicy(String),
    /// 连接资源失败。
    Connect(String),
    /// 目录扫描失败。
    Scan(String),
    /// 复制分片下载失败。
    Download(String),
    /// 复制分片上传失败。
    Upload(String),
    /// temp → 正式路径 rename 失败。
    Rename(String),
    /// 删除孤儿失败。
    Delete(String),
    /// 校验不一致（尺寸不匹配）。
    Verify(String),
    /// 目标已存在且冲突策略为 `fail`。
    AlreadyExists(String),
    /// 任务被取消。
    Canceled,
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskNotFound => write!(f, "sync task not found"),
            Self::Db(e) => write!(f, "db error: {e}"),
            Self::InvalidOptions(e) => write!(f, "invalid sync options: {e}"),
            Self::InvalidConflictPolicy(p) => write!(f, "invalid conflict policy: {p}"),
            Self::Connect(e) => write!(f, "connect resource failed: {e}"),
            Self::Scan(e) => write!(f, "scan failed: {e}"),
            Self::Download(e) => write!(f, "download failed: {e}"),
            Self::Upload(e) => write!(f, "upload failed: {e}"),
            Self::Rename(e) => write!(f, "rename failed: {e}"),
            Self::Delete(e) => write!(f, "delete failed: {e}"),
            Self::Verify(e) => write!(f, "verify failed: {e}"),
            Self::AlreadyExists(p) => write!(f, "destination already exists: {p}"),
            Self::Canceled => write!(f, "sync canceled"),
        }
    }
}

impl std::error::Error for SyncError {}

/// 一次同步的静态参数：同步选项 + 冲突策略。
/// 打包为一个结构体以避免 `run_plan` 超过 clippy 的参数数上限。
#[derive(Debug, Clone, Copy)]
pub struct SyncSpec<'a> {
    pub opts: &'a SyncOptions,
    pub conflict: rex_transfer::ConflictPolicy,
}

/// 目录同步协调器。`Arc` 封装以允许 `submit` 克隆自身进入后台任务。
#[derive(Clone)]
pub struct SyncCoordinator {
    pub handles: Arc<std::sync::Mutex<HashMap<String, tokio::task::AbortHandle>>>,
}

impl Default for SyncCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncCoordinator {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// 提交同步任务：后台驱动 [`SyncCoordinator::run`]，并注册 AbortHandle 供
    /// [`SyncCoordinator::abort`] 中止。
    pub fn submit(self: &Arc<Self>, state: AppState, task_id: String) {
        let coord = Arc::clone(self);
        let tid = task_id.clone();
        let handle = tokio::spawn(async move {
            let r = Self::run(&state, &tid).await;
            coord.handles.lock().unwrap().remove(&tid);
            r
        });
        self.handles
            .lock()
            .unwrap()
            .insert(task_id, handle.abort_handle());
    }

    /// 中止同步任务的后台任务。返回是否命中活跃句柄。
    ///
    /// 仅中止进程内任务；DB 状态仍由 `cancel_task` 持久化为 `canceled`，
    /// 引擎各阶段亦轮询 DB 状态作为兜底。
    pub fn abort(&self, task_id: &str) -> bool {
        match self.handles.lock().unwrap().remove(task_id) {
            Some(handle) => {
                handle.abort();
                true
            }
            None => false,
        }
    }

    /// 驱动一次完整同步：加载任务记录 → 打开 source/target 连接器 → `run_plan`。
    pub async fn run(state: &AppState, task_id: &str) -> Result<SyncPlan, SyncError> {
        let task = state
            .db
            .get_transfer_task(task_id)
            .map_err(|e| SyncError::Db(e.to_string()))?
            .ok_or(SyncError::TaskNotFound)?;

        let opts: SyncOptions = serde_json::from_str(&task.sync_options)
            .map_err(|e| SyncError::InvalidOptions(e.to_string()))?;
        let conflict = rex_transfer::ConflictPolicy::from_str(&task.conflict_policy)
            .ok_or_else(|| SyncError::InvalidConflictPolicy(task.conflict_policy.clone()))?;

        let mut source = crate::file_api::connect_resource(state, &task.source_resource_id)
            .await
            .map_err(|e| SyncError::Connect(e.to_string()))?;
        let mut target = crate::file_api::connect_resource(state, &task.target_resource_id)
            .await
            .map_err(|e| SyncError::Connect(e.to_string()))?;

        let res = Self::run_plan(
            state,
            task_id,
            SyncSpec {
                opts: &opts,
                conflict,
            },
            &mut *source,
            &mut *target,
            &task.source_path,
            &task.target_path,
        )
        .await;

        let _ = source.close().await;
        let _ = target.close().await;
        res
    }

    /// 同步内核（不关心连接器来历，仅依赖 `FileConnector` trait）。
    ///
    /// scanning → planning → running → verifying → completed。
    pub async fn run_plan(
        state: &AppState,
        task_id: &str,
        spec: SyncSpec<'_>,
        source: &mut dyn FileConnector,
        target: &mut dyn FileConnector,
        source_root: &str,
        target_root: &str,
    ) -> Result<SyncPlan, SyncError> {
        let opts = spec.opts;
        if Self::is_canceled(state, task_id) {
            Self::set_status(state, task_id, TransferStatus::Canceled);
            return Err(SyncError::Canceled);
        }

        // --- scanning：递归列两侧文件清单 ---
        Self::set_status(state, task_id, TransferStatus::Scanning);
        let (source_entries, target_entries) = match (
            Self::scan_tree(state, task_id, source, source_root, opts).await,
            Self::scan_tree(state, task_id, target, target_root, opts).await,
        ) {
            (Ok(s), Ok(t)) => (s, t),
            (Err(e), _) | (_, Err(e)) => {
                Self::fail(state, task_id, &e);
                return Err(e);
            }
        };

        // --- planning：纯函数 diff ---
        Self::set_status(state, task_id, TransferStatus::Planning);
        let plan = diff(&source_entries, &target_entries, opts);
        state
            .db
            .update_transfer_task_progress(task_id, plan.summary.total_bytes, 0, 0, None)
            .ok();

        // --- running：逐 action 直连搬运 / 删除 ---
        Self::set_status(state, task_id, TransferStatus::Running);
        let mut transferred: u64 = 0;
        let mut made_dirs: Vec<String> = Vec::new();
        for action in &plan.actions {
            if Self::is_canceled(state, task_id) {
                Self::set_status(state, task_id, TransferStatus::Canceled);
                return Err(SyncError::Canceled);
            }

            let (dest_root, src_root) = match action.dir {
                SyncActionDir::ToTarget => (target_root, source_root),
                SyncActionDir::ToSource => (source_root, target_root),
            };
            let to = join_path(dest_root, &action.rel_path);

            // 删除孤儿：仅删文件，不递归删目录（避免误删掩码保留的目录内容）。
            if action.action == SyncActionKind::Delete {
                let res = match action.dir {
                    SyncActionDir::ToTarget => target.delete(&to).await,
                    SyncActionDir::ToSource => source.delete(&to).await,
                };
                if let Err(e) = res {
                    let err = SyncError::Delete(format!("{to}: {e}"));
                    Self::fail(state, task_id, &err);
                    return Err(err);
                }
                continue;
            }

            let from = join_path(src_root, &action.rel_path);

            for dir in ancestor_dirs(&action.rel_path) {
                if made_dirs.contains(&dir) {
                    continue;
                }
                let dir_path = join_path(dest_root, &dir);
                let res = match action.dir {
                    SyncActionDir::ToTarget => target.mkdir(&dir_path).await,
                    SyncActionDir::ToSource => source.mkdir(&dir_path).await,
                };
                if let Err(e) = res {
                    // mkdir 失败多为「目录已存在」：非幂等 connector 会报错，但这不应
                    // 中断同步——真正写文件失败时才致命。
                    tracing::debug!(dir = %dir_path, error = %e, "sync mkdir failed (ignored)");
                }
                made_dirs.push(dir);
            }

            // reader 读 `from`，writer 写 `to` — 按动作方向切换两侧连接器。
            let copy = match action.dir {
                SyncActionDir::ToTarget => {
                    Self::copy_file(state, task_id, source, target, &from, &to, spec.conflict)
                }
                SyncActionDir::ToSource => {
                    Self::copy_file(state, task_id, target, source, &from, &to, spec.conflict)
                }
            };
            match copy.await {
                Ok(done) => {
                    transferred += done;
                    state
                        .db
                        .update_transfer_task_progress(
                            task_id,
                            plan.summary.total_bytes,
                            transferred,
                            0,
                            None,
                        )
                        .ok();
                    Self::broadcast_progress(
                        state,
                        task_id,
                        TransferStatus::Running.as_str_lossy(),
                        plan.summary.total_bytes,
                        transferred,
                    );
                }
                Err(e) => {
                    Self::fail(state, task_id, &e);
                    return Err(e);
                }
            }
        }

        // --- verifying → completed ---
        Self::set_status(state, task_id, TransferStatus::Verifying);
        if Self::is_canceled(state, task_id) {
            Self::set_status(state, task_id, TransferStatus::Canceled);
            return Err(SyncError::Canceled);
        }
        Self::set_status(state, task_id, TransferStatus::Completed);
        Ok(plan)
    }

    /// 递归扫描一侧目录树，返回相对根的文件清单（目录不进清单）。
    ///
    /// 显式工作栈（而非递归 async fn，避免无限大的 future），`exclude` 命中的
    /// 子树不再下钻——掩码在扫描期即生效。
    async fn scan_tree(
        state: &AppState,
        task_id: &str,
        conn: &mut dyn FileConnector,
        root: &str,
        opts: &SyncOptions,
    ) -> Result<Vec<SyncEntry>, SyncError> {
        let mut out = Vec::new();
        let mut stack: Vec<(String, usize)> = vec![(String::new(), 0)];

        while let Some((rel_dir, depth)) = stack.pop() {
            if depth > MAX_SCAN_DEPTH {
                return Err(SyncError::Scan(format!(
                    "directory tree too deep at {rel_dir}"
                )));
            }
            if Self::is_canceled(state, task_id) {
                return Err(SyncError::Canceled);
            }

            let abs = join_path(root, &rel_dir);
            let entries = conn
                .list(&abs)
                .await
                .map_err(|e| SyncError::Scan(format!("list {abs} failed: {e}")))?;

            for entry in entries {
                if Self::is_canceled(state, task_id) {
                    return Err(SyncError::Canceled);
                }
                let name = file_name(&entry.name);
                if name == "." || name == ".." || name.is_empty() {
                    continue;
                }
                let rel = if rel_dir.is_empty() {
                    name
                } else {
                    format!("{rel_dir}/{name}")
                };
                // exclude 命中则整棵子树跳过（include 的最终裁剪交给 diff 侧，
                // 否则 `src/**` 这类掩码会让整个子树无法下钻）。
                if opts.exclude.iter().any(|m| mask_matches(&rel, m)) {
                    continue;
                }
                if entry.is_dir {
                    stack.push((rel, depth + 1));
                } else {
                    out.push(SyncEntry::new(
                        rel,
                        entry.size,
                        parse_mtime(entry.modified.as_deref()),
                    ));
                }
            }
        }
        out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(out)
    }

    /// 直连复制单个文件：temp → stat 尺寸校验 → rename，返回搬运字节数。
    ///
    /// 字节从 `reader` 的 `download_range` 读出写入 `writer` 的 `upload`，
    /// 不经过浏览器。读写两侧由动作方向决定（`ToSource` 时互换）。
    async fn copy_file(
        state: &AppState,
        task_id: &str,
        reader: &mut dyn FileConnector,
        writer: &mut dyn FileConnector,
        from: &str,
        to: &str,
        conflict: rex_transfer::ConflictPolicy,
    ) -> Result<u64, SyncError> {
        let total = match reader.stat(from).await {
            Ok(entry) => entry.size,
            Err(e) => return Err(SyncError::Scan(format!("stat {from} failed: {e}"))),
        };

        let final_dst = match conflict {
            rex_transfer::ConflictPolicy::Skip if writer.stat(to).await.is_ok() => {
                return Ok(total)
            }
            rex_transfer::ConflictPolicy::Fail if writer.stat(to).await.is_ok() => {
                return Err(SyncError::AlreadyExists(to.to_string()))
            }
            rex_transfer::ConflictPolicy::Rename if writer.stat(to).await.is_ok() => {
                unique_name(writer, to).await?
            }
            _ => to.to_string(),
        };
        let temp = format!("{final_dst}{TEMP_SUFFIX}");

        let mut offset: u64 = 0;
        loop {
            if Self::is_canceled(state, task_id) {
                let _ = writer.delete(&temp).await;
                return Err(SyncError::Canceled);
            }
            // 在请求超出文件范围前短路：避免 S3 之类返回 416（Range Not
            // Satisfiable）的 connector。零字节文件只落一个空分片，让 temp
            // 真正在目标侧生成，后续 stat / rename 才有对象可依。
            if offset >= total {
                if total == 0 {
                    writer
                        .upload(&temp, Vec::new(), 0, None)
                        .await
                        .map_err(|e| SyncError::Upload(format!("{temp}: {e}")))?;
                }
                break;
            }

            let chunk = reader
                .download_range(from, offset, Some(CHUNK_SIZE))
                .await
                .map_err(|e| SyncError::Download(format!("{from}: {e}")))?;
            if chunk.is_empty() {
                break;
            }
            let chunk_len = chunk.len() as u64;
            writer
                .upload(&temp, chunk, offset, None)
                .await
                .map_err(|e| SyncError::Upload(format!("{temp}: {e}")))?;
            offset += chunk_len;
        }

        // 最后一片之后、verifying 之前再查一次取消，避免 clobber DB 中的 canceled。
        if Self::is_canceled(state, task_id) {
            let _ = writer.delete(&temp).await;
            return Err(SyncError::Canceled);
        }

        let written = match writer.stat(&temp).await {
            Ok(entry) => entry.size,
            Err(e) => {
                let _ = writer.delete(&temp).await;
                return Err(SyncError::Scan(format!("stat {temp} failed: {e}")));
            }
        };
        if written != total {
            let _ = writer.delete(&temp).await;
            return Err(SyncError::Verify(format!(
                "size mismatch: src={total} dst={written}"
            )));
        }

        if let Err(e) = writer.rename(&temp, &final_dst).await {
            let _ = writer.delete(&temp).await;
            return Err(SyncError::Rename(format!("{temp}: {e}")));
        }
        Ok(total)
    }

    fn is_canceled(state: &AppState, task_id: &str) -> bool {
        state
            .db
            .get_transfer_task(task_id)
            .ok()
            .flatten()
            .map(|r| r.status == "canceled")
            .unwrap_or(false)
    }

    /// 错误落 `failed`——但已被取消的任务不得被覆盖（Bug 1/2 范式）。
    fn fail(state: &AppState, task_id: &str, err: &SyncError) {
        if matches!(err, SyncError::Canceled) || Self::is_canceled(state, task_id) {
            return;
        }
        let msg = err.to_string();
        let _ = state.db.set_transfer_task_status(
            task_id,
            TransferStatus::Failed(msg.clone()).as_str_lossy(),
            Some(&msg),
        );
        Self::broadcast_progress(
            state,
            task_id,
            TransferStatus::Failed(msg.clone()).as_str_lossy(),
            0,
            0,
        );
        tracing::error!(action = "FILE_SYNC_FAILED", transfer_task_id = %task_id, error = %msg, "sync task failed");
    }

    fn set_status(state: &AppState, task_id: &str, status: TransferStatus) {
        let _ = state
            .db
            .set_transfer_task_status(task_id, status.as_str_lossy(), None);
        if let Ok(Some(rec)) = state.db.get_transfer_task(task_id) {
            Self::broadcast_progress(
                state,
                task_id,
                status.as_str_lossy(),
                rec.total_bytes.max(0) as u64,
                rec.transferred_bytes.max(0) as u64,
            );
        }
    }

    fn broadcast_progress(
        state: &AppState,
        task_id: &str,
        status: &str,
        total: u64,
        transferred: u64,
    ) {
        let _ = state.transfer_bcast.send(TransferProgressEvent {
            task_id: task_id.to_string(),
            transferred_bytes: transferred,
            total_bytes: total,
            speed_bytes_per_sec: 0,
            status: status.to_string(),
        });
    }
}

/// 取路径末段文件名（部分 SFTP/S3 列表返回全路径）。
fn file_name(name: &str) -> String {
    name.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(name)
        .to_string()
}

/// 根 + 相对目录（两侧根同构）。
fn join_path(root: &str, rel_dir: &str) -> String {
    if rel_dir.is_empty() {
        return root.to_string();
    }
    format!("{}/{}", root.trim_end_matches('/'), rel_dir)
}

/// 为 `Rename` 冲突生成不存在的目标名：`name (1).ext` ...
async fn unique_name(target: &mut dyn FileConnector, dst_path: &str) -> Result<String, SyncError> {
    let (stem, ext) = match dst_path.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() && !s.ends_with('/') => (s.to_string(), format!(".{e}")),
        _ => (dst_path.to_string(), String::new()),
    };
    for i in 1..1000u32 {
        let candidate = format!("{stem} ({i}){ext}");
        if target.stat(&candidate).await.is_err() {
            return Ok(candidate);
        }
    }
    Err(SyncError::AlreadyExists(dst_path.to_string()))
}

/// `FileEntry.modified` → Unix 秒。S3 给 unix 秒字符串，SFTP 给
/// `%Y-%m-%d %H:%M:%S`（UTC 解析）；无法解析返回 `None`（比较退化为仅比大小）。
fn parse_mtime(raw: Option<&str>) -> Option<i64> {
    let s = raw?.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(secs) = s.parse::<i64>() {
        return Some(secs);
    }
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|dt| dt.and_utc().timestamp())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use rex_common::file_transfer::{FileEntry, ProgressCallback, UploadResult};
    use rex_transfer::ConflictPolicy;

    use super::*;

    /// 内存目录树 FileConnector：`store` 存文件，目录由文件键前缀虚拟推导。
    ///
    /// `fail_*` 标志模拟底层故障；`cancel_hook` 在每次 upload 时回调（模拟
    /// 并发取消，DB 由另一线程写入）。
    #[derive(Clone, Default)]
    struct TreeConnector {
        store: Arc<Mutex<HashMap<String, Vec<u8>>>>,
        fail_list: bool,
        fail_upload: bool,
        fail_download: bool,
        cancel_hook: Option<Arc<dyn Fn() + Send + Sync>>,
    }

    impl TreeConnector {
        fn new(store: Arc<Mutex<HashMap<String, Vec<u8>>>>) -> Self {
            Self {
                store,
                ..Default::default()
            }
        }

        fn with_list_failure(mut self) -> Self {
            self.fail_list = true;
            self
        }

        fn with_upload_failure(mut self) -> Self {
            self.fail_upload = true;
            self
        }

        fn with_cancel_hook(mut self, hook: Arc<dyn Fn() + Send + Sync>) -> Self {
            self.cancel_hook = Some(hook);
            self
        }

        /// 直接落文件（绕过引擎，用于布置测试场景）。
        fn put(&self, path: &str, data: &[u8]) {
            self.store
                .lock()
                .unwrap()
                .insert(path.to_string(), data.to_vec());
        }

        fn get(&self, path: &str) -> Option<Vec<u8>> {
            self.store.lock().unwrap().get(path).cloned()
        }

        fn contains(&self, path: &str) -> bool {
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
            if self.fail_list {
                anyhow::bail!("list failed (simulated)");
            }
            Ok(self.children(path))
        }

        async fn stat(&mut self, path: &str) -> anyhow::Result<FileEntry> {
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
            if self.fail_upload {
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
            if self.fail_download {
                anyhow::bail!("download failed (simulated)");
            }
            let data = self.download(path).await?;
            let start = (offset as usize).min(data.len());
            let end = match limit {
                Some(l) => ((offset + l) as usize).min(data.len()),
                None => data.len(),
            };
            Ok(data[start..end].to_vec())
        }

        async fn delete(&mut self, path: &str) -> anyhow::Result<()> {
            self.store.lock().unwrap().remove(path);
            Ok(())
        }

        async fn rename(&mut self, from: &str, to: &str) -> anyhow::Result<()> {
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

    fn make_state() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());
        (dir, state)
    }

    fn make_sync_task(state: &AppState, opts: &SyncOptions) -> String {
        state
            .db
            .create_sync_task(&crate::models::NewSyncTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src".into(),
                target_path: "/dst".into(),
                sync_options: serde_json::to_string(opts).unwrap(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap()
    }

    #[tokio::test]
    async fn run_plan_upload_copies_files_and_completes() {
        let (_dir, state) = make_state();
        let task_id = make_sync_task(&state, &SyncOptions::default());

        let src = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        let dst = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        src.put("/src/new.txt", b"hello");
        src.put("/src/deep/nested.txt", b"nested");
        src.put("/src/empty.txt", b"");
        dst.put("/src-stale.txt", b"stale");
        dst.put("/dst/keep.txt", b"keep");

        let mut source = src.clone();
        let mut target = dst.clone();

        let plan = SyncCoordinator::run_plan(
            &state,
            &task_id,
            SyncSpec {
                opts: &SyncOptions::default(),
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src",
            "/dst",
        )
        .await
        .expect("sync must complete");

        assert_eq!(plan.summary.copies, 3);
        assert_eq!(plan.summary.deletes, 0);
        assert_eq!(dst.get("/dst/new.txt"), Some(b"hello".to_vec()));
        assert_eq!(dst.get("/dst/deep/nested.txt"), Some(b"nested".to_vec()));
        assert_eq!(dst.get("/dst/empty.txt"), Some(Vec::new()));
        assert!(dst.contains("/dst/keep.txt"), "unrelated file survives");
        assert!(
            !dst.contains("/dst/new.txt.rex.part"),
            "temp file must be renamed away"
        );

        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "completed");
        assert_eq!(rec.total_bytes, plan.summary.total_bytes as i64);
        assert_eq!(rec.transferred_bytes, plan.summary.total_bytes as i64);
    }

    #[tokio::test]
    async fn run_plan_upload_deletes_orphans_when_enabled() {
        let (_dir, state) = make_state();
        let opts = SyncOptions {
            delete_orphans: true,
            ..Default::default()
        };
        let task_id = make_sync_task(&state, &opts);

        let src = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        let dst = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        src.put("/src/keep.txt", b"same");
        dst.put("/dst/keep.txt", b"same");
        dst.put("/dst/orphan.txt", b"orphan");
        dst.put("/dst/nested/gone.txt", b"gone");

        let mut source = src.clone();
        let mut target = dst.clone();

        let plan = SyncCoordinator::run_plan(
            &state,
            &task_id,
            SyncSpec {
                opts: &opts,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src",
            "/dst",
        )
        .await
        .expect("sync must complete");

        assert_eq!(plan.summary.deletes, 2);
        assert!(!dst.contains("/dst/orphan.txt"), "orphan deleted");
        assert!(
            !dst.contains("/dst/nested/gone.txt"),
            "nested orphan deleted"
        );
        assert!(dst.contains("/dst/keep.txt"), "present file kept");
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "completed");
    }

    #[tokio::test]
    async fn run_plan_masks_prune_scan_and_protect_excluded_files() {
        let (_dir, state) = make_state();
        let opts = SyncOptions {
            exclude: vec!["*.log".into(), "vendor".into()],
            delete_orphans: true,
            ..Default::default()
        };
        let task_id = make_sync_task(&state, &opts);

        let src = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        let dst = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        src.put("/src/app.rs", b"fn main() {}");
        src.put("/src/debug.log", b"ignored");
        dst.put("/dst/vendor/lib.js", b"lib");
        dst.put("/dst/leftover.txt", b"orphan");

        let mut source = src.clone();
        let mut target = dst.clone();

        let plan = SyncCoordinator::run_plan(
            &state,
            &task_id,
            SyncSpec {
                opts: &opts,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src",
            "/dst",
        )
        .await
        .expect("sync must complete");

        assert_eq!(
            plan.actions
                .iter()
                .map(|a| a.rel_path.as_str())
                .collect::<Vec<_>>(),
            vec!["app.rs", "leftover.txt"],
            "excluded files must not appear in the plan"
        );
        assert_eq!(dst.get("/dst/app.rs"), Some(b"fn main() {}".to_vec()));
        assert!(
            !dst.contains("/dst/debug.log"),
            "excluded source file must not be copied"
        );
        assert!(
            dst.contains("/dst/vendor/lib.js"),
            "excluded target file must survive orphan deletion"
        );
        assert!(
            !dst.contains("/dst/leftover.txt"),
            "non-excluded orphan deleted"
        );
    }

    #[tokio::test]
    async fn run_plan_download_copies_target_to_source() {
        let (_dir, state) = make_state();
        let opts = SyncOptions {
            direction: rex_transfer::SyncDirection::Download,
            ..Default::default()
        };
        let task_id = make_sync_task(&state, &opts);

        let src = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        let dst = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        src.put("/src/old.txt", b"old");
        dst.put("/dst/fresh.txt", b"fresh");

        let mut source = src.clone();
        let mut target = dst.clone();

        let plan = SyncCoordinator::run_plan(
            &state,
            &task_id,
            SyncSpec {
                opts: &opts,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src",
            "/dst",
        )
        .await
        .expect("sync must complete");

        assert_eq!(plan.summary.copies, 1);
        assert_eq!(src.get("/src/fresh.txt"), Some(b"fresh".to_vec()));
        assert_eq!(dst.get("/dst/fresh.txt"), Some(b"fresh".to_vec()));
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "completed");
    }

    #[tokio::test]
    async fn run_plan_bidirectional_conflict_copies_newer_side() {
        let (_dir, state) = make_state();
        let opts = SyncOptions {
            direction: rex_transfer::SyncDirection::Bidirectional,
            ..Default::default()
        };
        let task_id = make_sync_task(&state, &opts);

        let src = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        let dst = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        src.put("/src/conflict.txt", b"target-newer");
        src.put("/src/only-src.txt", b"only-src");
        dst.put("/dst/conflict.txt", b"t");
        dst.put("/dst/only-tgt.txt", b"only-tgt");

        let mut source = src.clone();
        let mut target = dst.clone();

        let plan = SyncCoordinator::run_plan(
            &state,
            &task_id,
            SyncSpec {
                opts: &opts,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src",
            "/dst",
        )
        .await
        .expect("sync must complete");

        // 两侧时间都未知 → newer_side 以源侧为准 → 源覆盖目标
        let conflict = plan
            .actions
            .iter()
            .find(|a| a.rel_path == "conflict.txt")
            .expect("conflict planned");
        assert_eq!(conflict.action, SyncActionKind::Conflict);
        assert_eq!(conflict.dir, SyncActionDir::ToTarget);
        assert_eq!(
            target.get("/dst/conflict.txt"),
            Some(b"target-newer".to_vec()),
            "conflict resolved toward the newer (source) side"
        );
        assert_eq!(source.get("/src/only-tgt.txt"), Some(b"only-tgt".to_vec()));
        assert_eq!(target.get("/dst/only-src.txt"), Some(b"only-src".to_vec()));
        assert_eq!(plan.summary.deletes, 0, "bidirectional never deletes");
    }

    #[tokio::test]
    async fn run_plan_pre_canceled_returns_canceled_without_touching_files() {
        let (_dir, state) = make_state();
        let task_id = make_sync_task(&state, &SyncOptions::default());
        state
            .db
            .set_transfer_task_status(&task_id, "canceled", None)
            .unwrap();

        let src = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        let dst = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        src.put("/src/a.txt", b"a");

        let mut source = src.clone();
        let mut target = dst.clone();

        let res = SyncCoordinator::run_plan(
            &state,
            &task_id,
            SyncSpec {
                opts: &SyncOptions::default(),
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src",
            "/dst",
        )
        .await;

        assert!(matches!(res, Err(SyncError::Canceled)));
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "canceled", "canceled must not be clobbered");
        assert!(!dst.contains("/dst/a.txt"), "no bytes moved after cancel");
    }

    #[tokio::test]
    async fn run_plan_scan_failure_sets_failed_with_error() {
        let (_dir, state) = make_state();
        let task_id = make_sync_task(&state, &SyncOptions::default());

        let src = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        let dst = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));

        let mut source = src.with_list_failure();
        let mut target = dst.clone();

        let res = SyncCoordinator::run_plan(
            &state,
            &task_id,
            SyncSpec {
                opts: &SyncOptions::default(),
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src",
            "/dst",
        )
        .await;

        assert!(matches!(res, Err(SyncError::Scan(_))));
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "failed");
        assert!(rec.error.is_some(), "failed status must carry an error");
    }

    #[tokio::test]
    async fn run_plan_upload_failure_sets_failed() {
        let (_dir, state) = make_state();
        let task_id = make_sync_task(&state, &SyncOptions::default());

        let src = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        let dst = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        src.put("/src/a.bin", b"payload");

        let mut source = src.clone();
        let mut target = dst.with_upload_failure();

        let res = SyncCoordinator::run_plan(
            &state,
            &task_id,
            SyncSpec {
                opts: &SyncOptions::default(),
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src",
            "/dst",
        )
        .await;

        assert!(matches!(res, Err(SyncError::Upload(_))));
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "failed");
        assert!(rec.error.is_some());
    }

    #[tokio::test]
    async fn run_plan_cancel_during_upload_keeps_canceled() {
        let (_dir, state) = make_state();
        let task_id = make_sync_task(&state, &SyncOptions::default());

        let src = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        src.put("/src/big.bin", b"0123456789");
        let dst = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));

        let cancel_state = state.clone();
        let cancel_tid = task_id.clone();
        let hook: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = cancel_state
                .db
                .set_transfer_task_status(&cancel_tid, "canceled", None);
        });

        let mut source = src.clone();
        let mut target = dst.clone().with_cancel_hook(hook);

        let res = SyncCoordinator::run_plan(
            &state,
            &task_id,
            SyncSpec {
                opts: &SyncOptions::default(),
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src",
            "/dst",
        )
        .await;

        assert!(
            matches!(res, Err(SyncError::Canceled)),
            "cancel during copy must abort with Canceled, got {res:?}"
        );
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(
            rec.status, "canceled",
            "canceled must not be clobbered to failed/completed"
        );
    }

    #[tokio::test]
    async fn run_plan_conflict_policy_skip_leaves_target_intact() {
        let (_dir, state) = make_state();
        let opts = SyncOptions::default();
        let task_id = make_sync_task(&state, &opts);

        let src = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        let dst = TreeConnector::new(Arc::new(Mutex::new(HashMap::new())));
        src.put("/src/a.txt", b"source version");
        // 尺寸必须不同才会被计划为 Copy（mtime 两侧均未知时按大小比较）
        dst.put("/dst/a.txt", b"older target version");

        let mut source = src.clone();
        let mut target = dst.clone();

        let plan = SyncCoordinator::run_plan(
            &state,
            &task_id,
            SyncSpec {
                opts: &opts,
                conflict: ConflictPolicy::Skip,
            },
            &mut source,
            &mut target,
            "/src",
            "/dst",
        )
        .await
        .expect("skip must not fail the run");

        assert_eq!(plan.summary.copies, 1, "still planned as a copy");
        assert_eq!(
            dst.get("/dst/a.txt"),
            Some(b"older target version".to_vec()),
            "skip keeps the existing target file"
        );
        assert!(!dst.contains("/dst/a.txt.rex.part"), "no temp left behind");
    }

    #[tokio::test]
    async fn run_missing_task_returns_task_not_found() {
        let (_dir, state) = make_state();
        let res = SyncCoordinator::run(&state, "no-such-task").await;
        assert!(matches!(res, Err(SyncError::TaskNotFound)));
    }

    #[test]
    fn parse_mtime_handles_s3_seconds_and_sftp_format() {
        assert_eq!(parse_mtime(Some("1700000000")), Some(1700000000));
        assert_eq!(parse_mtime(Some("-1")), Some(-1));
        assert_eq!(
            parse_mtime(Some("2026-01-02 03:04:05")),
            Some(1767323045),
            "SFTP format parses as UTC"
        );
        assert_eq!(parse_mtime(Some("not a time")), None);
        assert_eq!(parse_mtime(Some("")), None);
        assert_eq!(parse_mtime(None), None);
    }

    #[test]
    fn file_name_extracts_last_segment() {
        assert_eq!(file_name("a/b/c.txt"), "c.txt");
        assert_eq!(file_name("a/b/c/"), "c");
        assert_eq!(file_name("top.txt"), "top.txt");
    }

    #[test]
    fn join_path_handles_root_and_nested() {
        assert_eq!(join_path("/src", ""), "/src");
        assert_eq!(join_path("/src/", ""), "/src/");
        assert_eq!(join_path("/src", "a/b"), "/src/a/b");
        assert_eq!(join_path("/src/", "a"), "/src/a");
    }

    #[test]
    fn sync_error_display_covers_variants() {
        assert_eq!(SyncError::Canceled.to_string(), "sync canceled");
        assert_eq!(
            SyncError::AlreadyExists("/a".into()).to_string(),
            "destination already exists: /a"
        );
        assert_eq!(SyncError::TaskNotFound.to_string(), "sync task not found");
    }
}
