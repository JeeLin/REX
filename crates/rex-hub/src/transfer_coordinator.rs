//! Hub 侧传输协调器 — T2 落地架构文档
//! (`docs/architecture/file-transfer.md` §TransferCoordinator)。
//!
//! `TransferCoordinator` 在服务端进程内驱动源端→目标端的直连流式传输：
//! 字节从 source `FileConnector` 的 `download_range` 读出，写入目标
//! `FileConnector` 的 `upload`，**不经过浏览器内存**。Hub 只负责编排
//! 任务生命周期（pending→running→verifying→completed/canceled/failed），
//! 前端仅创建任务、选择源/目标、展示进度、处理冲突。
//!
//! 流式合约（arch-doc T2）：
//! - 分片大小 `CHUNK_SIZE` (1 MiB)；
//! - `download_range(path, offset, Some(CHUNK))` → `upload(part, chunk, offset, None)` 循环；
//! - 每个分片轮次轮询 DB 状态，`canceled` 则中止；
//! - 校验目标 temp 文件尺寸==源文件尺寸，然后 `rename(temp → dst)`；
//! - `Move` 操作完成后 `source.delete(src_path)`。

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use rex_common::file_transfer::FileConnector;
use rex_transfer::{ConflictPolicy, TransferStatus};

use crate::app::{AppState, TransferProgressEvent};

/// 流式传输分片大小 — 1 MiB。与 S3 multipart 最小分片无关，仅为服务端直连
/// 的直连→直连传输分片。
pub const CHUNK_SIZE: u64 = 1024 * 1024;

/// 传输临时文件后缀：`{dst}.rex.part`。传输完成前目标路径始终以此命名，
/// 完成后原子 rename 到正式路径。
pub const TEMP_SUFFIX: &str = ".rex.part";

/// 传输操作类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferOp {
    /// 复制：目标存在后源保留。
    Copy,
    /// 移动：目标就位后删除源。
    Move,
}

/// 单次传输的静态参数：操作类型 + 冲突策略。
/// 打包为一个结构体以避免 `run_stream` 超过 clippy 的参数数上限。
#[derive(Debug, Clone, Copy)]
pub struct TransferSpec {
    pub op: TransferOp,
    pub conflict: ConflictPolicy,
}

/// 传输过程产生的错误。
#[derive(Debug)]
pub enum TransferError {
    /// 任务记录不存在。
    TaskNotFound,
    /// DB 写状态/进度失败。
    Db(String),
    /// 冲突策略字符串非法。
    InvalidConflictPolicy(String),
    /// 连接资源失败。
    Connect(String),
    /// 源文件 stat 失败。
    SourceStat(String),
    /// 目标文件 stat 失败。
    TargetStat(String),
    /// 分片下载失败。
    Download(String),
    /// 分片上传失败。
    Upload(String),
    /// temp→dst rename 失败。
    Rename(String),
    /// 源文件删除（Move）失败。
    Delete(String),
    /// 校验不一致（尺寸不匹配）。
    Verify(String),
    /// 目标已存在（Fail 冲突时）。
    AlreadyExists(String),
    /// 任务被取消。
    Canceled,
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskNotFound => write!(f, "transfer task not found"),
            Self::Db(e) => write!(f, "db error: {e}"),
            Self::InvalidConflictPolicy(p) => write!(f, "invalid conflict policy: {p}"),
            Self::Connect(e) => write!(f, "connect resource failed: {e}"),
            Self::SourceStat(e) => write!(f, "source stat failed: {e}"),
            Self::TargetStat(e) => write!(f, "target stat failed: {e}"),
            Self::Download(e) => write!(f, "download failed: {e}"),
            Self::Upload(e) => write!(f, "upload failed: {e}"),
            Self::Rename(e) => write!(f, "rename failed: {e}"),
            Self::Delete(e) => write!(f, "delete source failed: {e}"),
            Self::Verify(e) => write!(f, "verify failed: {e}"),
            Self::AlreadyExists(p) => write!(f, "destination already exists: {p}"),
            Self::Canceled => write!(f, "transfer canceled"),
        }
    }
}

impl std::error::Error for TransferError {}

/// 直连传输协调器。`Arc` 封装以允许 `submit` 克隆自身进入后台任务。
#[derive(Clone)]
pub struct TransferCoordinator {
    pub handles: Arc<std::sync::Mutex<HashMap<String, tokio::task::AbortHandle>>>,
}

impl Default for TransferCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl TransferCoordinator {
    /// 注册一个已中止句柄（用于测试/外部驱动）。
    pub fn register(&self, task_id: String, handle: tokio::task::AbortHandle) {
        self.handles.lock().unwrap().insert(task_id, handle);
    }

