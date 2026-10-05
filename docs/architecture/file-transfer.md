# 文件传输架构

## 设计原则

- **文件传输数据不经过浏览器。** 前端只负责创建任务、选择源/目标、展示进度、处理冲突。实际传输由后端完成。
- **能力模型替代类型判断。** `FileConnector` 通过 `FileCapabilitySet` 上报协议能力，Hub 与前端均通过能力探测（而非 `downcast::<S3Connector>` / `isS3`）决策，消除协议专属分支。
- **进度推送。** Hub 通过 WebSocket (`/ws/files`) 向订阅的任务推送实时进度，取代轮询。
- **浏览器直传边界。** `POST /upload` 与 `GET /download` 用于浏览器管理的小文件直接传输（如编辑器读写、快速上传），数据通过浏览器。这与 `TransferCoordinator` 驱动的服务器间直连传输任务（数据不经过浏览器）是两个路径：交互式小文件操作走浏览器直传，大批量 / 服务器间搬运走任务队列。

---

## FileConnector trait

统一抽象（定义于 `rex-common::file_transfer`），各协议实现它：

```rust
pub trait FileConnector: Send + Sync {
    async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>>;
    async fn stat(&mut self, path: &str) -> Result<FileEntry>;
    async fn upload(&mut self, remote_path: &str, data: Vec<u8>, offset: u64, progress: Option<&ProgressCallback>) -> Result<UploadResult>;
    async fn download(&mut self, path: &str) -> Result<Vec<u8>>;
    async fn download_range(&mut self, path: &str, offset: u64, limit: Option<u64>) -> Result<Vec<u8>>;
    async fn delete(&mut self, path: &str) -> Result<()>;
    async fn rename(&mut self, from: &str, to: &str) -> Result<()>;
    async fn mkdir(&mut self, path: &str) -> Result<()>;
    async fn read_for_edit(&mut self, path: &str) -> Result<Vec<u8>>;
    async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> Result<()>;
    async fn close(&mut self) -> Result<()>;

    // --- 能力模型（v0.91.0 T3） ---
    fn capability(&self) -> FileCapabilitySet;
    async fn presigned_url(&self, key: &str, expires_in_secs: u64) -> Result<String>;
    async fn get_acl(&self, key: &str) -> Result<String>;
    async fn put_acl(&self, key: &str, canned_acl: &str) -> Result<()>;
    async fn list_multipart_uploads(&self, prefix: &str) -> Result<Vec<(String, String)>>;
    async fn resume_multipart_upload(&self, key: &str, upload_id: &str, data: Vec<u8>, progress: Option<&ProgressCallback>) -> Result<()>;
    async fn abort_multipart_upload(&self, key: &str, upload_id: &str) -> Result<()>;
    async fn chmod(&mut self, path: &str, mode: &str) -> Result<()>;
}

pub struct FileCapabilitySet {
    pub chmod: bool,         // SFTP
    pub presigned_url: bool,  // S3
    pub acl: bool,            // S3 (get/put ACL)
    pub multipart: bool,      // S3 (list/resume/abort multipart)
}
```

### 能力模型

`FileCapabilitySet` 是一个平坦的 bool 结构，直接序列化为 JSON。各协议实现 `capability()` 按真实支持面返回。Hub 的 `GET /api/files/connector/{resource_id}/capability` 端点查询该能力集，前端据此显示/隐藏功能按钮（而非 `isS3` 协议分支）。

### 实现

| 实现 | 协议 | 说明 |
|------|------|------|
| `SftpConnector`（rex-ssh） | SSH/SFTP | 通过 SSH 通道的 SFTP；`chmod` = true |
| `S3Connector`（rex-s3） | S3/MinIO | 对象存储操作（含 multipart 续传）；`presigned_url`/`acl`/`multipart` = true |
| `AgentFileProxy`（rex-hub agent_proxy） | Agent 代理 | 内网资源经 `/ws/agent` 隧道代理文件操作；能力透传 |
| `MemConnector`（rex-transfer 测试用） | 内存 | 单测/集成测试的内存实现 |

---

## 传输写入策略

```text
写入临时文件：{target}.rex.part
  ↓
完成后校验大小和 SHA256
  ↓
校验通过 → 原子 rename 替换目标文件
校验失败 → 保留或清理临时文件
```

---

## TransferCoordinator

Hub 侧传输协调器，定义于 `rex-hub::transfer_coordinator`。驱动 source-connector → target-connector 的直连流式传输。

```rust
pub struct TransferCoordinator {
    handles: HashMap<String, AbortHandle>,  // task_id → 背景任务句柄
}

pub enum TransferOp { Copy, Move }
pub struct TransferSpec { pub op: TransferOp, pub conflict: ConflictPolicy }

pub enum TransferError {
    TaskNotFound, Db(String), InvalidConflictPolicy(String),
    Connect(String), SourceStat(String), TargetStat(String),
    Download(String), Upload(String), Rename(String),
    Delete(String), Verify(String), AlreadyExists(String), Canceled,
}
```

