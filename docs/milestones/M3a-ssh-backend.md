# M3a: SSH 终端后端

## Context

M0 交付了 Rust workspace 骨架。M1 实现了 Hub HTTP API、环境/资源 CRUD、登录认证、审计日志。M2 实现了 Agent 与 Hub 的连接（配置、注册、WebSocket 心跳、在线状态）。M3a 在此基础上实现 SSH 终端的后端能力：SSH 客户端 crate、SSH 凭据加密、Terminal session 模型、WebSocket 终端数据通道。M3b 将实现前端 terminal 页面。

## 产品边界

**做什么：**
- SSH 客户端 crate（`rex-ssh`）：连接、认证、PTY、数据读写
- SSH 资源配置验证与敏感凭据加密（AES-256-GCM）
- Terminal session 模型：生命周期管理、状态机、超时检测
- WebSocket 终端数据通道：前端 ↔ 后端实时终端 I/O
- Session REST API（创建、关闭）

**不做什么：**
- 前端 terminal 页面（M3b）
- Agent SSH 代理通道（M3b）
- 移动端方向键 + bash 历史（M3b）
- 后端审计 SSH 连接/断开（M3b）
- SFTP 内置面板（M4）
- 跳板机/代理跳转
- known_hosts 校验（使用 IgnoreHostKey）
- 私钥文件上传存储
- 会话持久化（进程重启后会话丢失）

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 3a.1 | SSH 客户端 crate | 后端 | ✅ |
| 3a.2 | SSH 资源配置验证与凭据加密 | 后端 | ✅ |
| 3a.3 | Terminal session 模型 | 后端 | ✅ |
| 3a.4 | WebSocket 终端数据通道 + REST API | 后端 | ✅ |

---

## 子任务 3a.1：SSH 客户端 crate

### 功能目标

创建 `rex-ssh` crate，封装 SSH 连接、认证、PTY 分配和数据读写。为后续 terminal session 和 WebSocket 数据通道提供底层能力。

### 文件结构

```text
crates/rex-ssh/
├── Cargo.toml       新增：russh、tokio、anyhow 依赖
└── src/
    ├── lib.rs       新增：导出模块
    ├── client.rs    新增：SshClient 封装
    └── auth.rs      新增：认证方式定义
```

根 `Cargo.toml` 添加 `russh`、`russh-keys` 依赖（使用 `workspace = true`）。

### 接口设计

```rust
// crates/rex-ssh/src/auth.rs
pub enum AuthMethod {
    Password(String),
    Key {
        private_key_path: String,
        passphrase: Option<String>,
    },
}

// crates/rex-ssh/src/client.rs
pub struct SshClient {
    // russh client session
}

pub struct SshEvent {
    pub data: Vec<u8>,          // 终端输出数据
    pub closed: bool,           // 连接已关闭
    pub error: Option<String>,  // 错误信息
}

impl SshClient {
    pub async fn connect(
        host: &str,
        port: u16,
        username: &str,
        auth: AuthMethod,
    ) -> anyhow::Result<Self>;

    pub async fn request_pty(&mut self, cols: u32, rows: u32) -> anyhow::Result<()>;

    pub async fn request_shell(&mut self) -> anyhow::Result<()>;

    pub async fn send_data(&mut self, data: &[u8]) -> anyhow::Result<()>;

    pub async fn window_change(&mut self, cols: u32, rows: u32) -> anyhow::Result<()>;

    pub async fn recv(&mut self) -> anyhow::Result<SshEvent>;

    pub async fn disconnect(&mut self) -> anyhow::Result<()>;
}
```

### 测试标准

- `auth_method_password` — Password 变体构造正确
- `auth_method_key` — Key 变体构造正确
- `ssh_event_struct` — SshEvent 字段可访问
- 连接测试标记为 `#[ignore]`（需要真实 SSH 服务器）

### 提交边界

```text
feat: add rex-ssh client crate with PTY support
```

不得包含：WebSocket、session 模型、前端、加密。

---

## 子任务 3a.2：SSH 资源配置验证与凭据加密

### 功能目标

为 SSH 资源添加配置验证和敏感凭据加密。前端提交的 SSH 资源配置（密码、私钥密码）在存入数据库前加密，读取时解密。API 层对 SSH 资源的 `config_json` 进行结构化验证。

### 文件结构

```text
crates/rex-ssh/
├── Cargo.toml           修改：添加 aes-gcm、sha2、hex 依赖
└── src/
    ├── lib.rs           修改：导出 crypto 模块
    └── crypto.rs        新增：加密/解密工具函数

crates/rex-hub/
├── Cargo.toml           修改：添加 rex-ssh 依赖
└── src/
    ├── ssh_config.rs    新增：SSH 配置 DTO 和验证
    └── resource.rs      修改：添加 SSH 配置验证逻辑
```

### 数据模型

**SSH 资源配置 JSON 结构（加密前）：**