    pub fn new() -> Self {
        Self {
            handles: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// 提交传输任务：后台驱动 [`TransferCoordinator::run`]，并注册 AbortHandle，
    /// 供 [`TransferCoordinator::abort`] 中止。
    ///
    /// `self: &Arc<Self>` 使得后台任务能克隆 `Arc` 并在完成后回收句柄。
    pub fn submit(self: &Arc<Self>, state: AppState, task_id: String, op: TransferOp) {
        let coord = Arc::clone(self);
        let tid = task_id.clone();
        let handle = tokio::spawn(async move {
            let r = Self::run(&state, &tid, op).await;
            coord.handles.lock().unwrap().remove(&tid);
            r
        });
        self.handles
            .lock()
            .unwrap()
            .insert(task_id, handle.abort_handle());
    }

    /// 中止指定任务的后台传输流。返回是否找到并中止了一个活跃句柄。
    ///
    /// 仅中止进程内任务；DB 状态仍由 `cancel_transfer_task` 持久化为 `canceled`，
    /// `run_stream` 亦轮询 DB 状态作为兜底。
    pub fn abort(&self, task_id: &str) -> bool {
        match self.handles.lock().unwrap().remove(task_id) {
            Some(handle) => {
                handle.abort();
                true
            }
            None => false,
        }
    }

    /// 驱动一次完整传输：加载任务记录 → 打开 source/target 连接器 → `run_stream`。
    pub async fn run(state: &AppState, task_id: &str, op: TransferOp) -> Result<(), TransferError> {
        let task = state
            .db
            .get_transfer_task(task_id)
            .map_err(|e| TransferError::Db(e.to_string()))?
            .ok_or(TransferError::TaskNotFound)?;

        let conflict = ConflictPolicy::from_str(&task.conflict_policy)
            .ok_or_else(|| TransferError::InvalidConflictPolicy(task.conflict_policy.clone()))?;

        let mut source = crate::file_api::connect_resource(state, &task.source_resource_id)
            .await
            .map_err(|e| TransferError::Connect(e.to_string()))?;
        let mut target = crate::file_api::connect_resource(state, &task.target_resource_id)
            .await
            .map_err(|e| TransferError::Connect(e.to_string()))?;

        let res = Self::run_stream(
            state,
            task_id,
            TransferSpec { op, conflict },
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

    /// 纯流式传输内核：不关心连接器来历，仅依靠 `FileConnector` trait。
    ///
    /// 状态机：running → (verifying → completed | failed | canceled)。
    /// 轮询 DB `canceled` 状态以支持协作式中止。
    pub async fn run_stream(
        state: &AppState,
        task_id: &str,
        spec: TransferSpec,
        source: &mut dyn FileConnector,
        target: &mut dyn FileConnector,
        src_path: &str,
        dst_path: &str,
    ) -> Result<(), TransferError> {
        // 优先轮询取消：若任务在进入 streaming 前已被取消，立即退出。
        // （顺序在 set_status(Running) 之前，避免 clobber DB 中的 canceled 标记。）
        if Self::is_canceled(state, task_id) {
            Self::set_status(state, task_id, TransferStatus::Canceled, None);
            return Err(TransferError::Canceled);
        }
        Self::set_status(state, task_id, TransferStatus::Running, None);

        // 源文件总大小
        let src_entry = match source.stat(src_path).await {
            Ok(entry) => entry,
            Err(e) => {
                let msg = format!("source stat failed: {e}");
                if !Self::is_canceled(state, task_id) {
                    Self::set_status(
                        state,
                        task_id,
                        TransferStatus::Failed(msg.clone()),
                        Some(&msg),
                    );
                }
                return Err(TransferError::SourceStat(msg));
            }
        };
        let total = src_entry.size;

        // 冲突处理（事先 stat 目标）
        let final_dst = match Self::resolve_conflict(target, dst_path, spec.conflict).await? {
            ConflictOutcome::Skip => {
                Self::set_status(state, task_id, TransferStatus::Completed, None);
                return Ok(());
            }
            ConflictOutcome::Path(p) => p,
        };

        let temp = format!("{final_dst}{TEMP_SUFFIX}");

        let mut offset: u64 = 0;
        loop {
            // 轮询取消标记（协作式中止）
            if Self::is_canceled(state, task_id) {
                let _ = target.delete(&temp).await;
                Self::set_status(state, task_id, TransferStatus::Canceled, None);
                return Err(TransferError::Canceled);
            }

            // 在请求超出文件范围前短路：避免 S3 等返回 416（Range Not Satisfiable）
            // 的 connector 与 SSH/Agent 返回空 vec 的不一致；此守卫对所有 connector 生效。
            if offset >= total {
                // 最后一片轮询后、进入 Verifying 之前重新检查取消标记：
                // 若取消在此窗口被打，则不应 clobber DB 中的 canceled 标记。
                if Self::is_canceled(state, task_id) {
                    let _ = target.delete(&temp).await;
                    Self::set_status(state, task_id, TransferStatus::Canceled, None);
                    return Err(TransferError::Canceled);
                }
                // 零字节源文件：上面的守卫在第一轮就成立，循环体一次都不执行，
                // temp 从未在目标侧生成 —— 随后的 `stat temp` 必然失败，整任务落
                // `TargetStat`。此处补落一个空分片，让 temp 真正存在，校验与
                // rename 才有对象可依（S3 `offset == 0` 走 PutObject 可写空对象，
                // SFTP 走 create 建空文件）。与 sync_coordinator::copy_file 同构。
                if total == 0 {
                    if let Err(e) = target.upload(&temp, Vec::new(), 0, None).await {
                        let msg = format!("upload failed: {e}");
                        if !Self::is_canceled(state, task_id) {
                            Self::set_status(
                                state,
                                task_id,
                                TransferStatus::Failed(msg.clone()),
                                Some(&msg),
                            );
                        }
                        return Err(TransferError::Upload(msg));
                    };
                    state
                        .db
                        .update_transfer_task_progress(task_id, total, offset, 0, None)
                        .ok();
                    Self::broadcast_progress(state, task_id, "running", total, offset, 0, None);
                }
                break;
            }

            let chunk = match source
                .download_range(src_path, offset, Some(CHUNK_SIZE))
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    let msg = format!("download failed: {e}");
                    if !Self::is_canceled(state, task_id) {
                        Self::set_status(
                            state,
                            task_id,
                            TransferStatus::Failed(msg.clone()),
                            Some(&msg),
                        );
                    }
                    return Err(TransferError::Download(msg));
                }
            };
            if chunk.is_empty() {
                break;
            }
            let chunk_len = chunk.len() as u64;
            // upload(chunk, offset=cumulative) — 目标以累计偏移追加分片
            if let Err(e) = target.upload(&temp, chunk, offset, None).await {
                let msg = format!("upload failed: {e}");
                if !Self::is_canceled(state, task_id) {
                    Self::set_status(
                        state,
                        task_id,
                        TransferStatus::Failed(msg.clone()),
                        Some(&msg),
                    );
                }
                return Err(TransferError::Upload(msg));
            };
            offset += chunk_len;
            state
                .db
                .update_transfer_task_progress(task_id, total, offset, 0, None)
                .ok();
            // T5.4：进度变化时广播给 WS 订阅者（进行中无失败原因）
            Self::broadcast_progress(state, task_id, "running", total, offset, 0, None);
            if chunk_len < CHUNK_SIZE {
                break;
            }
        }

        // 校验：temp 尺寸 == 源尺寸
        Self::set_status(state, task_id, TransferStatus::Verifying, None);
        let temp_entry = match target.stat(&temp).await {
            Ok(entry) => entry,
            Err(e) => {
                let msg = format!("target stat failed: {e}");
                if !Self::is_canceled(state, task_id) {
                    Self::set_status(
                        state,
                        task_id,
                        TransferStatus::Failed(msg.clone()),
                        Some(&msg),
                    );
                }
                return Err(TransferError::TargetStat(msg));
            }
        };
        if temp_entry.size != total {
            let _ = target.delete(&temp).await;
            let msg = format!("size mismatch: src={} dst={}", total, temp_entry.size);
            Self::set_status(
                state,
                task_id,
                TransferStatus::Failed(msg.clone()),
                Some(&msg),
            );
            return Err(TransferError::Verify(msg));
        }

        // 原子落盘：temp → final
        if let Err(e) = target.rename(&temp, &final_dst).await {
            let msg = e.to_string();
            // S3 rename 非原子（copy+delete），失败时清理可能泄漏的 .rex.part temp
            let _ = target.delete(&temp).await;
            Self::set_status(
                state,
                task_id,
                TransferStatus::Failed(msg.clone()),
                Some(&msg),
            );
            return Err(TransferError::Rename(msg));
        }

        // Move：删除源
        if spec.op == TransferOp::Move {
            if let Err(e) = source.delete(src_path).await {
                let msg = e.to_string();
                Self::set_status(
                    state,
                    task_id,
                    TransferStatus::Failed(msg.clone()),
                    Some(&msg),
                );
                return Err(TransferError::Delete(msg));
            }
        }

        Self::set_status(state, task_id, TransferStatus::Completed, None);
        Ok(())
    }

    /// 冲突处理：stat 目标后，按策略决定落盘路径。
    async fn resolve_conflict(
        target: &mut dyn FileConnector,
        dst_path: &str,
        policy: ConflictPolicy,
    ) -> Result<ConflictOutcome, TransferError> {
        let exists = target.stat(dst_path).await.is_ok();
        if !exists {
            return Ok(ConflictOutcome::Path(dst_path.to_string()));
        }
        match policy {
            ConflictPolicy::Overwrite => Ok(ConflictOutcome::Path(dst_path.to_string())),
            ConflictPolicy::Skip => Ok(ConflictOutcome::Skip),
            ConflictPolicy::Fail => Err(TransferError::AlreadyExists(dst_path.to_string())),
            ConflictPolicy::Rename => {
                let renamed = Self::unique_name(target, dst_path).await?;
                Ok(ConflictOutcome::Path(renamed))
            }
        }
    }

    /// 为 `Rename` 冲突生成不存在的目标名：`name (1).ext`, `name (2).ext`, ...
    async fn unique_name(
        target: &mut dyn FileConnector,
        dst_path: &str,
    ) -> Result<String, TransferError> {
        let (stem, ext) = split_stem_ext(dst_path);
        for i in 1..1000u32 {
            let candidate = format!("{stem} ({i}){ext}");
            if target.stat(&candidate).await.is_err() {
                return Ok(candidate);
            }
        }
        Err(TransferError::AlreadyExists(dst_path.to_string()))
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

    /// 发布传输进度/状态变更到 WS 广播通道（T5.4）。
    /// `total`/`transferred`/`speed` 来自 DB 任务记录的当前快照。
    ///
    /// `error` 只在失败终态携带，供前端直接显示失败原因而不必等轮询兜底。
    fn broadcast_progress(
        state: &AppState,
        task_id: &str,
        status: &str,
        total: u64,
        transferred: u64,
        speed: u64,
        error: Option<&str>,
    ) {
        let event = TransferProgressEvent {
            task_id: task_id.to_string(),
            transferred_bytes: transferred,
            total_bytes: total,
            speed_bytes_per_sec: speed,
            status: status.to_string(),
            error: error.map(|e| e.to_string()),
        };
        // 忽略发送错误（无订阅者时）
        let _ = state.transfer_bcast.send(event);
    }

    fn set_status(state: &AppState, task_id: &str, status: TransferStatus, error: Option<&str>) {
        let _ = state
            .db
            .set_transfer_task_status(task_id, status.as_str_lossy(), error);
        // T5.4：状态变更时广播，顺便携带当前进度快照。
        if let Ok(Some(rec)) = state.db.get_transfer_task(task_id) {
            Self::broadcast_progress(
                state,
                task_id,
                status.as_str_lossy(),
                rec.total_bytes as u64,
                rec.transferred_bytes as u64,
                rec.speed_bytes_per_sec as u64,
                error,
            );
        }
    }
}

enum ConflictOutcome {
    Path(String),
    Skip,
}

/// 拆分路径 stem/ext，不改变目录结构。
fn split_stem_ext(path: &str) -> (String, String) {
    match path.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() && !s.ends_with('/') => (s.to_string(), format!(".{e}")),
        _ => (path.to_string(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use rex_common::file_transfer::{FileConnector, FileEntry, ProgressCallback, UploadResult};

    use super::*;

    /// 内存 FileConnector：以 append-at-offset 语义捕获上传字节，
    /// download_range 返回固定范围 — 用于验证 T2 流式合约。
    ///
    /// `simulate_s3_eof`: 模拟 S3 416 — 越界时返回 Err 而非空 vec。
    /// `fail_rename`: rename 始终失败。
    /// `fail_delete`: delete 始终失败。
    #[derive(Clone)]
    struct MockConnector {
        store: Arc<Mutex<HashMap<String, Vec<u8>>>>,
        simulate_s3_eof: bool,
        fail_rename: bool,
        fail_delete: bool,
        fail_upload: bool,
        fail_download: bool,
        fail_stat: bool,
        /// Hook called at the start of every upload — used to simulate a
        /// concurrent cancel (DB write from another task/thread).
        cancel_hook: Option<Arc<dyn Fn() + Send + Sync>>,
    }

    impl Default for MockConnector {
        fn default() -> Self {
            Self {
                store: Arc::new(Mutex::new(HashMap::new())),
                simulate_s3_eof: false,
                fail_rename: false,
                fail_delete: false,
                fail_upload: false,
                fail_download: false,
                fail_stat: false,
                cancel_hook: None,
            }
        }
    }

    impl MockConnector {
        fn new(store: Arc<Mutex<HashMap<String, Vec<u8>>>>) -> Self {
            Self {
                store,
                ..Default::default()
            }
        }

        fn with_flags(
            store: Arc<Mutex<HashMap<String, Vec<u8>>>>,
            simulate_s3_eof: bool,
            fail_rename: bool,
            fail_delete: bool,
        ) -> Self {
            Self {
                store,
                simulate_s3_eof,
                fail_rename,
                fail_delete,
                ..Default::default()
            }
        }

        /// Builder for setting fail_upload.
        fn with_fail_upload(mut self, val: bool) -> Self {
            self.fail_upload = val;
            self
        }

        /// Builder for setting fail_download.
        fn with_fail_download(mut self, val: bool) -> Self {
            self.fail_download = val;
            self
        }

        /// Builder for setting fail_stat.
        fn with_fail_stat(mut self, val: bool) -> Self {
            self.fail_stat = val;
            self
        }

        /// Builder for installing a cancel-on-upload hook.
        fn with_cancel_hook(mut self, hook: Arc<dyn Fn() + Send + Sync>) -> Self {
            self.cancel_hook = Some(hook);
            self
        }
    }

    #[async_trait]
    impl FileConnector for MockConnector {
        async fn list(&mut self, _path: &str) -> anyhow::Result<Vec<FileEntry>> {
            Ok(vec![])
        }

        async fn stat(&mut self, path: &str) -> anyhow::Result<FileEntry> {
            if self.fail_stat {
                anyhow::bail!("stat failed (simulated)");
            }
            let store = self.store.lock().unwrap();
            match store.get(path) {
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
                anyhow::bail!("download range failed (simulated)");
            }
            let data = self.download(path).await?;
            if self.simulate_s3_eof && offset >= data.len() as u64 {
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
            if self.fail_delete {
                anyhow::bail!("delete failed (simulated)");
            }
            self.store.lock().unwrap().remove(path);
            Ok(())
        }

        async fn rename(&mut self, from: &str, to: &str) -> anyhow::Result<()> {
            if self.fail_rename {
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

    fn make_state() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::resource_conn::build_test_state(dir.path());
        (dir, state)
    }

    #[tokio::test]
    async fn run_stream_copies_bytes_and_marks_completed() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/a.bin".into(),
                target_path: "/dst/a.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        // 跨越两个分片：第一片填满 CHUNK_SIZE，第二片 5 字节，
        // 验证 append-at-offset 跨分片拼接。
        let payload: Vec<u8> = (0..CHUNK_SIZE + 5)
            .map(|i| (i as u8).wrapping_add(7))
            .collect();

        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/a.bin".to_string(), payload.clone());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut source = MockConnector::new(src_store.clone());
        let mut target = MockConnector::new(dst_store.clone());

        TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/a.bin",
            "/dst/a.bin",
        )
        .await
        .unwrap();

        let captured = dst_store
            .lock()
            .unwrap()
            .get("/dst/a.bin")
            .cloned()
            .expect("destination file present after rename");
        assert_eq!(captured, payload, "captured bytes must equal source bytes");
        assert!(
            dst_store
                .lock()
                .unwrap()
                .get("/dst/a.bin.rex.part")
                .is_none(),
            "temp file must be renamed away"
        );

        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "completed");
        // Copy：源文件保留
        assert!(src_store.lock().unwrap().contains_key("/src/a.bin"));
    }

    #[tokio::test]
    async fn run_stream_move_deletes_source_and_captures_bytes() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/m.bin".into(),
                target_path: "/dst/m.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let payload: Vec<u8> = vec![1u8, 2, 3, 4, 5];
        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/m.bin".to_string(), payload.clone());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut source = MockConnector::new(src_store.clone());
        let mut target = MockConnector::new(dst_store.clone());

        TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Move,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/m.bin",
            "/dst/m.bin",
        )
        .await
        .unwrap();

        assert_eq!(
            dst_store.lock().unwrap().get("/dst/m.bin"),
            Some(&payload),
            "Move must still capture all bytes at destination"
        );
        assert!(
            !src_store.lock().unwrap().contains_key("/src/m.bin"),
            "Move must delete the source after successful transfer"
        );
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "completed");
    }

    #[tokio::test]
    async fn run_stream_canceled_aborts_and_marks_canceled() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/c.bin".into(),
                target_path: "/dst/c.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        // 预先标注取消；run_stream 轮询到后应中止。
        state
            .db
            .set_transfer_task_status(&task_id, "canceled", None)
            .unwrap();

        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/c.bin".to_string(), vec![9u8; 1024]);
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut source = MockConnector::new(src_store.clone());
        let mut target = MockConnector::new(dst_store.clone());

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/c.bin",
            "/dst/c.bin",
        )
        .await;
        assert!(matches!(res, Err(TransferError::Canceled)));
    }