### 任务生命周期

```text
pending → running → verifying → completed
                    ↓         ↓
              failed  ←  错误/校验不通过
                    ↓
              canceled  ←  用户取消
```

- `run_stream` 从 Redis 获取 `TransferTask`，建立 source/target connector，按 `CHUNK_SIZE`（1 MiB）分片传输。
- 每轮循环检查 `is_canceled()`，如取消则中止。
- 传输完成前检查 `is_canceled()`，再进入 `verifying` 阶段，防止 cancel-clobber。
- 错误路径（`SourceStat`/`Download`/`Upload`/`TargetStat`）通过 `?` 传播前，呼叫 `set_status(Failed, ...)` 记录失败状态。

---

## 前端交互

文件传输页面和标签页只负责：

- 创建任务
- 选择源和目标
- 展示进度
- 暂停/恢复/取消
- 处理冲突

### 文件操作端点（`/api/files/*`，由 `file_api` 提供）

| 端点 | 说明 |
|------|------|
| `POST /connect` | 按 resource 建立后端 `FileConnector`（SFTP / S3 / Agent 代理），返回 `session_id` |
| `POST /disconnect` | 断开会话 |
| `GET /list` | 列目录 |
| `POST /upload` | 浏览器 blob → 后端直传（文件直传，绕过任务队列） |
| `GET /download` | 后端 → 浏览器直传下载 |
| `POST /mkdir` | 建目录 |
| `POST /rename` | 重命名/移动 |
| `POST /delete` | 删除 |
| `POST /chmod` | 修改文件权限（SFTP，T4 补齐） |
| `GET /acl` · `PUT /acl` | S3 ACL 读/写 |
| `POST /presigned-url` | 获取 S3 预签名 URL |
| `GET /connector/{resource_id}/capability` | 查询 FileCapabilitySet（T3） |
| `GET /transfer` | 列表服务器端传输任务（persistent） |
| `POST /transfer/action` | 创建搬运任务（source/target server-side 直连） |
| `GET /transfer/{id}` | 查询任务详情（status/progress） |
| `POST /transfer/{id}/cancel` | 取消任务（持久化 canceled 标记） |
| `GET /read-for-edit` | 读取小文件内容（编辑器，最大 5MB） |
| `POST /save-from-edit` | 保存编辑内容 |

> 已移除（v0.91.0 T6 清理）：`GET /stat`、`GET /s3/multipart-uploads`、`POST /s3/resume-upload`、`POST /s3/abort-upload`。

### WebSocket 进度推送（v0.91.0 T5.4）

```
/ws/files?token=jwt
```

- 订阅服务器端传输任务进度。
- 客户端发送 `{"type":"subscribe","task_id":"..."}` 订阅指定任务。
- 服务端广播 `{"type":"transfer.progress","task_id":"...","transferred":N,"total":N,"speed":N}` 实时进度。
- 任务结束时广播 `{"type":"transfer.done","task_id":"...","status":"completed|failed|canceled"}`。
- Hub 侧 `TransferCoordinator` 在状态变更时触发广播，前端 `stores/transfer.ts` 订阅并更新 store。
- Polling 作为 fallback，WS 断连时回退到轮询。

### 跨连接传输路径（v0.91.0 T2）

```text
前端选择源文件 + 目标连接
  ↓
POST /api/files/transfer/action  (body: source_id, target_id, paths, op, conflict)
  ↓
Hub 建立 source / target 两个 FileConnector → TransferCoordinator::run_stream
  ↓
source.download_range(path, offset, CHUNK)  分片读取
  ↓
target.upload(remote_path, chunk, offset)  分片写入
  ↓
校验目标 temp 文件尺寸 == 源文件尺寸 → rename(.rex.part → dst)
  ↓
Move 操作完成后 source.delete(src_path)
  ↓
审计日志落地
```

### 统一传输队列（v0.91.0 T5）

前端通过 Pinia `stores/transfer.ts` 管理统一传输队列，单一数据源：

- **服务器端任务**：`POST /transfer/action` 创建，WS `/ws/files` 推送进度，轮询为 fallback。
- **浏览器任务**：浏览器直传/下载（如编辑器保存）通过 `store.pushBrowserTask()` 加入队列，XHR 事件驱动进度。
- 队列显示：FilesPage 底部抽屉 + FilesDrawer 右侧面板，共享同一 store。
- 冲突处理：`overwrite` / `skip` / `rename` / `fail`，前端在 transfer.action 请求前弹窗选择。

### 冲突处理

```ts
type ConflictPolicy = 'overwrite' | 'skip' | 'rename' | 'fail';
```

写入策略：

```text
目标文件存在
  ↓
根据冲突策略生成目标路径
  ↓
写入 {target}.rex.part
  ↓
完成后校验大小和 SHA256
  ↓
原子 rename
```
