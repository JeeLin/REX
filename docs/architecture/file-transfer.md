# 文件传输架构

## 设计原则

**文件传输数据不经过浏览器。** 前端只负责创建任务、选择源/目标、展示进度、处理冲突。实际传输由后端完成。

---

## FileConnector trait

```rust
#[async_trait]
pub trait FileConnector: Send + Sync {
    /// 获取文件/目录元信息
    async fn stat(&self, path: &str) -> Result<FileStat>;

    /// 打开文件进行读取
    async fn open_read(&self, path: &str) -> Result<InputStream>;

    /// 打开文件进行写入
    async fn open_write(&self, path: &str) -> Result<OutputStream>;

    /// 重命名/移动文件
    async fn rename(&self, from: &str, to: &str) -> Result<()>;

    /// 删除文件
    async fn remove(&self, path: &str) -> Result<()>;

    /// 列出目录内容
    async fn list_dir(&self, path: &str) -> Result<Vec<FileStat>>;
}
```

### 实现

| 实现 | 协议 | 说明 |
|------|------|------|
| `SshFileConnector` | SSH/SFTP | 通过 SSH 通道的 SFTP |
| `SftpFileConnector` | SFTP | 独立 SFTP 连接 |
| `MysqlFileConnector` | MySQL | 数据库导出/导入 |
| `S3FileConnector` | S3/MinIO | 对象存储操作 |
| `DockerFileConnector` | Docker | 容器内文件操作 |
| `LocalFileConnector` | 本地 | Agent 本机文件 |

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

### 创建传输任务

```http
POST /api/transfers
Content-Type: application/json

{
  "source": {
    "type": "ssh",
    "resourceId": "res_xxx",
    "path": "/home/pi/file.tar.gz"
  },
  "target": {
    "type": "sftp",
    "resourceId": "res_yyy",
    "path": "/volume1/backup/file.tar.gz"
  },
  "conflict": "rename"
}
```

### 进度推送

```http
WS /ws/transfers
```

事件：

```ts
type TransferEvent =
  | { type: 'progress'; taskId: string; progress: TransferProgress }
  | { type: 'completed'; taskId: string }
  | { type: 'failed'; taskId: string; message: string }
  | { type: 'canceled'; taskId: string };
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

### 跨连接传输路径

```text
前端选择源文件 + 目标连接
  ↓
Hub 创建 transfer task
  ↓
TransferCoordinator 找到 source connector 和 target connector
  ↓
source.open_read()
  ↓
target.open_write()
  ↓
流式分块复制
  ↓
校验、rename、写入审计日志
```