    #[tokio::test]
    async fn run_stream_exact_chunk_multiple_copies_without_416() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/exact.bin".into(),
                target_path: "/dst/exact.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        // src 尺寸恰好为 2*CHUNK_SIZE — 整数倍，触发 S3 416 边界
        let payload: Vec<u8> = (0..2 * CHUNK_SIZE)
            .map(|i| (i as u8).wrapping_add(3))
            .collect();

        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/exact.bin".to_string(), payload.clone());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        // source 模拟 S3：越界 download_range 返回 Err (416)
        let mut source = MockConnector::with_flags(src_store.clone(), true, false, false);
        let mut target = MockConnector::new(dst_store.clone());

        TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/exact.bin",
            "/dst/exact.bin",
        )
        .await
        .unwrap();

        // temp 已 rename，source 保留
        let captured = dst_store
            .lock()
            .unwrap()
            .get("/dst/exact.bin")
            .cloned()
            .expect("destination file present after rename");
        assert_eq!(captured, payload, "captured bytes must equal source bytes");
        assert!(
            dst_store
                .lock()
                .unwrap()
                .get("/dst/exact.bin.rex.part")
                .is_none(),
            "temp file must be renamed away"
        );
        assert!(
            src_store.lock().unwrap().contains_key("/src/exact.bin"),
            "Copy must preserve source"
        );

        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "completed");
        assert_eq!(rec.total_bytes, (2 * CHUNK_SIZE) as i64);
        assert_eq!(rec.transferred_bytes, (2 * CHUNK_SIZE) as i64);
    }

    #[tokio::test]
    async fn run_stream_rename_failure_cleans_temp_and_marks_failed() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/r.bin".into(),
                target_path: "/dst/r.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let payload: Vec<u8> = vec![42u8; (CHUNK_SIZE + 10) as usize];
        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/r.bin".to_string(), payload.clone());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        // target rename 始终失败 → temp 应被清理，status=Failed
        let mut source = MockConnector::new(src_store.clone());
        let mut target = MockConnector::with_flags(dst_store.clone(), false, true, false);

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/r.bin",
            "/dst/r.bin",
        )
        .await;
        assert!(
            matches!(res, Err(TransferError::Rename(_))),
            "rename failure must surface as TransferError::Rename"
        );

        // temp 已清理
        assert!(
            dst_store
                .lock()
                .unwrap()
                .get("/dst/r.bin.rex.part")
                .is_none(),
            "temp file must be cleaned up after rename failure"
        );
        // final 文件不应存在
        assert!(
            dst_store.lock().unwrap().get("/dst/r.bin").is_none(),
            "final file must not exist after rename failure"
        );

        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "failed");
        assert!(
            rec.error.is_some(),
            "Failed status must carry an error message"
        );
    }

    #[tokio::test]
    async fn run_stream_move_delete_failure_marks_failed() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/d.bin".into(),
                target_path: "/dst/d.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let payload: Vec<u8> = vec![7u8; CHUNK_SIZE as usize];
        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/d.bin".to_string(), payload.clone());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        // source delete 始终失败 → status=Failed (非 verifying)
        let mut source = MockConnector::with_flags(src_store.clone(), false, false, true);
        let mut target = MockConnector::new(dst_store.clone());

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Move,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/d.bin",
            "/dst/d.bin",
        )
        .await;
        assert!(
            matches!(res, Err(TransferError::Delete(_))),
            "delete failure must surface as TransferError::Delete"
        );

        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "failed");
        assert!(
            rec.error.is_some(),
            "Failed status must carry an error message"
        );
    }

    #[test]
    fn split_stem_ext_handles_dirs_and_no_ext() {
        assert_eq!(
            split_stem_ext("/a/b/c.txt"),
            ("/a/b/c".to_string(), ".txt".to_string())
        );
        assert_eq!(
            split_stem_ext("/a/b/noext"),
            ("/a/b/noext".to_string(), "".to_string())
        );
        assert_eq!(
            split_stem_ext("file."),
            ("file".to_string(), ".".to_string())
        );
    }

    /// MockConnector (non-S3) 调 6 trait S3-only 方法 → trait 默认实现
    /// 回 `UnsupportedProtocolError`，Handler 据此映射 `UNSUPPORTED_PROTOCOL`。
    #[tokio::test]
    async fn mock_connector_s3_only_operations_are_unsupported() {
        use rex_common::file_transfer::UnsupportedProtocolError;

        let conn = MockConnector::new(Arc::new(Mutex::new(HashMap::new())));
        let conn: &dyn FileConnector = &conn;

        let errors = vec![
            conn.presigned_url("k", 60).await.unwrap_err(),
            conn.list_multipart_uploads("p").await.unwrap_err(),
            conn.resume_multipart_upload("k", "u", Vec::new(), None)
                .await
                .unwrap_err(),
            conn.abort_multipart_upload("k", "u").await.unwrap_err(),
            conn.get_acl("k").await.unwrap_err(),
            conn.put_acl("k", "private").await.unwrap_err(),
        ];
        for e in &errors {
            assert!(
                e.downcast_ref::<UnsupportedProtocolError>().is_some(),
                "non-S3 connector must return UnsupportedProtocolError for S3-only ops"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Bug 1: cancel-clobber window — cancel set during the last upload
    // (when offset reaches total) must not be clobbered by Verifying/Completed.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn cancel_during_last_upload_preserves_canceled_not_clobbered_by_verify() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/w.bin".into(),
                target_path: "/dst/w.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        // File of exactly CHUNK_SIZE so offset reaches total after one upload.
        let payload: Vec<u8> = (0..CHUNK_SIZE).map(|i| (i as u8).wrapping_add(9)).collect();
        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/w.bin".to_string(), payload);
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        // Cancel hook: set DB to "canceled" during the target's upload.
        let cancel_state = state.clone();
        let cancel_tid = task_id.clone();
        let hook: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = cancel_state
                .db
                .set_transfer_task_status(&cancel_tid, "canceled", None);
        });

        let mut source = MockConnector::new(src_store);
        let mut target = MockConnector::new(dst_store.clone()).with_cancel_hook(hook);

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/w.bin",
            "/dst/w.bin",
        )
        .await;

        assert!(
            matches!(res, Err(TransferError::Canceled)),
            "cancel during transfer must abort with Canceled, got {res:?}"
        );

        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(
            rec.status, "canceled",
            "canceled status must not be clobbered to verifying/completed"
        );
        assert!(
            dst_store
                .lock()
                .unwrap()
                .get("/dst/w.bin.rex.part")
                .is_none(),
            "temp file must be cleaned up on cancel"
        );
    }

    // -----------------------------------------------------------------------
    // Bug 2: error paths must set Failed status (unless already canceled).
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn source_stat_failure_sets_failed_status() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/missing.bin".into(),
                target_path: "/dst/missing.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        // /src/missing.bin is NOT in the store → stat fails
        let src_store = Arc::new(Mutex::new(HashMap::new()));
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut source = MockConnector::new(src_store);
        let mut target = MockConnector::new(dst_store);

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/missing.bin",
            "/dst/missing.bin",
        )
        .await;

        assert!(matches!(res, Err(TransferError::SourceStat(_))));
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "failed");
        assert!(rec.error.is_some());
    }

    #[tokio::test]
    async fn download_failure_sets_failed_status() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/dl.bin".into(),
                target_path: "/dst/dl.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let payload: Vec<u8> = (0..CHUNK_SIZE + 5)
            .map(|i| (i as u8).wrapping_add(3))
            .collect();
        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/dl.bin".to_string(), payload);
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut source = MockConnector::new(src_store).with_fail_download(true);
        let mut target = MockConnector::new(dst_store);

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/dl.bin",
            "/dst/dl.bin",
        )
        .await;

        assert!(matches!(res, Err(TransferError::Download(_))));
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "failed");
    }

    #[tokio::test]
    async fn upload_failure_sets_failed_status() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/up.bin".into(),
                target_path: "/dst/up.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let payload: Vec<u8> = (0..CHUNK_SIZE + 5)
            .map(|i| (i as u8).wrapping_add(5))
            .collect();
        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/up.bin".to_string(), payload);
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut source = MockConnector::new(src_store);
        let mut target = MockConnector::new(dst_store).with_fail_upload(true);

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/up.bin",
            "/dst/up.bin",
        )
        .await;

        assert!(matches!(res, Err(TransferError::Upload(_))));
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "failed");
    }

    #[tokio::test]
    async fn target_stat_failure_sets_failed_status() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/ts.bin".into(),
                target_path: "/dst/ts.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let payload: Vec<u8> = vec![1u8, 2, 3, 4, 5];
        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/ts.bin".to_string(), payload);
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut source = MockConnector::new(src_store);
        // fail_stat makes target.stat fail: resolve_conflict treats target
        // as non-existent (proceeds), uploads succeed, then verify stat
        // of the temp file fails → TargetStat error → Failed.
        let mut target = MockConnector::new(dst_store).with_fail_stat(true);

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/ts.bin",
            "/dst/ts.bin",
        )
        .await;

        assert!(matches!(res, Err(TransferError::TargetStat(_))));
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        // Must be "failed", not "verifying" (which was set just before the stat).
        assert_eq!(rec.status, "failed");
        assert!(rec.error.is_some());
    }

    // -----------------------------------------------------------------------
    // Bug: zero-byte source — the `offset >= total` guard fires on the very
    // first iteration, so the loop body never uploads anything and the temp
    // file is never created on the target. The following `stat temp` then
    // fails and the whole task lands in `failed`.
    // -----------------------------------------------------------------------

    /// Copy of a zero-byte source must produce a zero-byte destination file
    /// (target connector = SFTP-like, source connector = S3-like 416 on
    /// out-of-range). Both directions of the temp lifecycle are covered:
    /// the temp object must exist and must be renamed away.
    #[tokio::test]
    async fn run_stream_zero_byte_source_copies_empty_file() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/empty.bin".into(),
                target_path: "/dst/empty.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/empty.bin".to_string(), Vec::new());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        // source simulates S3: any download_range at offset 0 on an empty
        // object is out of range (416). The engine must never ask.
        let mut source = MockConnector::with_flags(src_store.clone(), true, false, false);
        let mut target = MockConnector::new(dst_store.clone());

        TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/empty.bin",
            "/dst/empty.bin",
        )
        .await
        .expect("zero-byte transfer must complete");

        let dst = dst_store.lock().unwrap();
        assert_eq!(
            dst.get("/dst/empty.bin"),
            Some(&Vec::new()),
            "destination must exist and be empty"
        );
        assert!(
            !dst.contains_key("/dst/empty.bin.rex.part"),
            "temp file must be renamed away"
        );
        drop(dst);

        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "completed");
        assert_eq!(rec.total_bytes, 0);
        assert_eq!(rec.transferred_bytes, 0);
        assert!(
            src_store.lock().unwrap().contains_key("/src/empty.bin"),
            "Copy must preserve the zero-byte source"
        );
    }

    /// Move of a zero-byte source: the empty file lands at the destination and
    /// the source is removed, same as any non-empty transfer.
    #[tokio::test]
    async fn run_stream_zero_byte_move_deletes_source() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/empty-move.bin".into(),
                target_path: "/dst/empty-move.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/empty-move.bin".to_string(), Vec::new());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut source = MockConnector::new(src_store.clone());
        let mut target = MockConnector::new(dst_store.clone());

        TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Move,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/empty-move.bin",
            "/dst/empty-move.bin",
        )
        .await
        .expect("zero-byte move must complete");

        assert_eq!(
            dst_store.lock().unwrap().get("/dst/empty-move.bin"),
            Some(&Vec::new())
        );
        assert!(
            !src_store
                .lock()
                .unwrap()
                .contains_key("/src/empty-move.bin"),
            "Move must delete the zero-byte source"
        );
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "completed");
    }

    /// Skip on an existing destination short-circuits before any upload, so it
    /// already worked for zero-byte sources. Pin it so the new empty-shard
    /// upload cannot regress that path (e.g. by running before the conflict
    /// check and clobbering the existing file).
    #[tokio::test]
    async fn run_stream_zero_byte_skip_leaves_destination_untouched() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/empty-skip.bin".into(),
                target_path: "/dst/empty-skip.bin".into(),
                conflict_policy: Some("skip".into()),
            })
            .unwrap();

        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/empty-skip.bin".to_string(), Vec::new());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));
        dst_store
            .lock()
            .unwrap()
            .insert("/dst/empty-skip.bin".to_string(), b"kept".to_vec());

        let mut source = MockConnector::new(src_store);
        let mut target = MockConnector::new(dst_store.clone());

        TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Skip,
            },
            &mut source,
            &mut target,
            "/src/empty-skip.bin",
            "/dst/empty-skip.bin",
        )
        .await
        .expect("skip must complete");

        assert_eq!(
            dst_store.lock().unwrap().get("/dst/empty-skip.bin"),
            Some(&b"kept".to_vec()),
            "Skip must not touch an existing destination"
        );
        assert!(
            !dst_store
                .lock()
                .unwrap()
                .contains_key("/dst/empty-skip.bin.rex.part"),
            "Skip must not create a temp file"
        );
    }

    /// A zero-byte source whose empty-shard upload fails must still land in
    /// `failed` with an `Upload` error — the new branch must not swallow the
    /// error or fall through to the temp stat.
    #[tokio::test]
    async fn run_stream_zero_byte_upload_failure_sets_failed() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/empty-up.bin".into(),
                target_path: "/dst/empty-up.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/empty-up.bin".to_string(), Vec::new());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut source = MockConnector::new(src_store);
        let mut target = MockConnector::new(dst_store).with_fail_upload(true);

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/empty-up.bin",
            "/dst/empty-up.bin",
        )
        .await;

        assert!(
            matches!(res, Err(TransferError::Upload(_))),
            "empty-shard upload failure must surface as TransferError::Upload, got {res:?}"
        );
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "failed");
        assert!(rec.error.is_some());
    }

    /// A zero-byte source whose empty-shard upload races a cancel must stay
    /// `canceled`: the cancel re-check inside the `offset >= total` branch runs
    /// before the upload, and the error path guards on `is_canceled`.
    #[tokio::test]
    async fn run_stream_zero_byte_canceled_keeps_canceled() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/empty-cancel.bin".into(),
                target_path: "/dst/empty-cancel.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/empty-cancel.bin".to_string(), Vec::new());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut source = MockConnector::new(src_store);
        let mut target = MockConnector::new(dst_store.clone());

        // Pre-canceled: the loop-top cancel guard returns before the
        // `offset >= total` branch, so the new empty-shard upload must never
        // run — no temp file, no destination file.
        state
            .db
            .set_transfer_task_status(&task_id, "canceled", None)
            .unwrap();

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/empty-cancel.bin",
            "/dst/empty-cancel.bin",
        )
        .await;

        assert!(matches!(res, Err(TransferError::Canceled)));
        let dst = dst_store.lock().unwrap();
        assert!(
            dst.get("/dst/empty-cancel.bin").is_none()
                && !dst.contains_key("/dst/empty-cancel.bin.rex.part"),
            "pre-canceled zero-byte transfer must not write anything, got {:?}",
            dst.keys().collect::<Vec<_>>()
        );
        drop(dst);
        let rec = state.db.get_transfer_task(&task_id).unwrap().unwrap();
        assert_eq!(rec.status, "canceled", "canceled must not be clobbered");
    }

    // -----------------------------------------------------------------------
    // Failure reason on the terminal WS event: the queue row must be able to
    // show *why* a transfer failed without waiting for the polling fallback.
    // -----------------------------------------------------------------------

    fn drain(
        rx: &mut tokio::sync::broadcast::Receiver<crate::app::TransferProgressEvent>,
    ) -> Vec<crate::app::TransferProgressEvent> {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    }

    #[tokio::test]
    async fn failed_terminal_event_carries_the_failure_reason() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/e.bin".into(),
                target_path: "/dst/e.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let payload: Vec<u8> = (0..CHUNK_SIZE + 5)
            .map(|i| (i as u8).wrapping_add(1))
            .collect();
        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/e.bin".to_string(), payload);
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut rx = state.transfer_bcast.subscribe();
        let mut source = MockConnector::new(src_store);
        let mut target = MockConnector::new(dst_store).with_fail_upload(true);

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/e.bin",
            "/dst/e.bin",
        )
        .await;
        assert!(matches!(res, Err(TransferError::Upload(_))));

        let events = drain(&mut rx);
        let failed = events
            .iter()
            .find(|e| e.status == "failed")
            .expect("a failed terminal event must be broadcast");
        assert_eq!(
            failed.error.as_deref(),
            Some("upload failed: upload failed (simulated)"),
            "the failed event must carry the reason the queue row shows"
        );

        // Non-terminal events must stay reason-free.
        for e in events.iter().filter(|e| e.status != "failed") {
            assert!(
                e.error.is_none(),
                "non-failed event {} must not carry an error",
                e.status
            );
        }
    }

    #[tokio::test]
    async fn cancel_and_completed_terminal_events_carry_no_failure_reason() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/ok.bin".into(),
                target_path: "/dst/ok.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/ok.bin".to_string(), b"payload".to_vec());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));

        let mut rx = state.transfer_bcast.subscribe();
        let mut source = MockConnector::new(src_store);
        let mut target = MockConnector::new(dst_store);

        TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/ok.bin",
            "/dst/ok.bin",
        )
        .await
        .expect("transfer must complete");

        let events = drain(&mut rx);
        let completed = events
            .iter()
            .find(|e| e.status == "completed")
            .expect("a completed terminal event must be broadcast");
        assert!(
            completed.error.is_none(),
            "a completed transfer has no failure reason"
        );
        assert!(
            !events.iter().any(|e| e.status == "failed"),
            "a successful transfer must not broadcast failed: {events:?}"
        );
    }

    /// A canceled transfer is not a failure: the terminal event must stay
    /// reason-free and the row must not be told why it "failed".
    #[tokio::test]
    async fn canceled_terminal_event_carries_no_failure_reason() {
        let (_dir, state) = make_state();
        let task_id = state
            .db
            .create_transfer_task(&crate::models::NewTransferTask {
                source_resource_id: "src".into(),
                target_resource_id: "dst".into(),
                source_path: "/src/cancel.bin".into(),
                target_path: "/dst/cancel.bin".into(),
                conflict_policy: Some("overwrite".into()),
            })
            .unwrap();

        let src_store = Arc::new(Mutex::new(HashMap::new()));
        src_store
            .lock()
            .unwrap()
            .insert("/src/cancel.bin".to_string(), b"data".to_vec());
        let dst_store = Arc::new(Mutex::new(HashMap::new()));
        state
            .db
            .set_transfer_task_status(&task_id, "canceled", None)
            .unwrap();

        let mut rx = state.transfer_bcast.subscribe();
        let mut source = MockConnector::new(src_store);
        let mut target = MockConnector::new(dst_store);

        let res = TransferCoordinator::run_stream(
            &state,
            &task_id,
            TransferSpec {
                op: TransferOp::Copy,
                conflict: ConflictPolicy::Overwrite,
            },
            &mut source,
            &mut target,
            "/src/cancel.bin",
            "/dst/cancel.bin",
        )
        .await;
        assert!(matches!(res, Err(TransferError::Canceled)));

        for e in drain(&mut rx) {
            assert!(
                e.error.is_none(),
                "cancel terminal event ({}) must not carry a failure reason",
                e.status
            );
        }
    }
}
