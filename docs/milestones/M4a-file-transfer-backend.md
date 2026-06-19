# M4a: 文件传输后端

## Context

M0-M3b 完成了项目骨架、Hub 管理 API、Agent 连接、SSH 终端。M4a 实现文件传输的后端基础：FileConnector trait、SFTP connector、本地文件 connector、传输任务模型。

## 产品边界

**做什么：**
- `rex-transfer` crate：FileConnector trait、文件元数据模型
- SFTP connector：通过 SSH 连接实现文件操作
- 本地文件 connector：本地文件系统操作
- 传输任务模型：任务创建、进度、状态管理
- `/api/transfers` REST API（创建、查询、取消）
- `/ws/transfers` WebSocket 实时进度推送

**不做什么：**
- 前端文件页面（M4b）
- 前端传输队列 UI（M4b）
- 临时文件写入和 SHA256 校验（M4b）
- 冲突策略（M4b）
- 审计日志扩展（M4b）
- 跳板机/代理跳转

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 4a.1 | FileConnector trait + 文件模型 | 后端 | ✅ |
| 4a.2 | SFTP connector 实现 | 后端 | ✅ |
| 4a.3 | 本地文件 connector | 后端 | ✅ |
| 4a.4 | 传输任务模型 + REST API + WebSocket | 后端 | ✅ |

---

## 子任务 4a.1：FileConnector trait

### 功能目标

创建 `rex-transfer` crate，定义文件操作统一接口和数据模型。

### 文件结构

```text
crates/rex-transfer/
├── Cargo.toml
└── src/
    └── lib.rs
```

### 接口设计

```rust
#[async_trait::async_trait]
pub trait FileConnector: Send + Sync {
    fn connector_name(&self) -> &'static str;
    async fn list(&self, path: &Path) -> Result<Vec<FileEntry>>;
    async fn metadata(&self, path: &Path) -> Result<FileEntry>;
    async fn read(&self, path: &Path) -> Result<FileRead>;
    async fn write(&self, path: &Path, bytes: &[u8]) -> Result<()>;
}
```

### 测试标准

- trait 对象安全
- 文件模型序列化/反序列化

### 提交信息

```
feat: add file connector trait and models
```

---

## 子任务 4a.2：SFTP connector

### 功能目标

实现基于 SSH 的 SFTP 文件操作 connector。

### 文件结构

```text
crates/rex-transfer/src/
└── sftp.rs
```

### 接口设计

```rust
pub struct SftpConnector {
    client: SshClient,
}

impl SftpConnector {
    pub async fn connect(host: &str, port: u16, username: &str, auth: AuthMethod) -> Result<Self>;
}
```

### 测试标准

- 连接、列表、读取、写入、断开
- 错误处理（连接失败、权限不足）

### 提交信息

```
feat: add SFTP connector implementation
```

---

## 子任务 4a.3：本地文件 connector

### 功能目标

实现本地文件系统的文件操作 connector。

### 文件结构

```text
crates/rex-transfer/src/
└── local.rs
```

### 接口设计

```rust
pub struct LocalConnector {
    base_path: PathBuf,
}

impl LocalConnector {
    pub fn new(base_path: PathBuf) -> Result<Self>;
}
```

### 测试标准

- 列表、元数据、读取、写入
- 路径安全检查（防止目录穿越）

### 提交信息

```
feat: add local file connector
```

---

## 子任务 4a.4：传输任务模型

### 功能目标

实现传输任务的创建、进度追踪、状态管理和 REST API。

### 文件结构

```text
crates/rex-transfer/src/
└── task.rs

crates/rex-hub/src/
├── transfer.rs
└── routes.rs    修改
```

### 接口设计

```rust
pub struct TransferTask {
    pub id: String,
    pub source: TransferEndpoint,
    pub target: TransferEndpoint,
    pub status: TransferStatus,
    pub progress: TransferProgress,
}

pub enum TransferStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

pub struct TransferProgress {
    pub total_bytes: u64,
    pub transferred_bytes: u64,
}
```

REST API：
- `POST /api/transfers` — 创建传输任务
- `GET /api/transfers` — 列表
- `GET /api/transfers/:id` — 详情
- `DELETE /api/transfers/:id` — 取消

WebSocket：
- `GET /ws/transfers/:id` — 实时进度推送

### 测试标准

- 任务创建、状态流转
- REST API 集成测试

### 提交信息

```
feat: add transfer task model and REST API
```

## 设计核对点

- [ ] 文件数据不经过浏览器
- [ ] FileConnector trait 可扩展
- [ ] 传输进度实时推送
- [ ] 任务可取消

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交