```json
{
  "host": "192.0.2.1",
  "port": 22,
  "username": "pi",
  "auth": {
    "type": "password",
    "password": "明文密码"
  },
  "terminal": {
    "encoding": "utf-8",
    "keepAliveSeconds": 60
  }
}
```

**SSH 资源配置 JSON 结构（加密后，存库）：**

```json
{
  "host": "192.0.2.1",
  "port": 22,
  "username": "pi",
  "auth": {
    "type": "password",
    "passwordEncrypted": "base64(nonce || ciphertext || tag)"
  },
  "terminal": {
    "encoding": "utf-8",
    "keepAliveSeconds": 60
  }
}
```

**加密方案：**

```text
REX_SECRET_KEY (UTF-8)
  ↓
SHA-256 哈希 → 32 字节派生密钥
  ↓
AES-256-GCM 加密（随机 12 字节 nonce）
  ↓
存储格式：base64(nonce + ciphertext + tag)
```

### 接口设计

```rust
// crates/rex-ssh/src/crypto.rs
pub fn derive_key(secret_key: &str) -> [u8; 32];
pub fn encrypt(plaintext: &str, secret_key: &str) -> String;
pub fn decrypt(ciphertext: &str, secret_key: &str) -> anyhow::Result<String>;

// crates/rex-hub/src/ssh_config.rs
#[derive(Debug, Deserialize)]
pub struct SshAuthConfig {
    #[serde(rename = "type")]
    pub auth_type: String,
    pub password: Option<String>,
    pub password_encrypted: Option<String>,
    pub private_key_path: Option<String>,
    pub passphrase: Option<String>,
    pub passphrase_encrypted: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SshTerminalConfig {
    pub encoding: Option<String>,
    pub keep_alive_seconds: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SshResourceConfig {
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub auth: SshAuthConfig,
    pub terminal: Option<SshTerminalConfig>,
}

impl SshResourceConfig {
    pub fn from_json(json: &str) -> anyhow::Result<Self>;
    pub fn encrypt_sensitive(&self, secret_key: &str) -> anyhow::Result<String>;
    pub fn from_encrypted_json(json: &str, secret_key: &str) -> anyhow::Result<Self>;
    pub fn to_auth_method(&self, secret_key: &str) -> anyhow::Result<rex_ssh::auth::AuthMethod>;
}
```

### 测试标准

**rex-ssh/crypto.rs：**
- `derive_key_deterministic` — 相同输入产生相同密钥
- `derive_key_different_inputs` — 不同输入产生不同密钥
- `encrypt_decrypt_roundtrip` — 加密后解密还原原文
- `decrypt_invalid_input` — 无效输入返回错误
- `decrypt_wrong_key` — 用错误密钥解密返回错误
- `encrypt_empty_string` — 空字符串可正常加密解密

**rex-hub/ssh_config.rs：**
- `parse_valid_password_config` — 合法密码配置解析成功
- `parse_valid_key_config` — 合法密钥配置解析成功
- `parse_missing_host` — 缺少 host 返回错误
- `parse_missing_username` — 缺少 username 返回错误
- `parse_invalid_auth_type` — 非法 auth.type 返回错误
- `parse_invalid_json` — 非法 JSON 返回错误
- `default_port_is_22` — 未指定 port 时默认 22
- `encrypt_sensitive_fields` — 敏感字段加密后不含明文
- `decrypt_restores_original` — 解密后还原原始配置
- `to_auth_method_password` — 密码认证转换正确
- `to_auth_method_key` — 密钥认证转换正确

### 提交边界

```text
feat: add SSH config validation and credential encryption
```

不得包含：WebSocket、session 模型、前端、审计。

---

## 子任务 3a.3：Terminal session 模型

### 功能目标

实现终端会话生命周期管理。每个 SSH 连接对应一个 TerminalSession，管理连接状态、PTY 尺寸、超时检测和资源清理。Session 存储在内存中，不持久化到数据库。

### 文件结构

```text
crates/rex-hub/
└── src/
    └── terminal.rs    新增：TerminalSession + SessionManager
```

### 数据模型

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Connecting,
    Connected,
    Active,
    Closed,
}

pub struct TerminalSession {
    pub id: String,                    // sess_xxxxxxxx
    pub resource_id: String,
    pub state: SessionState,
    pub cols: u32,
    pub rows: u32,
    pub created_at: Instant,
    pub last_active_at: Instant,
    client: Option<SshClient>,
}

pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, TerminalSession>>>,
    max_idle_secs: u64,                // 默认 900 (15 分钟)
}
```

### 接口设计

```rust
impl TerminalSession {
    pub fn new(resource_id: &str, cols: u32, rows: u32) -> Self;
    pub fn set_client(&mut self, client: SshClient) -> anyhow::Result<()>;
    pub async fn init_shell(&mut self) -> anyhow::Result<()>;
    pub async fn send_data(&mut self, data: &[u8]) -> anyhow::Result<()>;
    pub async fn recv(&mut self) -> anyhow::Result<SshEvent>;
    pub async fn resize(&mut self, cols: u32, rows: u32) -> anyhow::Result<()>;
    pub async fn close(&mut self) -> anyhow::Result<()>;
    pub fn touch(&mut self);
    pub fn is_idle_timeout(&self, max_idle_secs: u64) -> bool;
}

