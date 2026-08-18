# 文件传输架构

## 设计原则

**文件传输数据不经过浏览器。** 前端只负责创建任务、选择源/目标、展示进度、处理冲突。实际传输由后端完成。

---

## FileConnector trait

统一抽象（定义于 `rex-common::file_transfer`），各协议实现它：

```rust
pub trait FileConnector: Send + Sync {
    async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>>;
    async fn stat(&mut self, path: &str) -> Result<FileEntry>;
    async fn upload(&mut self, remote_path: &str, data: Vec<u8>, offset: u64, progress: Option<&ProgressCallback>) -> Result<UploadResult>;
    async fn download(&mut self, path: &str) -> Result<Vec<u8>>;
    // 支持 Range：从 offset 开始最多 limit 字节（续传/分片）
    async fn download_range(&mut self, path: &str, offset: u64, limit: Option<u64>) -> Result<Vec<u8>>;
    async fn delete(&mut self, path: &str) -> Result<()>;
    async fn rename(&mut self, from: &str, to: &str) -> Result<()>;
    async fn mkdir(&mut self, path: &str) -> Result<()>;
    // 编辑器临时下载/保存（限小文件，最大 5MB）
    async fn read_for_edit(&mut self, path: &str) -> Result<Vec<u8>>;
    async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
```

### 实现

| 实现 | 协议 | 说明 |
|------|------|------|
| `SftpConnector`（rex-ssh） | SSH/SFTP | 通过 SSH 通道的 SFTP |
| `S3Connector`（rex-s3） | S3/MinIO | 对象存储操作（含 multipart 续传） |
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

```rust
pub struct TransferCoordinator {
    tasks: HashMap<String, TransferTask>,
    connectors: HashMap<String, Box<dyn FileConnector>>,
}

pub struct TransferTask {
    id: String,
    source: TransferEndpoint,
    target: TransferEndpoint,
    status: TransferStatus,
    progress: TransferProgress,
}

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

pub struct TransferProgress {
    total_bytes: u64,
    transferred_bytes: u64,
    speed_bytes_per_sec: u64,
    elapsed: Duration,
    eta: Option<Duration>,
}
```

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
| `POST /connect` | 按 resource 建立后端 `FileConnector`（SFTP 或 S3） |
| `GET /list` | 列目录 |
| `GET /stat` | 取文件/目录元信息 |
| `POST /mkdir` | 建目录 |
| `POST /rename` | 重命名/移动 |
| `DELETE /delete` | 删除 |
| `POST /upload` | 上传（支持 offset 断点续传，`progress` 回调回报进度） |
| `GET /download` | 下载（支持 Range，对应 `download_range`） |
| `POST /acl` | S3 ACL 读写 |

> 前端通过 `FileConnector` 的 `upload(offset)/download_range(offset, limit)` 实现断点续传与分片；进度由后端 `ProgressCallback` 经 REST 响应或前端轮询/状态展示，不经过浏览器中转数据。

### 跨连接传输路径

```text
前端选择源文件 + 目标连接
  ↓
Hub 建立 source / target 两个 FileConnector
  ↓
source.download_range() 分片读取
  ↓
target.upload(remote_path, chunk, offset, progress) 分片写入
  ↓
校验、rename、写入审计日志
```

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