impl SessionManager {
    pub fn new(max_idle_secs: u64) -> Self;
    pub fn create_session(&self, resource_id: &str, cols: u32, rows: u32) -> String;
    pub fn get(&self, session_id: &str) -> Option<TerminalSession>;
    pub async fn remove_session(&self, session_id: &str) -> anyhow::Result<()>;
    pub async fn cleanup_idle(&self);
    pub fn count(&self) -> usize;
}
```

### 测试标准

- `session_new_state_is_connecting` — 新建会话状态为 Connecting
- `session_set_client_transitions_to_connected` — 设置 client 后状态为 Connected
- `session_send_data_without_client_fails` — 无 client 时 send_data 返回错误
- `session_touch_updates_last_active` — touch 更新 last_active_at
- `session_is_idle_timeout` — 空闲超时检测正确
- `session_is_not_idle_when_active` — 活跃会话不超时
- `manager_create_and_get` — 创建并获取会话
- `manager_remove_session` — 移除会话
- `manager_cleanup_idle` — 清理空闲会话
- `manager_count` — 会话计数正确
- `manager_get_nonexistent_returns_none` — 获取不存在的会话返回 None

### 提交边界

```text
feat: add terminal session model
```

不得包含：WebSocket 通道、前端、Agent 代理、审计。

---

## 子任务 3a.4：WebSocket 终端数据通道 + REST API

### 功能目标

实现前端通过 WebSocket 与后端 TerminalSession 之间的实时数据通道。前端通过 WebSocket 发送终端输入、接收终端输出、发送窗口大小调整命令。后端在收到 WebSocket 连接后，自动完成 SSH 连接、PTY 分配、shell 初始化。同时提供 REST API 创建和关闭 session。

### 文件结构

```text
crates/rex-hub/src/
├── ws_terminal.rs    新增：WebSocket 终端数据通道处理
├── terminal.rs       不变
└── routes.rs         修改：注册 WebSocket 路由和 REST API
```

### 消息协议

```json
// 前端 → 后端
{ "type": "terminal.input", "payload": { "data": "base64编码的输入" } }
{ "type": "terminal.resize", "payload": { "cols": 120, "rows": 40 } }

// 后端 → 前端
{ "type": "terminal.output", "payload": { "data": "base64编码的输出" } }
{ "type": "terminal.closed", "payload": { "exit_status": 0 } }
{ "type": "terminal.error", "payload": { "message": "错误信息" } }
```

### REST API

```text
POST /api/ssh/sessions
Request: { resource_id, cols, rows }
Response: { session_id }

DELETE /api/ssh/sessions/:session_id
Response: 204 No Content
```

### 接口设计

```rust
// crates/rex-hub/src/ws_terminal.rs
#[derive(Debug, Deserialize)]
pub struct TerminalMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: serde_json::Value,
}

pub async fn terminal_ws_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse;

async fn handle_terminal_socket(
    socket: WebSocket,
    session_id: String,
    state: Arc<AppState>,
);
```

### 后端流程

```text
前端 POST /api/ssh/sessions { resource_id, cols, rows }
  ↓
返回 { session_id }
  ↓
前端连接 /ws/terminal/:session_id
  ↓
后端从 DB 读取资源配置
  ↓
SshResourceConfig::from_encrypted_json()
  ↓
SshClient::connect()
  ↓
TerminalSession::set_client() → init_shell()
  ↓
开始循环：
  前端发消息 → 后端处理（input/resize）
  后端 recv → 前端输出（output/closed/error）
```

### AppState 扩展

```rust
pub struct AppState {
    pub db: Arc<Database>,
    pub secret_key: String,
    pub connections: Arc<AgentConnections>,
    pub sessions: Arc<SessionManager>,  // 新增
}
```

### 测试标准

**单元测试：**
- `terminal_message_deserialize` — 消息反序列化正确
- `terminal_output_message_serialize` — 输出消息序列化正确

**集成测试（标记 ignore，需要真实 SSH）：**
- `ws_terminal_connect_and_send` — WebSocket 连接并发送数据

### 提交边界

```text
feat: add WebSocket terminal data channel and session REST API
```

不得包含：前端、Agent 代理、审计。

---

## 设计核对点

- [x] 单用户、自托管定位：不引入多用户、RBAC
- [x] SSH 连接由 Hub 直连目标服务器（Agent 代理在 M3b）
- [x] SSH 密码/私钥密码加密存储，密钥从 REX_SECRET_KEY 派生
- [x] Terminal session 不持久化，进程重启后会话丢失
- [x] WebSocket 消息使用 base64 编码二进制数据
- [x] Session 空闲超时 15 分钟自动断开
- [x] 不实现跳板机、known_hosts 校验、私钥上传
- [x] 前端 terminal 页面在 M3b 实现

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交
