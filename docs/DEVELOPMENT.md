# REX Hub — 开发文档

本文档记录实现细节，供开发参考。产品功能和设计决策请参阅 [PRODUCT.md](PRODUCT.md)。

---

## 1. 技术栈

| 层 | 技术 |
|---|---|
| 后端 | Rust（async-std / tokio） |
| 前端 | Vue 3 + Vite |
| 终端 | xterm.js |
| 通信 | WebSocket + HTTPS |
| 加密 | TLS 1.3, AES-256, ECDHE-X25519 |

---

## 2. Rust crate 结构

代码从第一阶段开始按 crate 组织，编译成两个二进制（`rex-hub` / `rex-agent`）。

```text
crates/
├── rex-common        通用类型、错误定义、配置解析
├── rex-ssh           SSH/SFTP 协议实现
├── rex-mysql         MySQL 协议实现
├── rex-postgresql    PostgreSQL 协议实现
├── rex-redis         Redis 协议实现
├── rex-docker        Docker 协议实现
├── rex-sqlite        SQLite 协议实现
├── rex-s3            S3/MinIO 协议实现
├── rex-transfer      文件传输引擎
├── rex-tunnel        WebSocket 隧道
├── rex-supervisor    进程 supervisor（启动、监控、状态判断、替换、回滚）
├── rex-hub           Hub 二进制入口（整合所有 crate + 前端静态资源）
└── rex-agent         Agent 二进制入口（整合所有 crate）
```

Hub 和 Agent 共享所有协议 crate，区别在于：

```text
rex-hub   = 所有 crate + 前端静态资源（embedded）
rex-agent = 所有 crate（无前端）
```

---

## 3. 进程模型

### 3.1 入口逻辑

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--worker".to_string()) {
        // worker 模式：直接运行业务逻辑
        run_worker();
    } else {
        // supervisor 模式：启动 worker 并监控
        run_supervisor();
    }
}
```

### 3.2 supervisor 循环

```rust
fn run_supervisor() {
    loop {
        // 1. 启动 worker
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--worker")
            .spawn()
            .expect("failed to start worker");

        // 2. 等待 worker 退出
        let exit_status = child.wait().expect("worker wait failed");
        let code = exit_status.code().unwrap_or(12);

        // 3. 读取 update-state.json
        let state = read_update_state();

        // 4. 根据退出码和状态判断下一步
        match code {
            0 => {
                if state.phase == "requested" {
                    replace_binary(&state.staged_path);
                    clean_rollback(&state.rollback_path);
                }
                thread::sleep(Duration::from_secs(1));
            }
            10 => {
                if state.phase == "requested" {
                    replace_binary(&state.staged_path);
                    write_update_state(UpdateState {
                        phase: "starting_new".to_string(),
                        attempt: 0,
                        ..Default::default()
                    });
                }
            }
            11 | 12 => {
                if state.attempt >= 3 {
                    rollback(&state.rollback_path);
                } else {
                    increment_attempt();
                }
            }
            _ => {
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}
```

### 3.3 worker 模式

```rust
fn run_worker() {
    // 检查是否处于更新验证阶段
    if env::var("REX_UPDATE_PENDING").is_ok() {
        // 跳过更新检查，直接进入健康检查
        run_health_check();
        return;
    }

    // 正常业务逻辑
    start_business();

    // 定期检查更新
    loop {
        check_for_update();
        sleep(Duration::from_secs(3600)); // 每 24 小时
    }
}
```

---

## 4. 退出码语义

| 退出码 | 含义 | supervisor 行为 |
|--------|------|----------------|
| `0` | 正常退出 | 检查 update-state.json，决定是否替换二进制 |
| `10` | 请求更新 | 读取 update-state.json，替换二进制，启动新版 |
| `11` | 健康检查失败 | attempt + 1，连续 3 次回滚 |
| `12` | 崩溃/异常退出 | attempt + 1，连续 3 次回滚 |

supervisor 决策逻辑：

| 退出码 | update-state.phase | attempt | 动作 |
|--------|-------------------|---------|------|
| 0 | idle | any | 重启 worker |
| 0 | requested | any | 替换二进制，启动新版 |
| 10 | requested | < 3 | 替换二进制，启动新版 |
| 10 | requested | >= 3 | 回滚 |
| 11 | any | < 3 | attempt + 1，重启 |
| 11 | any | >= 3 | 回滚 |
| 12 | any | < 3 | attempt + 1，重启 |
| 12 | any | >= 3 | 回滚 |

---

## 5. 更新状态文件

### 5.1 路径

```text
{data-dir}/update-state.json
```

### 5.2 Schema

```json
{
  "phase": "idle",
  "target_version": "",
  "old_version": "v0.1.0",
  "staged_path": "",
  "rollback_path": "",
  "attempt": 0
}
```

### 5.3 状态枚举

| 状态 | 说明 |
|------|------|
| `idle` | 无更新，正常运行 |
| `requested` | worker 已下载新版本并写入状态，请求 supervisor 重启 |
| `starting_new` | supervisor 正在替换二进制并启动新版 worker |
| `committed` | 新版本健康检查通过，更新完成，删除旧备份 |
| `rolling_back` | 新版本健康检查失败，supervisor 正在恢复旧版 |
| `rolled_back` | 回滚完成，旧版 worker 正在运行 |
| `failed` | 连续 3 次启动失败，更新终止，保留旧版 |

### 5.4 状态流转

```text
idle
  ↓ (worker 下载新版)
requested
  ↓ (supervisor 替换二进制)
starting_new
  ↓ (健康检查通过)          ↓ (健康检查失败, attempt < 3)    ↓ (attempt >= 3)
committed               rolling_back                    rolling_back
  ↓                       ↓                                ↓
idle                    重启 worker                       rolled_back
                          ↓                                  ↓
                        committed / failed                  idle
```

---

## 6. 文件传输

### 6.1 FileConnector trait

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

### 6.2 实现

| 实现 | 协议 | 说明 |
|------|------|------|
| `SshFileConnector` | SSH/SFTP | 通过 SSH 通道的 SFTP |
| `SftpFileConnector` | SFTP | 独立 SFTP 连接 |
| `MysqlFileConnector` | MySQL | 数据库导出/导入 |
| `S3FileConnector` | S3/MinIO | 对象存储操作 |
| `DockerFileConnector` | Docker | 容器内文件操作 |
| `LocalFileConnector` | 本地 | Agent 本机文件 |

### 6.3 传输写入策略

```text
写入临时文件：{target}.rex.part
  ↓
完成后校验大小和 SHA256
  ↓
校验通过 → 原子 rename 替换目标文件
校验失败 → 保留或清理临时文件
```

### 6.4 TransferCoordinator

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

## 7. API 端点

### 7.1 Agent 相关

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/api/agent/download?os={os}&arch={arch}` | 下载指定平台的 Agent 二进制 |
| `POST` | `/api/agent/register` | Agent 注册（首次连接） |
| `WS` | `/ws/agent` | Agent WebSocket 隧道 |

### 7.2 健康检查

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/healthz` | Hub 健康检查，返回 200 表示正常 |

### 7.3 文件传输

| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/api/transfers` | 创建传输任务 |
| `GET` | `/api/transfers` | 查询传输任务列表 |
| `GET` | `/api/transfers/{id}` | 查询单个任务详情 |
| `POST` | `/api/transfers/{id}/cancel` | 取消传输任务 |
| `POST` | `/api/transfers/{id}/pause` | 暂停传输任务 |
| `POST` | `/api/transfers/{id}/resume` | 恢复传输任务 |
| `WS` | `/ws/transfers` | 传输进度实时推送 |

---

## 8. 配置文件

### 8.1 Hub 配置

```yaml
# hub.yaml
listen: ":3000"
data_dir: "./data"
secret_key: "${REX_SECRET_KEY}"
tls:
  cert: ""
  key: ""
update:
  enabled: true
  check_interval: 86400  # 秒
  github_repo: "owner/rex-hub"
```

### 8.2 Agent 配置

```yaml
# agent.yaml
server: "https://hub.example.com"
token: "rex_env_xxx"
name: "内网 Agent"
data_dir: "./data"
auto_update: true
```

### 8.3 agent.json（Agent 身份持久化）

```json
{
  "id": "agt_7x8k9m",
  "name": "树莓派 Agent",
  "token": "rex_env_xxx",
  "created_at": "2024-06-01T00:00:00Z"
}
```

---

## 9. 目录结构

### 9.1 Hub 二进制部署

```text
/opt/rex-hub/
├── rex-hub                当前运行的二进制
├── hub.yaml               配置文件
├── data/
│   ├── certs/             TLS 证书
│   ├── queries/           保存的 SQL 查询
│   └── update-state.json  更新状态
├── update/
│   ├── staging/           待替换的新版本
│   └── rollback/          旧版本备份
└── logs/
```

### 9.2 Agent 二进制部署

```text
/opt/rex-agent/
├── rex-agent              当前运行的二进制
├── agent.yaml             配置文件
├── data/
│   ├── agent.json         Agent 身份
│   └── update-state.json  更新状态
├── update/
│   ├── staging/           待替换的新版本
│   └── rollback/          旧版本备份
└── logs/
```

### 9.3 Docker 部署

```text
/app/
├── rex-hub / rex-agent    容器内唯一二进制
├── data/                   挂载点 → Docker volume
│   ├── agent.json
│   ├── update-state.json
│   └── ...
└── update/
    ├── staging/
    └── rollback/
```

---

## 10. Docker 信号处理

### 10.1 PID 1 信号转发

```text
docker stop → SIGTERM → PID 1 (supervisor)
  ↓
supervisor 转发 SIGTERM → worker 子进程
  ↓
worker 优雅关闭（关闭 WebSocket 连接、释放资源）
  ↓
worker 退出
  ↓
supervisor 退出
  ↓
容器停止
```

### 10.2 超时处理

```text
docker stop -t 30 → SIGTERM → 等待 30 秒 → SIGKILL
```

supervisor 收到 SIGTERM 后，应在 30 秒内完成 worker 关闭和自身退出。

---

## 11. Windows 差异

### 11.1 问题

- Windows 不支持 POSIX `fork + exec`
- 运行中的 `.exe` 文件通常被锁定，不能直接替换

### 11.2 解决方案

```text
用户启动 rex-hub.exe
  ↓
首次启动时复制自身到 data/rex-supervisor.exe
  ↓
rex-supervisor.exe 常驻运行（supervisor 角色）
  ↓
rex-supervisor.exe 启动 data/rex-worker.exe（worker 角色）
  ↓
更新时：
  rex-supervisor.exe 下载新版 → data/rex-worker.new.exe
  停止旧 worker.exe → 替换为新 worker.exe → 启动新 worker.exe
  supervisor 副本不变，不需要在运行时替换自身
```

### 11.3 对用户的入口

仍然是单个命令：`rex-hub.exe` 或 `rex-agent.exe`

首次启动后，用户会看到 data 目录下多了 `rex-supervisor.exe` 和 `rex-worker.exe`。

---

## 12. 前端工程结构

### 12.1 前端包

前端作为独立 Vue 3 + Vite 包开发，由 Hub 在构建或启动时嵌入/托管。

```text
packages/
└── rex-console-web/
    ├── package.json
    ├── index.html
    ├── src/
    │   ├── main.ts
    │   ├── App.vue
    │   ├── router.ts
    │   ├── api/
    │   ├── stores/
    │   ├── components/
    │   ├── layouts/
    │   ├── pages/
    │   ├── styles/
    │   └── i18n/
    ├── public/
    └── vite.config.ts
```
~
Hub 二进制只负责静态资源托管和 API 服务；前端不持有敏感凭据，不中转文件数据。

### 12.2 页面路由

| 页面 | 路由 | 布局 | 说明 |
|------|------|------|------|
| `login.html` | `/login` | 全屏 | 登录认证 |
| `dashboard.html` | `/dashboard` | 标准布局 | 仪表盘 |
| `environments.html` | `/environments` | 标准布局 | 环境列表 |
| `environment.html` | `/environments/:id` | 标准布局 | 环境详情 |
| `environment-new.html` | `/environments/new` | 标准布局 | 创建环境 |
| `resource-new.html` | `/resources/new` | 标准布局 | 创建资源向导 |
| `app.html` | `/workspace` | 工作区布局 | 多标签分屏 |
| `terminal.html` | `/terminal` | 全屏 | SSH 终端 |
| `sql.html` | `/sql` | 全屏 | SQL 控制台 |
| `files.html` | `/files` | 全屏 | 文件管理 |
| `agents.html` | `/agents` | 标准布局 | Agent 管理 |
| `audit-log.html` | `/audit-log` | 标准布局 | 审计日志 |
| `settings.html` | `/settings` | 标准布局 | 个人设置 |

原型阶段的独立 HTML 文件可以作为页面级参考，但正式实现应迁移为 Vue 路由和组件。

### 12.3 全局组件

建议沉淀以下通用组件：

| 组件 | 说明 |
|------|------|
| `AppLayout` | 桌面侧边栏 + 移动端底部导航 |
| `FullScreenLayout` | 终端、SQL、文件管理等全屏页面 |
| `WorkspaceLayout` | 多标签 + 分屏工作区 |
| `ResourceIcon` | 协议图标、颜色、状态点 |
| `ContextMenu` | 统一右键菜单，支持 divider、danger、submenu |
| `ConfirmDialog` | 删除、断开、重启、重置令牌等确认弹窗 |
| `ToastProvider` | 全局 Toast |
| `ThemeToggle` | 深色/浅色/跟随系统切换 |
| `I18nSwitch` | 中文/英文切换 |
| `TransferQueuePanel` | 文件传输队列 |
| `VersionOverview` | Hub 和 Agent 版本总览 |

### 12.4 全局状态

前端状态建议按模块拆分：

```text
stores/
├── auth.ts        登录态、会话过期、登出
├── user.ts        用户设置、主题、语言
├── env.ts         环境列表、环境详情
├── resource.ts    资源列表、连接方式、凭据引用
├── agent.ts       Agent 列表、在线状态、版本、平台
├── workspace.ts   标签页、分屏布局、连接状态
├── transfer.ts    传输任务、进度、冲突处理
├── audit.ts       审计日志筛选条件和列表
└── ui.ts          Toast、Modal、Loading
```

状态数据优先从 Hub API 获取，本地只做 UI 状态缓存。敏感字段不在 localStorage 中明文保存。

---

## 13. 工作区实现

### 13.1 标签页模型

```ts
interface WorkspaceTab {
  id: string;
  resourceId: string;
  protocol: ResourceProtocol;
  title: string;
  status: 'connecting' | 'online' | 'offline' | 'error';
  layoutSlot?: number;
  component: 'Terminal' | 'SqlConsole' | 'FileManager' | 'Docker' | 'Redis' | 'S3';
}
```

去重规则：相同 `resourceId + protocol` 默认只保留一个标签；如果用户选择“复制标签”，可以打开同资源的第二个会话。

### 13.2 分屏布局

布局状态保存在前端：

```ts
interface LayoutState {
  mode: 'single' | 'left-right' | 'top-bottom' | 'quad' | 'sidebar-main';
  slots: Record<number, string | null>; // slotId -> tabId
}
```

拖拽标签时只更新 `LayoutState.slots`，不重建连接。

### 13.3 连接菜单

`Ctrl+K` / `Ctrl+N` 打开全局搜索菜单：

- 搜索资源名称、地址、协议、环境名称。
- 按环境分组展示。
- Enter 连接选中资源。
- 支持“在新标签中连接”。

### 13.4 右键菜单

所有页面统一使用 `ContextMenu` 组件。菜单项结构：

```ts
interface MenuItem {
  label: string;
  icon?: string;
  disabled?: boolean;
  danger?: boolean;
  divider?: boolean;
  sub?: MenuItem[];
  action?: () => void;
}
```

支持嵌套 submenu，例如“发送到…” → 目标连接列表。

---

## 14. SSH 终端实现

### 14.1 xterm.js 集成

终端页面包含：

- xterm.js 实例
- WebSocket 数据通道
- 工具栏
- SFTP 面板
- 移动端浮动工具栏
- 右键菜单

终端尺寸变化时需要通知后端：

```ts
socket.send(JSON.stringify({
  type: 'resize',
  cols: terminal.cols,
  rows: terminal.rows,
}));
```

### 14.2 SSH 内置 SFTP

SSH 终端通过同一个 SSH 连接打开 SFTP 面板：

```text
SSH 会话建立
  ↓
检测是否支持 SFTP subsystem
  ↓
支持 → 显示工具栏「📁 SFTP」
  ↓
点击后打开 SFTP 面板，复用当前 FileConnector
```

独立 SFTP 标签和 SSH 内置 SFTP 面板应共享同一个文件传输入口，允许互相拖拽传输。

### 14.3 移动端方向键

移动端工具栏提供：

- ↑ ↓ ← →
- Tab
- Enter
- ^C
- ^L
- 历史
- 粘贴
- 缩放

按键通过 WebSocket 发送为控制字符或转义序列：

```ts
const KEYMAP: Record<string, string> = {
  ArrowUp: '\x1b[A',
  ArrowDown: '\x1b[B',
  ArrowRight: '\x1b[C',
  ArrowLeft: '\x1b[D',
  Tab: '\t',
  Enter: '\r',
  CtrlC: '\x03',
  CtrlL: '\x0c',
};
```

### 14.4 Bash 历史记录选择

前端维护最近输入历史：

```ts
interface TerminalHistoryItem {
  id: string;
  command: string;
  cwd?: string;
  createdAt: number;
}
```

移动端“历史”按钮打开搜索弹窗：

- 输入过滤历史命令
- 点击后发送命令文本到终端
- 可选择插入到当前输入行，不立即执行

目录补全建议优先由 shell 自身完成；前端只在 Tab 键、方向键、历史选择上提供辅助输入能力，避免模拟复杂 shell 行为。

### 14.5 终端安全

- 粘贴大段内容前提示确认。
- 不保存终端输出中的敏感内容到本地日志。
- 断开连接前确认，避免误关正在运行的任务。
- 审计日志记录“建立/断开 SSH 会话”，默认不记录完整命令；如需命令审计，应提供单独开关并提示风险。

---

## 15. SQL 控制台实现

### 15.1 页面结构

SQL 控制台由以下区域组成：

```text
SqlConsole
├── SqlTopBar
├── QueryTabs
├── SchemaSidebar
├── SqlEditor
├── SqlResultGrid
├── GlobalQueryModal
└── AiAssistantDrawer
```

### 15.2 查询标签

```ts
interface QueryTab {
  id: string;
  title: string;
  savedPath?: string;
  content: string;
  dirty: boolean;
  database?: string;
}
```

保存查询文件时写入 Hub 数据目录，例如：

```text
{data-dir}/queries/{environment-id}/{query-id}.sql
```

### 15.3 结果集

后端返回分页结果，避免一次性加载超大结果集：

```ts
interface QueryResultPage {
  columns: QueryColumn[];
  rows: Array<Record<string, unknown>>;
  totalRows: number;
  page: number;
  pageSize: number;
  elapsedMs: number;
}
```

### 15.4 AI 助手安全

AI 助手只作为辅助：

- 默认只读建议，不直接执行生成 SQL。
- 生成 SQL 必须进入编辑器，由用户确认后执行。
- 对 `DELETE`、`DROP`、`TRUNCATE`、`UPDATE` 等高风险语句，在执行前弹出确认框。
- 审计日志记录“AI 生成 SQL 被用户执行”。

---

## 16. 文件传输实现

### 16.1 前端交互

文件传输页面和标签页只负责：

- 创建任务
- 选择源和目标
- 展示进度
- 暂停/恢复/取消
- 处理冲突

文件数据不经过浏览器。

### 16.2 创建传输任务

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

响应：

```json
{
  "id": "xfer_abc",
  "status": "pending",
  "progress": {
    "totalBytes": 0,
    "transferredBytes": 0,
    "speedBytesPerSec": 0,
    "eta": null
  }
}
```

### 16.3 进度推送

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

### 16.4 冲突处理

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

### 16.5 跨连接传输路径

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

---

## 17. Agent 管理实现

### 17.1 Agent 页面功能

Agent 页面需要实现：

- Agent 卡片列表
- 在线/离线状态
- 平台标签：OS + arch
- 当前版本
- Agent ID
- 连接 IP
- 延迟、运行时间、传输量
- 配置弹窗
- 日志查看器
- 重启 Agent
- 重置令牌
- 部署指南
- 二进制下载按钮

### 17.2 Agent 下载端点

Hub 需要内置各平台 Agent 二进制，或从部署包目录读取。

```http
GET /api/agent/download?os={os}&arch={arch}
```

支持平台：

| os | arch | 示例 |
|----|------|------|
| `linux` | `amd64` | Linux x86_64 |
| `linux` | `arm64` | Linux ARM64 |
| `linux` | `armv7l` | Raspberry Pi 32-bit |
| `darwin` | `arm64` | Apple Silicon |
| `darwin` | `amd64` | Intel macOS |
| `windows` | `amd64` | Windows x86_64 |

响应头建议：

```http
Content-Type: application/octet-stream
Content-Disposition: attachment; filename="rex-agent"
X-REX-Version: v0.1.0
X-REX-SHA256: <sha256>
```

### 17.3 Agent 注册

首次连接：

```http
POST /api/agent/register
```

请求：

```json
{
  "token": "rex_env_xxx",
  "name": "内网 Agent",
  "version": "v0.1.0",
  "sha256": "<agent-binary-sha256>",
  "platform": {
    "os": "linux",
    "arch": "arm64",
    "hostname": "raspberrypi",
    "osVersion": "Ubuntu 22.04"
  }
}
```

响应：

```json
{
  "id": "agt_7x8k9m",
  "name": "内网 Agent",
  "token": "rex_env_xxx",
  "created_at": "2024-06-01T00:00:00Z"
}
```

Agent 将响应保存到 `agent.json`。

### 17.4 Agent 心跳

```http
WS /ws/agent
```

心跳消息：

```json
{
  "type": "heartbeat",
  "agentId": "agt_7x8k9m",
  "version": "v0.1.0",
  "sha256": "<agent-binary-sha256>",
  "resources": ["res_ssh_1", "res_mysql_1"],
  "metrics": {
    "latencyMs": 12,
    "rxBytes": 1024,
    "txBytes": 2048
  }
}
```

Hub 校验版本和 SHA256：

- 匹配：返回正常。
- 不匹配：返回需要更新、下载 URL、目标 SHA256。
- 版本策略：只允许与 Hub 部署包一致的 Agent 版本。

### 17.5 重置令牌

重置令牌只修改 Hub 端环境配置，并通知当前在线 Agent 断开：

```text
用户点击重置令牌
  ↓
生成新 token
  ↓
保存新 token
  ↓
向当前 Agent 发送 disconnect / token_revoked
  ↓
旧 Agent 无法重新连接
  ↓
新 Agent 使用新 token 注册
```

---

## 18. 审计日志实现

### 18.1 审计事件

```ts
interface AuditEvent {
  id: string;
  time: string;
  user: 'admin';
  environmentId?: string;
  resourceId?: string;
  agentId?: string;
  type:
    | 'login'
    | 'logout'
    | 'connect'
    | 'disconnect'
    | 'sql_query'
    | 'file_upload'
    | 'file_download'
    | 'file_delete'
    | 'ssh_command'
    | 'agent_restart'
    | 'token_reset'
    | 'update_check'
    | 'update_apply'
    | 'update_rollback';
  result: 'success' | 'failure' | 'canceled';
  summary: string;
  detail?: Record<string, unknown>;
}
```

### 18.2 查询接口

```http
GET /api/audit-log?from=2024-06-01T00:00:00Z&to=2024-06-02T00:00:00Z&type=file_download
```

响应：

```json
{
  "items": [],
  "total": 128,
  "page": 1,
  "pageSize": 50
}
```

### 18.3 安全策略

- 删除文件、断开连接、重置令牌、重启 Agent 必须二次确认。
- 文件删除默认进入审计日志。
- SQL 高危语句执行前确认。
- AI 生成 SQL 不自动执行。
- 配置中的密码、token、secret 必须加密存储。
- 日志导出 CSV 时隐藏敏感字段。

---

## 19. 更新实现细节

### 19.1 Hub Release 结构

GitHub Release 建议包含：

```text
rex-hub-v0.2.0-linux-amd64
rex-hub-v0.2.0-linux-arm64
rex-hub-v0.2.0-darwin-arm64
rex-hub-v0.2.0-darwin-amd64
rex-hub-v0.2.0-windows-amd64.exe
rex-hub-v0.2.0-SHA256SUMS
```

Hub worker 下载对应平台二进制和 `SHA256SUMS`，校验通过后才进入替换流程。

### 19.2 Agent 下载包

Hub 部署包需要包含同版本 Agent 二进制：

```text
agent-binaries/
├── rex-agent-linux-amd64
├── rex-agent-linux-arm64
├── rex-agent-linux-armv7l
├── rex-agent-darwin-arm64
├── rex-agent-darwin-amd64
└── rex-agent-windows-amd64.exe
```

Hub 在 `/api/agent/download` 中读取对应文件，并返回 SHA256。

### 19.3 原子替换

Unix/Linux/macOS：

```text
当前二进制：/opt/rex-hub/rex-hub
staging：/opt/rex-hub/update/staging/rex-hub.v0.2.0
rollback：/opt/rex-hub/update/rollback/rex-hub.v0.1.0
```

替换流程：

```text
rename 当前二进制 → rollback
rename staging → 当前二进制
chmod +x 当前二进制
启动新 worker
```

### 19.4 更新状态写入要求

`update-state.json` 必须原子写入，避免 supervisor 读到半写入文件：

```text
写 update-state.json.tmp
fsync 临时文件
rename update-state.json.tmp → update-state.json
```

### 19.5 避免更新死循环

父进程启动新 worker 时：

```text
REX_UPDATE_PENDING=1
REX_TARGET_VERSION=v0.2.0
```

worker 检测到 `REX_UPDATE_PENDING=1` 时：

- 不检查 GitHub / Hub 更新。
- 只执行健康检查。
- 健康通过后写入 `phase=committed`。
- 健康失败则退出，让 supervisor 增加 attempt。

---

## 20. 配置与目录约定

### 20.1 Hub 配置

```yaml
# hub.yaml
listen: ":3000"
data_dir: "./data"
secret_key: "${REX_SECRET_KEY}"
tls:
  cert: ""
  key: ""
update:
  enabled: true
  check_interval: 86400
  github_repo: "owner/rex-hub"
```

### 20.2 Agent 配置

```yaml
# agent.yaml
server: "https://hub.example.com"
token: "rex_env_xxx"
name: "内网 Agent"
data_dir: "./data"
auto_update: true
```

### 20.3 Hub 数据目录

```text
{data-dir}/
├── certs/
├── queries/
├── audit-log.db
├── settings.json
├── agent-binaries/
└── update/
    ├── staging/
    └── rollback/
```

### 20.4 Agent 数据目录

```text
{data-dir}/
├── agent.json
├── update-state.json
└── logs/
```

---

## 21. 原型到正式实现的迁移建议

### 21.1 保留原型中的交互设计

以下原型交互应保留为正式实现需求：

- 标签右键菜单
- 全局连接菜单
- 多标签分屏
- SSH 终端内置 SFTP
- 移动端方向键和历史选择
- SQL 右键菜单
- 文件右键“发送到…”
- Agent 页面二进制下载按钮
- 设置页版本总览
- 深色/浅色/跟随系统主题
- 中文/英文 i18n

### 21.2 原型文件角色

原型 HTML 文件用于交互验证，不建议直接作为最终产品代码：

```text
prototype/
├── dashboard.html
├── environments.html
├── environment.html
├── terminal.html
├── sql.html
├── files.html
├── agents.html
├── audit-log.html
├── settings.html
└── shared.js
```

正式实现时，应将交互拆成 Vue 组件、stores、API client 和主题/i18n 模块。

### 21.3 第一阶段交付范围

第一阶段建议优先完成：

1. Hub 单二进制启动。
2. 父进程 supervisor + 1 个 worker。
3. 登录、环境、资源、工作区基础连接。
4. SSH 终端、SQL、文件管理。
5. Agent 单二进制启动和连接。
6. Agent 页面显示状态和版本。
7. 文件传输不经过浏览器。
8. 审计日志记录关键操作。

### 21.4 第二阶段交付范围

第二阶段增加：

1. Hub GitHub Release 检查。
2. Agent 从 Hub 下载二进制。
3. SHA256 校验。
4. `update-state.json` 状态机。
5. supervisor 替换二进制和回滚。
6. `REX_UPDATE_PENDING` 防死循环。
7. 设置页版本总览和更新提示。
8. Docker 内同样使用 supervisor + worker，不需要 s6-overlay。

---

## 22. 后端工程结构

### 22.1 仓库结构

建议后端使用 Rust workspace：

```text
rex-hub/
├── Cargo.toml
├── README.md
├── docker/
│   ├── Dockerfile.hub
│   └── Dockerfile.agent
├── crates/
│   ├── rex-common/
│   ├── rex-ssh/
│   ├── rex-mysql/
│   ├── rex-postgresql/
│   ├── rex-redis/
│   ├── rex-docker/
│   ├── rex-sqlite/
│   ├── rex-s3/
│   ├── rex-transfer/
│   ├── rex-tunnel/
│   ├── rex-supervisor/
│   ├── rex-hub/
│   └── rex-agent/
├── packages/
│   └── rex-console-web/
└── docs/
```

### 22.2 Workspace 依赖

根 `Cargo.toml` 定义 workspace 和共享依赖版本：

```toml
[workspace]
members = [
  "crates/rex-common",
  "crates/rex-ssh",
  "crates/rex-mysql",
  "crates/rex-postgresql",
  "crates/rex-redis",
  "crates/rex-docker",
  "crates/rex-sqlite",
  "crates/rex-s3",
  "crates/rex-transfer",
  "crates/rex-tunnel",
  "crates/rex-supervisor",
  "crates/rex-hub",
  "crates/rex-agent",
]
resolver = "2"

[workspace.package]
edition = "2021"
license = "MIT"
repository = "https://github.com/owner/rex-hub"

[workspace.dependencies]
anyhow = "1"
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
uuid = { version = "1", features = ["v4", "serde"] }
```

### 22.3 `rex-common`

`rex-common` 放所有 crate 共用的类型和错误：

```text
rex-common/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── config.rs
    ├── error.rs
    ├── platform.rs
    ├── version.rs
    ├── id.rs
    └── time.rs
```

建议类型：

```rust
pub enum ResourceProtocol {
    Ssh,
    Sftp,
    Mysql,
    Postgresql,
    Redis,
    Docker,
    Sqlite,
    S3,
}

pub enum ConnectionMode {
    AgentProxy,
    Direct,
}

pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub os_version: Option<String>,
}
```

### 22.4 `rex-hub`

Hub 二进制入口：

```text
rex-hub/
└── src/
    ├── main.rs
    ├── cli.rs
    ├── config.rs
    ├── routes.rs
    ├── services/
    ├── models/
    ├── db.rs
    ├── auth.rs
    ├── agent.rs
    ├── update.rs
    └── static_assets.rs
```

职责：

- 启动 supervisor。
- 启动 worker。
- 托管前端静态资源。
- 提供 HTTP API。
- 提供 WebSocket 隧道。
- 管理环境、资源、Agent、审计日志。
- 第二阶段负责 Hub 更新检查和 Agent 二进制分发。

### 22.5 `rex-agent`

Agent 二进制入口：

```text
rex-agent/
└── src/
    ├── main.rs
    ├── cli.rs
    ├── config.rs
    ├── agent.rs
    ├── identity.rs
    ├── heartbeat.rs
    ├── update.rs
    └── resources.rs
```

职责：

- 启动 supervisor。
- 启动 worker。
- 加载或创建 `agent.json`。
- 主动连接 Hub。
- 维护 WebSocket 心跳。
- 代理 SSH、数据库、文件、Docker 等资源连接。
- 第二阶段从 Hub 下载同版本 Agent 二进制并自更新。

### 22.6 协议 crate 边界

每个协议 crate 只负责协议实现，不依赖 Hub 或 Agent 业务层。

```text
rex-ssh/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── client.rs
    ├── sftp.rs
    ├── terminal.rs
    └── error.rs
```

协议 crate 输出统一能力：

```rust
pub trait ResourceConnector: Send + Sync {
    fn protocol(&self) -> ResourceProtocol;
    fn connect(&self, config: ResourceConfig) -> impl Future<Output = Result<Connection>>;
}
```

Hub 和 Agent 都通过同一套协议 crate 建立连接，区别只在于连接入口：

- Hub 直连资源：Hub worker 直接连接目标。
- Agent 代理资源：Hub 通过 Agent WebSocket 隧道请求 Agent worker 连接目标。

---

## 23. 数据模型

### 23.1 存储选择

第一阶段建议使用 SQLite 作为 Hub 本地数据库：

```text
{data-dir}/hub.db
```

原因：

- 单文件，适合自托管。
- 部署简单。
- 足够支撑个人使用场景。
- 后续可替换为 PostgreSQL，不影响业务模型。

### 23.2 核心表

```sql
CREATE TABLE environments (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  connection_mode TEXT NOT NULL,
  agent_token_hash TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE agents (
  id TEXT PRIMARY KEY,
  environment_id TEXT NOT NULL,
  name TEXT NOT NULL,
  token_hash TEXT NOT NULL,
  version TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  os TEXT NOT NULL,
  arch TEXT NOT NULL,
  hostname TEXT,
  os_version TEXT,
  status TEXT NOT NULL,
  last_seen_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE resources (
  id TEXT PRIMARY KEY,
  environment_id TEXT NOT NULL,
  name TEXT NOT NULL,
  protocol TEXT NOT NULL,
  connection_mode TEXT NOT NULL,
  agent_id TEXT,
  config_json TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE audit_log (
  id TEXT PRIMARY KEY,
  time TEXT NOT NULL,
  user TEXT NOT NULL,
  environment_id TEXT,
  resource_id TEXT,
  agent_id TEXT,
  type TEXT NOT NULL,
  result TEXT NOT NULL,
  summary TEXT NOT NULL,
  detail_json TEXT
);
```

### 23.3 凭据加密

资源密码、SSH 私钥密码、Agent token 等敏感字段不应明文存库。

建议流程：

```text
REX_SECRET_KEY
  ↓
派生加密密钥
  ↓
AES-256-GCM 加密资源配置中的敏感字段
  ↓
config_json 中只保存密文、nonce、算法标识
```

敏感字段包括：

- SSH 密码
- SSH 私钥密码
- MySQL 密码
- PostgreSQL 密码
- Redis 密码
- S3 secret key
- Agent token hash 的原始 token

### 23.4 资源配置结构

```json
{
  "host": "192.0.2.1",
  "port": 22,
  "username": "pi",
  "auth": {
    "type": "password",
    "passwordEncrypted": "..."
  },
  "terminal": {
    "encoding": "utf-8",
    "keepAliveSeconds": 60
  }
}
```

---

## 24. API 设计规范

### 24.1 认证

第一阶段可以使用单用户登录：

```http
POST /api/auth/login
```

请求：

```json
{
  "username": "admin",
  "password": "password"
}
```

响应：

```json
{
  "token": "rex_session_xxx",
  "expiresAt": "2026-06-17T00:00:00Z"
}
```

认证方式：

- 登录成功后设置 HttpOnly cookie，或返回 bearer token。
- 前端请求携带 cookie / Authorization header。
- 所有管理 API 必须认证。
- `/healthz` 可以公开，只返回基础健康状态。

### 24.2 统一错误响应

```json
{
  "error": {
    "code": "RESOURCE_NOT_FOUND",
    "message": "资源不存在",
    "details": {}
  }
}
```

建议错误码：

```text
AUTH_REQUIRED
AUTH_INVALID
RESOURCE_NOT_FOUND
ENVIRONMENT_NOT_FOUND
AGENT_NOT_FOUND
AGENT_OFFLINE
CONNECTION_FAILED
TRANSFER_NOT_FOUND
UPDATE_NOT_AVAILABLE
UPDATE_FAILED
VALIDATION_ERROR
INTERNAL_ERROR
```

### 24.3 分页响应

```json
{
  "items": [],
  "page": 1,
  "pageSize": 50,
  "total": 128
}
```

### 24.4 WebSocket 消息

统一消息结构：

```ts
interface REXMessage<T = unknown> {
  id?: string;
  type: string;
  payload: T;
}
```

示例：

```json
{
  "type": "terminal.data",
  "payload": {
    "data": "base64-or-utf8-stream"
  }
}
```

---

## 25. 连接通道实现

### 25.1 直连资源

```text
浏览器
  ↓ WebSocket
Hub worker
  ↓ 协议 crate
目标资源
```

适用：

- 公网 SSH
- 云数据库
- 公网 S3/MinIO
- Hub 所在网络可直接访问的资源

### 25.2 Agent 代理资源

```text
浏览器
  ↓ WebSocket
Hub worker
  ↓ Agent WebSocket 隧道
Agent worker
  ↓ 协议 crate
内网目标资源
```

适用：

- 家庭内网服务器
- 公司内网数据库
- 没有公网 IP 的 NAS
- 不允许开放入站端口的设备

### 25.3 通道协议

Hub 与 Agent 之间通过 WebSocket 隧道传输：

```json
{
  "type": "resource.connect",
  "payload": {
    "requestId": "req_abc",
    "resourceId": "res_ssh_1",
    "protocol": "ssh",
    "config": {}
  }
}
```

Agent 响应：

```json
{
  "type": "resource.connected",
  "payload": {
    "requestId": "req_abc",
    "channelId": "ch_123"
  }
}
```

后续数据通过 `channelId` 复用同一条 WebSocket。

---

## 26. Docker 构建与部署实现

### 26.1 Hub Dockerfile

```dockerfile
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY rex-hub /usr/local/bin/rex-hub
WORKDIR /app

VOLUME ["/app/data"]
EXPOSE 3000

ENTRYPOINT ["rex-hub"]
```

### 26.2 Agent Dockerfile

```dockerfile
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY rex-agent /usr/local/bin/rex-agent
WORKDIR /app

VOLUME ["/app/data"]

ENTRYPOINT ["rex-agent"]
```

### 26.3 Docker 内更新限制

Docker 内可以更新二进制文件，但不能更新“当前镜像”。

因此 Docker 部署下的自动更新语义是：

- 容器内二进制可以替换、重启 worker。
- 镜像本身不会自动变成新版本。
- 用户后续仍需要手动 `docker pull` / 重启镜像，或者由外部部署系统完成镜像更新。
- REX 内部只保证当前容器内二进制和 worker 的更新/回滚。

这一点需要在前端更新提示中区分：

- Hub Docker：提示“检测到新版本，点击后下载二进制并重启容器内进程；镜像仍需手动更新”。
- Hub 二进制：提示“检测到新版本，点击后替换二进制并重启”。

### 26.4 Docker 停止流程

```text
docker stop
  ↓
SIGTERM 发给 PID 1
  ↓
supervisor 设置 stopping
  ↓
supervisor 发送 SIGTERM 给 worker
  ↓
worker 关闭连接、刷新状态
  ↓
worker 退出
  ↓
supervisor 退出
```

实现要求：

- supervisor 必须处理 `SIGTERM`。
- supervisor 不能忽略 Docker stop。
- worker 必须在 30 秒内退出。
- 如果 worker 不退出，supervisor 可以发送 `SIGKILL`，然后自身退出。

---

## 27. 开发任务规划

### 27.1 里程碑 0：项目骨架

#### 功能目标

建立可编译、可提交、可扩展的 Rust workspace 骨架，为后续 Hub / Agent 开发提供稳定入口和进程模型基础。

本里程碑只交付：

- 根 `Cargo.toml` workspace 配置。
- `crates/rex-common` 通用 crate。
- `crates/rex-hub` Hub 二进制入口。
- `crates/rex-agent` Agent 二进制入口。
- 基础 CLI 参数解析。
- `--worker` 入口区分。
- 第一阶段 supervisor：父进程启动 worker、等待退出、worker 退出后重启。
- 基础 `tracing` 日志。
- 可运行的验证命令。

本里程碑不交付：

- HTTP API。
- SQLite 数据库。
- 登录认证。
- Agent 注册 / 心跳。
- SSH / SFTP / SQL / 文件传输协议实现。
- 自动更新。
- 前端正式工程。

#### 产品边界

- 保持 REX Hub 单用户、自托管定位，不引入多用户、RBAC、企业协作概念。
- Hub / Agent 均采用“单二进制 + supervisor + worker”模型。
- 第一阶段 supervisor 只负责启动 worker、监控退出、重启 worker；不负责自动更新。
- worker 数量为 1，不引入多 worker 调度。
- Agent 仍保持单二进制目标，不在本阶段拆分协议子进程。
- 文件传输、协议连接、前端工作区等行为只在文档中预留边界，不在本阶段实现。

#### 文件结构

第一阶段骨架结构：

```text
.
├── Cargo.toml
├── README.md
├── docs/
│   ├── PRODUCT.md
│   └── DEVELOPMENT.md
├── prototype/
└── crates/
    ├── rex-common/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── cli.rs
    │       └── supervisor.rs
    ├── rex-hub/
    │   ├── Cargo.toml
    │   └── src/
    │       └── main.rs
    └── rex-agent/
        ├── Cargo.toml
        └── src/
            └── main.rs
```

根 `Cargo.toml` 负责声明 workspace 成员和共享依赖版本。

crate 内部依赖必须使用：

```toml
[dependencies]
rex-common = { workspace = true }
tracing = { workspace = true }
```

不得在子 crate 中重复声明依赖版本。

#### 接口设计

##### CLI

Hub 和 Agent 共享基础 CLI 解析：

```rust
pub struct Cli {
    pub worker: bool,
}

pub fn parse_cli() -> Cli;
```

行为：

- 包含 `--worker` 时进入 worker 模式。
- 不包含 `--worker` 时进入 supervisor 模式。
- 其他未知参数在本阶段返回错误并退出。
- `rex-hub --worker` 和 `rex-agent --worker` 只打印日志并退出，便于测试。

##### supervisor

`rex-common::supervisor` 提供第一阶段 supervisor 能力：

```rust
pub struct SupervisorConfig {
    pub worker_arg: String,
    pub restart_delay: Duration,
}

pub fn run_supervisor(config: SupervisorConfig) -> anyhow::Result<()>;
```

行为：

- supervisor 使用当前可执行文件启动 worker。
- 启动时追加 `--worker` 参数。
- 等待 worker 退出。
- 打印 worker 退出状态。
- 延迟后重新启动 worker。
- 收到 `SIGTERM` 时停止重启并退出。

##### worker

worker 模式由 Hub / Agent 各自入口实现：

```rust
fn run_worker() {
    tracing::info!("rex-hub worker started");
}
```

Agent worker 同理，只打印不同日志。

#### 数据模型

本里程碑不引入持久化数据模型。

只允许存在内存状态：

- `Cli`：CLI 参数解析结果。
- `SupervisorConfig`：supervisor 运行配置。

不创建数据库、不写入配置文件、不生成运行状态文件。

#### 状态流转

第一阶段 supervisor 状态流转：

```text
supervisor start
  ↓
spawn worker --worker
  ↓
wait worker exit
  ↓
log exit status
  ↓
restart_delay
  ↓
spawn worker --worker
```

收到 `SIGTERM`：

```text
supervisor stopping
  ↓
不再启动新的 worker
  ↓
exit 0
```

worker 状态：

```text
worker start
  ↓
log started
  ↓
exit 0
```

#### 前端交互

本里程碑不实现正式前端。

后续前端开发时必须以 `prototype/*.html`、`prototype/shared.js`、`prototype/css/*.css` 为交互标准：

- 保留标签右键菜单。
- 保留全局连接菜单。
- 保留多标签分屏。
- 保留 SSH 终端内置 SFTP。
- 保留移动端方向键和历史选择。
- 保留 SQL 右键菜单。
- 保留文件右键“发送到…”。
- 保留 Agent 页面二进制下载按钮。
- 保留设置页版本总览。
- 保留深色/浅色/跟随系统主题。
- 保留中文/英文 i18n。

原型修改只能用于交互验证和标准校准，不应把原型 HTML 直接作为正式前端实现。正式前端后续按功能域组织：

```text
packages/rex-console-web/src/
├── features/
├── pages/
├── components/
├── api/
├── stores/
├── layouts/
├── styles/
└── i18n/
```

#### 后端流程

Hub 启动：

```text
rex-hub
  ↓
parse CLI
  ↓
--worker?
  ├─ yes → run_hub_worker()
  └─ no  → run_supervisor("--worker")
```

Agent 启动：

```text
rex-agent
  ↓
parse CLI
  ↓
--worker?
  ├─ yes → run_agent_worker()
  └─ no  → run_supervisor("--worker")
```

第一阶段 worker 不启动 HTTP、不连接 Hub、不建立协议连接，只验证进程模型和日志。

#### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。所有公开函数和关键逻辑路径必须有测试。

测试在开发之后、review 之前完成。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo run -p rex-hub
cargo run -p rex-agent
cargo run -p rex-hub -- --worker
cargo run -p rex-agent -- --worker
```

验证点：

- `cargo fmt --check` 通过。
- `cargo clippy --workspace --all-targets` 无阻塞问题。
- `cargo test --workspace` 通过。
- `rex-hub` 和 `rex-agent` 可分别编译。
- `rex-hub` 默认进入 supervisor 模式，能启动 worker 并重启。
- `rex-agent` 默认进入 supervisor 模式，能启动 worker 并重启。
- `rex-hub --worker` 只运行 worker 并退出。
- `rex-agent --worker` 只运行 worker 并退出。
- 子 crate 不重复声明共享依赖版本。
- 文档未引入多用户、RBAC、企业协作或自动更新实现。

#### 提交边界

本里程碑建议拆成 1 个提交：

```text
chore: add Rust workspace skeleton
```

提交内容：

- 根 `Cargo.toml`
- `crates/rex-common`
- `crates/rex-hub`
- `crates/rex-agent`
- 基础 CLI / supervisor 代码
- 最小单元测试

不得包含：

- HTTP 框架。
- 数据库迁移。
- 协议实现。
- 前端工程初始化。
- 自动更新代码。

### 27.2 里程碑 1：Hub 基础服务

目标：Hub 可以启动并提供基础 API。

任务：

1. 添加 HTTP 框架 + `/healthz`。
2. 添加配置加载：`hub.yaml` 和环境变量。
3. 添加 SQLite 初始化。
4. 添加环境 CRUD API。
5. 添加资源 CRUD API。
6. 添加登录 API。
7. 添加基础认证中间件。
8. 添加审计日志写入。

完成标准：

- Hub worker 启动后能监听端口。
- `/healthz` 返回 200。
- 可以创建环境。
- 可以创建资源。
- 未登录请求被拒绝。
- 登录成功请求通过。

#### 子任务 1.1：HTTP 框架 + `/healthz`

##### 功能目标

Hub worker 启动后监听 HTTP 端口，提供 `/healthz` 健康检查端点。验证 HTTP 框架集成和基础路由能力。

##### 产品边界

- `/healthz` 是公开端点，不需要认证。
- `/healthz` 只返回基础健康状态，不暴露内部版本或组件状态。
- 使用 Axum 作为 HTTP 框架（Tokio 生态，与项目 async-runtime 一致）。
- 监听端口从配置读取，默认 `:3000`。
- worker 模式下启动 HTTP 服务；supervisor 模式不变。
- 本子任务不实现认证中间件、数据库、配置文件加载。

##### 文件结构

```text
crates/rex-hub/
├── Cargo.toml        修改：添加 axum, tokio, tracing 依赖
└── src/
    ├── main.rs       修改：worker 模式启动 HTTP 服务
    └── routes.rs     新增：路由定义和 /healthz handler
```

##### 接口设计

```rust
// routes.rs
use axum::{routing::get, Router};

pub fn app() -> Router {
    Router::new().route("/healthz", get(healthz))
}

async fn healthz() -> &'static str {
    "ok"
}
```

##### 数据模型

本子任务不引入数据模型。

##### 状态流转

```text
rex-hub worker start
  ↓
init tracing
  ↓
create axum Router
  ↓
bind to 0.0.0.0:3000
  ↓
serve HTTP
  ↓
GET /healthz → 200 "ok"
```

##### 后端流程

```rust
// main.rs worker 模式
fn run_worker() {
    tracing::info!("rex-hub worker started");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, routes::app()).await.unwrap();
    });
}
```

##### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。所有公开函数和关键逻辑路径必须有测试。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
curl http://localhost:3000/healthz  # 手动验证返回 "ok"
```

验证点：

- `cargo fmt --check` 通过。
- `cargo clippy --workspace --all-targets` 无阻塞问题。
- `cargo test --workspace` 通过。
- `/healthz` 返回 HTTP 200 + body "ok"。
- supervisor 模式行为不变（重启 worker 能力保留）。

##### 提交边界

```text
feat: add axum HTTP server and /healthz endpoint
```

提交内容：

- `crates/rex-hub/src/routes.rs`
- `crates/rex-hub/src/main.rs`（worker 模式启动 HTTP）
- `crates/rex-hub/Cargo.toml`（添加依赖）
- 根 `Cargo.toml`（添加 tokio 依赖）

不得包含：

- 配置文件加载（子任务 1.2 实现）。
- SQLite 数据库（子任务 1.3 实现）。
- 认证中间件（子任务 1.7 实现）。

#### 子任务 1.2：配置加载

##### 功能目标

Hub worker 启动时加载配置文件 `hub.yaml`，支持环境变量覆盖。配置用于后续子任务的端口监听、数据目录、密钥等参数。

##### 产品边界

- 配置文件路径默认为当前目录 `hub.yaml`，可通过 `--config` 参数指定。
- 环境变量格式为 `REX_<FIELD>`，例如 `REX_LISTEN`、`REX_DATA_DIR`。
- 环境变量优先级高于配置文件。
- 配置文件不包含敏感信息（密码、密钥），敏感信息通过环境变量传入。
- 本子任务不实现 TLS 配置、更新配置。
- 使用 `serde` + `serde_yaml` 解析配置。

##### 文件结构

```text
crates/rex-hub/
└── src/
    ├── main.rs       修改：解析 --config 参数，加载配置
    └── config.rs     新增：HubConfig 结构体和加载逻辑
```

根 `Cargo.toml` 添加 `serde` 和 `serde_yaml` 依赖。

##### 接口设计

```rust
// config.rs
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct HubConfig {
    pub listen: String,
    pub data_dir: PathBuf,
    pub secret_key: String,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:3000".to_string(),
            data_dir: PathBuf::from("./data"),
            secret_key: String::new(),
        }
    }
}

impl HubConfig {
    pub fn load(config_path: Option<&str>) -> anyhow::Result<Self> {
        // 1. 读取配置文件
        // 2. 应用环境变量覆盖
        // 3. 返回最终配置
    }
}
```

CLI 扩展：

```rust
// main.rs
pub fn parse_cli() -> Cli {
    let args: Vec<String> = std::env::args().collect();
    let worker = args.contains(&"--worker".to_string());
    let config = args.iter().position(|a| a == "--config")
        .and_then(|i| args.get(i + 1).cloned());
    Cli { worker, config }
}
```

##### 数据模型

配置文件 `hub.yaml`：

```yaml
listen: ":3000"
data_dir: "./data"
secret_key: "${REX_SECRET_KEY}"
```

环境变量：

| 环境变量 | 配置字段 | 说明 |
|----------|----------|------|
| `REX_LISTEN` | `listen` | 监听地址，如 `0.0.0.0:3000` |
| `REX_DATA_DIR` | `data_dir` | 数据目录 |
| `REX_SECRET_KEY` | `secret_key` | 加密密钥 |

##### 状态流转

```text
parse CLI
  ↓
--config 提供？
  ├─ yes → 读取指定文件
  └─ no  → 读取 ./hub.yaml（若存在）
  ↓
文件不存在？
  ├─ yes → 使用默认配置
  └─ no  → serde_yaml 解析
  ↓
应用环境变量覆盖
  ↓
返回 HubConfig
  ↓
run_worker_async(config)
```

##### 后端流程

```rust
// main.rs
fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = parse_cli();
    let config = HubConfig::load(cli.config.as_deref())?;

    if cli.worker {
        let rt = tokio::runtime::Runtime::new()
            .context("failed to create tokio runtime")?;
        rt.block_on(run_worker_async(config));
    } else {
        run_supervisor(SupervisorConfig {
            worker_arg: "--worker".to_string(),
            restart_delay: Duration::from_secs(1),
        })?;
    }
    Ok(())
}
```

##### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。所有公开函数和关键逻辑路径必须有测试。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

测试用例：

```rust
#[test]
fn default_config() {
    let config = HubConfig::default();
    assert_eq!(config.listen, "0.0.0.0:3000");
    assert_eq!(config.data_dir, PathBuf::from("./data"));
}

#[test]
fn load_missing_file_uses_default() {
    let config = HubConfig::load(Some("/nonexistent/path")).unwrap();
    // 应使用默认值
}
```

##### 提交边界

```text
feat: add HubConfig loading with env override
```

提交内容：

- `crates/rex-hub/src/config.rs`
- `crates/rex-hub/src/main.rs`（CLI 扩展、config 参数传递）
- `crates/rex-common/src/cli.rs`（CLI 结构体扩展）
- 根 `Cargo.toml`（添加 serde、serde_yaml 依赖）

不得包含：

- SQLite 数据库（子任务 1.3 实现）。
- TLS 配置。
- 更新配置。

#### 子任务 1.3：SQLite 初始化

##### 功能目标

Hub worker 启动时初始化 SQLite 数据库，创建核心表结构。为后续子任务的环境 CRUD、资源 CRUD、审计日志提供存储基础。

##### 产品边界

- SQLite 数据库文件位于 `{data_dir}/hub.db`。
- 使用 `rusqlite` 作为 SQLite 驱动，`tokio::task::spawn_blocking` 异步封装。
- 表结构包括：`environments`、`agents`、`resources`、`audit_log`。
- 本子任务只初始化表结构，不实现 CRUD 操作。
- 数据库初始化失败时 worker 应启动失败（启动时检查）。

##### 文件结构

```text
crates/rex-hub/
└── src/
    ├── main.rs       修改：worker 模式调用 db 初始化
    ├── config.rs     已有：data_dir 字段
    └── db.rs         新增：SQLite 初始化和表创建

crates/rex-common/
└── src/
    └── db.rs         新增：通用数据库类型（可选）
```

根 `Cargo.toml` 添加 `rusqlite` 依赖。

##### 接口设计

```rust
// crates/rex-hub/src/db.rs
use std::path::Path;
use anyhow::{Context, Result};
use rusqlite::Connection;

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("failed to open database: {}", db_path.display()))?;
        Self::run_migrations(&conn)?;
        Ok(Self { conn })
    }

    fn run_migrations(conn: &Connection) -> Result<()> {
        conn.execute_batch(include_str!("migrations.sql"))
            .context("failed to run database migrations")?;
        Ok(())
    }
}
```

##### 数据模型

表结构（`migrations.sql`）：

```sql
CREATE TABLE IF NOT EXISTS environments (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    connection_mode TEXT NOT NULL,
    agent_token_hash TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    environment_id TEXT NOT NULL,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    version TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    os TEXT NOT NULL,
    arch TEXT NOT NULL,
    hostname TEXT,
    os_version TEXT,
    status TEXT NOT NULL,
    last_seen_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS resources (
    id TEXT PRIMARY KEY,
    environment_id TEXT NOT NULL,
    name TEXT NOT NULL,
    protocol TEXT NOT NULL,
    connection_mode TEXT NOT NULL,
    agent_id TEXT,
    config_json TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    time TEXT NOT NULL,
    user TEXT NOT NULL,
    environment_id TEXT,
    resource_id TEXT,
    agent_id TEXT,
    type TEXT NOT NULL,
    result TEXT NOT NULL,
    summary TEXT NOT NULL,
    detail_json TEXT
);
```

##### 后端流程

```text
parse CLI
  ↓
load HubConfig
  ↓
ensure data_dir exists (std::fs::create_dir_all)
  ↓
Database::new(data_dir.join("hub.db"))
  ↓
run migrations
  ↓
store Database in Axum state (Arc<Database>)
  ↓
start HTTP server
```

##### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。所有公开函数和关键逻辑路径必须有测试。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

测试用例：

```rust
#[test]
fn database_initialization() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();
    // 验证表已创建
    let tables: Vec<String> = db.conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert!(tables.contains(&"environments".to_string()));
    assert!(tables.contains(&"agents".to_string()));
    assert!(tables.contains(&"resources".to_string()));
    assert!(tables.contains(&"audit_log".to_string()));
}

#[test]
fn database_invalid_path_errors() {
    let result = Database::new(Path::new("/nonexistent/db/test.db"));
    assert!(result.is_err());
}
```

##### 提交边界

```text
feat: add SQLite database initialization with schema
```

提交内容：

- `crates/rex-hub/src/db.rs`
- `crates/rex-hub/src/main.rs`（worker 模式调用 db 初始化）
- `crates/rex-hub/Cargo.toml`（添加 rusqlite、tempfile 依赖）
- 根 `Cargo.toml`（添加 rusqlite、tempfile 依赖）

不得包含：

- 环境 CRUD API（子任务 1.4 实现）。
- 资源 CRUD API（子任务 1.5 实现）。
- 认证中间件（子任务 1.7 实现）。

#### 子任务 1.4：环境 CRUD API

##### 功能目标

实现环境（Environment）的完整 CRUD API：列表查询、创建、查询详情、更新、删除。环境是 REX Hub 的核心组织单元，资源和 Agent 都归属于某个环境。

##### 产品边界

- 环境 API 暂不包含认证（子任务 1.7 添加认证中间件后统一保护）。
- 环境名称在同一 Hub 内不重复。
- 删除环境时，若该环境下存在资源或 Agent，返回错误拒绝删除。
- 环境连接模式只有 `agent_proxy` 和 `direct` 两种。
- Agent token 只在创建 `agent_proxy` 模式环境时由后端生成，不接受前端传入。
- 本子任务不实现资源 CRUD（子任务 1.5）。

##### 文件结构

```text
crates/rex-hub/
└── src/
    ├── main.rs       修改：将 Database 注入 Axum state
    ├── routes.rs     修改：注册环境路由
    ├── db.rs         修改：添加环境 CRUD 方法
    └── env.rs        新增：环境路由 handler
```

根 `Cargo.toml` 添加 `uuid` 依赖（v4 生成 ID）。

##### 接口设计

```http
GET    /api/environments          # 列表查询
POST   /api/environments          # 创建环境
GET    /api/environments/:id      # 查询详情
PUT    /api/environments/:id      # 更新环境
DELETE /api/environments/:id      # 删除环境
```

请求体（创建）：

```json
{
  "name": "生产环境",
  "description": "生产服务器集群",
  "connection_mode": "agent_proxy"
}
```

请求体（更新）：

```json
{
  "name": "生产环境（已改名）",
  "description": "更新后的描述"
}
```

响应结构遵循统一响应格式：

```json
{
  "data": { ... }
}
```

错误响应：

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "环境名称不能为空"
  }
}
```

##### 数据模型

复用 `environments` 表（已在子任务 1.3 创建）：

```sql
CREATE TABLE IF NOT EXISTS environments (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    connection_mode TEXT NOT NULL,
    agent_token_hash TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

ID 生成格式：`env_{8位hex}`，例如 `env_a1b2c3d4`。

时间格式：ISO 8601 UTC，例如 `2026-06-17T12:00:00Z`。

##### 状态流转

```text
POST /api/environments
  ↓
验证 name 非空
  ↓
验证 connection_mode 是 agent_proxy 或 direct
  ↓
检查名称唯一性
  ↓
生成 ID 和时间戳
  ↓
INSERT INTO environments
  ↓
返回 201 + 环境数据
```

```text
DELETE /api/environments/:id
  ↓
查询环境是否存在
  ↓
检查是否有关联资源或 Agent
  ↓
有关联 → 返回 409 CONFLICT
无关联 → DELETE
  ↓
返回 204
```

##### 后端流程

将 `Database` 作为 Axum shared state：

```rust
// main.rs
use std::sync::Arc;

async fn run_worker_async(config: config::HubConfig) {
    // ...
    let database = db::Database::new(&db_path).expect("...");
    let state = Arc::new(database);
    let listener = tokio::net::TcpListener::bind(&config.listen).await.unwrap();
    axum::serve(listener, routes::app(state)).await.unwrap();
}
```

```rust
// routes.rs
use std::sync::Arc;

pub fn app(db: Arc<db::Database>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/environments", get(env::list_envs).post(env::create_env))
        .route(
            "/api/environments/:id",
            get(env::get_env).put(env::update_env).delete(env::delete_env),
        )
        .with_state(db)
}
```

##### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。所有公开函数和关键逻辑路径必须有测试。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

测试用例：

1. 创建环境成功
2. 创建环境缺少名称返回错误
3. 创建环境无效连接模式返回错误
4. 列表查询返回已创建的环境
5. 查询单个环境成功
6. 查询不存在的环境返回 404
7. 更新环境成功
8. 删除无关联的环境成功
9. 删除有关联资源的环境返回 409

##### 提交边界

```text
feat: add environment CRUD API
```

提交内容：

- `crates/rex-hub/src/env.rs`
- `crates/rex-hub/src/routes.rs`（注册路由）
- `crates/rex-hub/src/db.rs`（添加环境查询方法）
- `crates/rex-hub/src/main.rs`（注入 Database state）
- 根 `Cargo.toml`（添加 uuid 依赖）

不得包含：

- 资源 CRUD API（子任务 1.5 实现）。
- 登录 API（子任务 1.6 实现）。
- 认证中间件（子任务 1.7 实现）。

#### 子任务 1.5：资源 CRUD API

##### 功能目标

实现资源（Resource）的完整 CRUD API：列表查询（按环境过滤）、创建、查询详情、更新、删除。资源归属于某个环境，代表可连接的远程服务实例（SSH、MySQL、PostgreSQL 等）。

##### 产品边界

- 资源 API 暂不包含认证（子任务 1.7 添加认证中间件后统一保护）。
- 资源必须归属于一个已存在的环境。
- 资源名称在同一环境下不重复。
- 协议类型支持：ssh、sftp、mysql、postgresql、redis、docker、sqlite、s3。
- 连接模式支持：`agent_proxy` 和 `direct`。
- `config_json` 存储连接配置（JSON 字符串），本期不解析内容。
- 本子任务不实现登录 API（子任务 1.6）和认证中间件（子任务 1.7）。

##### 文件结构

```text
crates/rex-hub/
└── src/
    ├── main.rs       修改：注册资源路由
    ├── routes.rs     修改：注册资源路由
    ├── db.rs         修改：添加资源 CRUD 方法
    └── resource.rs   新增：资源路由 handler
```

##### 接口设计

```http
GET    /api/environments/:env_id/resources          # 按环境列表查询
POST   /api/environments/:env_id/resources          # 创建资源
GET    /api/environments/:env_id/resources/:id      # 查询详情
PUT    /api/environments/:env_id/resources/:id      # 更新资源
DELETE /api/environments/:env_id/resources/:id      # 删除资源
```

请求体（创建）：

```json
{
  "name": "Web 服务器",
  "protocol": "ssh",
  "connection_mode": "agent_proxy",
  "agent_id": null,
  "config_json": "{\"host\":\"192.0.2.1\",\"port\":22}"
}
```

响应格式遵循统一错误格式。

##### 数据模型

复用 `resources` 表（已在子任务 1.3 创建）。

ID 生成格式：`res_{8位hex}`，例如 `res_f5e6d7c8`。

##### 后端流程

```text
POST /api/environments/:env_id/resources
  ↓
验证环境存在
  ↓
验证必填字段
  ↓
验证 protocol 合法
  ↓
验证 connection_mode 合法
  ↓
检查资源名称在同一环境下唯一
  ↓
INSERT INTO resources
  ↓
返回 201 + 资源数据
```

##### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

测试用例：

1. 创建资源成功
2. 创建资源环境不存在返回 404
3. 创建资源缺少名称返回错误
4. 创建资源无效协议返回错误
5. 创建资源名称重复返回错误
6. 按环境列表查询返回已创建的资源
7. 查询单个资源成功
8. 更新资源成功
9. 删除资源成功

##### 提交边界

```text
feat: add resource CRUD API
```

提交内容：

- `crates/rex-hub/src/resource.rs`
- `crates/rex-hub/src/routes.rs`（注册资源路由）
- `crates/rex-hub/src/db.rs`（添加资源查询方法）
- `crates/rex-hub/src/main.rs`（添加 mod resource）

不得包含：

- 登录 API（子任务 1.6 实现）。
- 认证中间件（子任务 1.7 实现）。
- 审计日志写入（子任务 1.8 实现）。

#### 子任务 1.6：登录 API

##### 功能目标

实现单用户登录 API，支持密码登录并返回会话 token。为后续认证中间件（子任务 1.7）提供 token 验证基础。

##### 产品边界

- 单用户模型，固定用户名 `admin`，密码通过环境变量 `REX_PASSWORD` 或配置文件设置。
- 登录成功返回 JWT token，有效期 7 天。
- token 使用 `secret_key` 签名（HS256）。
- 登录失败返回 401，不暴露具体原因（用户名错误或密码错误统一返回"认证失败"）。
- 默认密码为 `admin`（首次部署时用户应修改）。
- 本子任务只实现登录 API，不实现认证中间件（子任务 1.7）。
- 不实现注册、登出、刷新 token 等能力。

##### 文件结构

```text
crates/rex-hub/
└── src/
    ├── main.rs       修改：添加 mod auth
    ├── routes.rs     修改：注册登录路由
    ├── config.rs     修改：添加 password 字段
    ├── db.rs         修改：添加 settings 表查询方法
    ├── auth.rs       新增：登录 handler 和 JWT 生成
    └── migrations.sql 修改：添加 settings 表
```

根 `Cargo.toml` 添加 `jsonwebtoken` 和 `base64` 依赖。

##### 接口设计

```http
POST /api/auth/login
```

请求体：

```json
{
  "password": "admin"
}
```

成功响应（200）：

```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "expiresAt": "2026-06-24T12:00:00Z"
}
```

失败响应（401）：

```json
{
  "error": {
    "code": "AUTH_INVALID",
    "message": "密码错误"
  }
}
```

##### 数据模型

新增 `settings` 表用于存储密码哈希：

```sql
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

存储内容：

```text
password_hash = argon2(password)
```

密码哈希使用 `argon2` 算道（推荐）或 `bcrypt`。

##### 状态流转

```text
POST /api/auth/login
  ↓
解析请求体获取 password
  ↓
查询 settings 表获取 password_hash
  ↓
未找到 password_hash → 使用默认密码 admin
  ↓
验证密码（argon2::verify_encoded）
  ↓
失败 → 返回 401 AUTH_INVALID
  ↓
生成 JWT token（HS256，有效期 7 天）
  ↓
返回 200 + token + expiresAt
```

##### 后端流程

```rust
// auth.rs
pub async fn login(
    State(db): State<Arc<Database>>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 1. 从 settings 表读取 password_hash
    // 2. 验证密码
    // 3. 生成 JWT token
    // 4. 返回 token 和过期时间
}
```

JWT payload：

```json
{
  "sub": "admin",
  "exp": 1750000000
}
```

##### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

测试用例：

1. 登录成功返回 token
2. 登录失败返回 401
3. 默认密码可用
4. 空密码返回 401
5. JWT token 包含正确 payload
6. JWT token 过期时间正确

##### 提交边界

```text
feat: add login API with JWT token
```

提交内容：

- `crates/rex-hub/src/auth.rs`
- `crates/rex-hub/src/routes.rs`（注册路由）
- `crates/rex-hub/src/db.rs`（添加 settings 查询方法）
- `crates/rex-hub/src/config.rs`（添加 password 字段）
- `crates/rex-hub/src/migrations.sql`（添加 settings 表）
- `crates/rex-hub/src/main.rs`（添加 mod auth）
- 根 `Cargo.toml`（添加 jsonwebtoken、base64、argon2 依赖）

不得包含：

- 认证中间件（子任务 1.7 实现）。
- 审计日志写入（子任务 1.8 实现）。
- 注册、登出、刷新 token。

#### 子任务 1.7：基础认证中间件

##### 功能目标

为所有管理 API 添加 JWT 认证中间件。未登录请求被拒绝（`/healthz` 和 `/api/auth/login` 除外）。登录成功后请求通过。

##### 产品边界

- `/healthz` 公开，不需要认证。
- `/api/auth/login` 公开，不需要认证。
- 其他所有 `/api/*` 端点需要携带有效的 Bearer token。
- 未携带 token 返回 `401 AUTH_REQUIRED`。
- token 无效或过期返回 `401 AUTH_INVALID`。
- token 从 `Authorization: Bearer <token>` 头部读取。
- 认证失败不返回具体原因（统一错误格式）。
- 本子任务不实现权限控制（单用户模型，所有已认证用户等同 admin）。

##### 文件结构

```text
crates/rex-hub/
└── src/
    ├── routes.rs     修改：添加认证中间件层
    ├── auth.rs       已有：verify_token 函数
```

无需新增文件，利用 Axum 的 `axum::middleware` 机制。

##### 接口设计

认证中间件签名：

```rust
pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>;
```

行为：
- 从 `Authorization` 头部提取 `Bearer <token>`。
- 使用 `secret_key` 验证 JWT token。
- 验证通过：继续请求处理。
- 验证失败：返回 `401`。

路由层级：

```text
公开路由（不经过中间件）：
  GET  /healthz
  POST /api/auth/login

受保护路由（经过中间件）：
  GET    /api/environments
  POST   /api/environments
  GET    /api/environments/:id
  PUT    /api/environments/:id
  DELETE /api/environments/:id
  GET    /api/environments/:env_id/resources
  POST   /api/environments/:env_id/resources
  GET    /api/environments/:env_id/resources/:id
  PUT    /api/environments/:env_id/resources/:id
  DELETE /api/environments/:env_id/resources/:id
```

##### 后端流程

```rust
// routes.rs
pub fn app(db: Arc<Database>) -> Router {
    let public_routes = Router::new()
        .route("/healthz", get(healthz))
        .route("/api/auth/login", post(auth::login));

    let protected_routes = Router::new()
        .route("/api/environments", get(env::list_envs).post(env::create_env))
        .route("/api/environments/:id", get(env::get_env).put(env::update_env).delete(env::delete_env))
        .route("/api/environments/:env_id/resources", get(resource::list_resources).post(resource::create_resource))
        .route("/api/environments/:env_id/resources/:id", get(resource::get_resource).put(resource::update_resource).delete(resource::delete_resource))
        .layer(axum::middleware::from_fn(auth::auth_middleware));

    public_routes.merge(protected_routes).with_state(db)
}
```

中间件实现：

```rust
// auth.rs
pub async fn auth_middleware(
    headers: axum::http::header::HeaderMap,
    State(db): State<Arc<Database>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&headers);
    match token {
        Some(token) => {
            let secret_key = "dev-secret-key";
            if verify_token(secret_key, &token) {
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
```

Axum middleware 需要 `FromRequestParts` trait，使用 `axum::middleware::from_fn_with_state`。

##### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

测试用例：

1. `/healthz` 不需要认证
2. `/api/auth/login` 不需要认证
3. 无 token 访问 `/api/environments` 返回 401
4. 有效 token 访问 `/api/environments` 返回 200
5. 无效 token 返回 401
6. 过期 token 返回 401
7. 缺少 Bearer 前缀返回 401

##### 提交边界

```text
feat: add auth middleware for protected API routes
```

提交内容：

- `crates/rex-hub/src/routes.rs`（路由层级化 + 中间件）
- `crates/rex-hub/src/auth.rs`（auth_middleware 函数）

不得包含：

- 审计日志写入（子任务 1.8 实现）。
- 权限控制。
- 角色管理。

#### 子任务 1.8：审计日志写入

##### 功能目标

实现审计日志写入能力，为环境 CRUD、资源 CRUD、登录操作自动记录审计日志。提供审计日志查询 API（列表查询，支持时间范围和类型过滤）。审计日志是安全合规的核心能力，记录所有管理操作的 who/what/when/result。

##### 产品边界

- 所有写操作（创建、更新、删除）自动写入审计日志。
- 登录成功和登录失败都写入审计日志。
- 审计日志用户固定为 `admin`（单用户模型）。
- 查询 API 支持时间范围过滤（`from` / `to`）。
- 查询 API 支持类型过滤（`type`）。
- 查询 API 支持分页（`page` / `pageSize`）。
- 审计日志不可修改、不可删除（只追加）。
- 本子任务不实现前端审计日志页面。
- 本子任务不实现审计日志导出。

##### 文件结构

```text
crates/rex-hub/
└── src/
    ├── main.rs       修改：添加 mod audit
    ├── routes.rs     修改：注册审计日志查询路由
    ├── db.rs         修改：添加审计日志写入和查询方法
    ├── audit.rs      新增：审计日志 handler 和写入函数
    ├── env.rs        修改：创建/更新/删除操作后写入审计日志
    ├── resource.rs   修改：创建/更新/删除操作后写入审计日志
    └── auth.rs       修改：登录成功/失败后写入审计日志
```

##### 接口设计

```http
GET /api/audit-log?from=2024-06-01T00:00:00Z&to=2024-06-02T00:00:00Z&type=login&page=1&pageSize=50
```

查询参数（全部可选）：

| 参数 | 类型 | 说明 |
|------|------|------|
| `from` | ISO 8601 | 开始时间（含） |
| `to` | ISO 8601 | 结束时间（含） |
| `type` | string | 事件类型过滤 |
| `page` | u32 | 页码，默认 1 |
| `pageSize` | u32 | 每页条数，默认 50，最大 100 |

成功响应（200）：

```json
{
  "items": [
    {
      "id": "log_a1b2c3d4",
      "time": "2026-06-17T12:00:00Z",
      "user": "admin",
      "environmentId": null,
      "resourceId": null,
      "agentId": null,
      "type": "login",
      "result": "success",
      "summary": "登录成功",
      "detail": null
    }
  ],
  "total": 128,
  "page": 1,
  "pageSize": 50
}
```

##### 数据模型

复用 `audit_log` 表（已在子任务 1.3 创建）：

```sql
CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    time TEXT NOT NULL,
    user TEXT NOT NULL,
    environment_id TEXT,
    resource_id TEXT,
    agent_id TEXT,
    type TEXT NOT NULL,
    result TEXT NOT NULL,
    summary TEXT NOT NULL,
    detail_json TEXT
);
```

ID 生成格式：`log_{8位hex}`，例如 `log_f5e6d7c8`。

事件类型枚举：

| type | 说明 | summary 示例 |
|------|------|-------------|
| `login` | 登录 | "登录成功" / "登录失败：密码错误" |
| `environment_create` | 创建环境 | "创建环境「生产环境」" |
| `environment_update` | 更新环境 | "更新环境「生产环境」" |
| `environment_delete` | 删除环境 | "删除环境「生产环境」" |
| `resource_create` | 创建资源 | "创建资源「Web 服务器」" |
| `resource_update` | 更新资源 | "更新资源「Web 服务器」" |
| `resource_delete` | 删除资源 | "删除资源「Web 服务器」" |

结果枚举：`success`、`failure`。

##### 写入函数设计

```rust
// audit.rs
pub struct AuditEntry {
    pub environment_id: Option<String>,
    pub resource_id: Option<String>,
    pub agent_id: Option<String>,
    pub audit_type: String,
    pub result: String,
    pub summary: String,
    pub detail: Option<String>,
}

pub fn write_audit(db: &Database, entry: AuditEntry) {
    // 生成 ID、时间戳，写入 audit_log 表
}
```

所有写操作在事务完成后调用 `write_audit`。审计日志写入失败不阻塞主操作（只记录 tracing warn）。

##### 状态流转

```text
管理操作（创建/更新/删除/登录）
  ↓
执行主操作
  ↓
主操作成功？
  ├─ yes → result = "success"
  └─ no  → result = "failure"
  ↓
构造 AuditEntry
  ↓
write_audit(db, entry)
  ↓
写入 audit_log 表
```

##### 查询流程

```text
GET /api/audit-log?from=...&to=...&type=...
  ↓
构建 WHERE 条件（动态拼接）
  ↓
SELECT COUNT(*) → total
  ↓
SELECT * ORDER BY time DESC LIMIT offset, pageSize
  ↓
返回 { items, total, page, pageSize }
```

##### 后端流程

路由注册：

```rust
// routes.rs - protected_routes 中添加
.route("/api/audit-log", get(audit::list_audit_logs))
```

各 handler 中调用审计写入：

```rust
// env.rs - create_env 中
audit::write_audit(&db, AuditEntry {
    environment_id: Some(id.clone()),
    resource_id: None,
    agent_id: None,
    audit_type: "environment_create".to_string(),
    result: "success".to_string(),
    summary: format!("创建环境「{}」", name),
    detail: None,
});
```

##### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

测试用例：

1. `write_audit` 写入后可查询到记录
2. 查询 API 返回正确的分页结构
3. `from` / `to` 时间过滤正确
4. `type` 过滤正确
5. 创建环境后审计日志自动写入
6. 删除环境后审计日志自动写入
7. 创建资源后审计日志自动写入
8. 删除资源后审计日志自动写入
9. 登录成功写入审计日志
10. 登录失败写入审计日志
11. 空查询返回空列表和 total=0
12. 分页参数正确工作

##### 提交边界

```text
feat: add audit log writing for management operations
```

提交内容：

- `crates/rex-hub/src/audit.rs`
- `crates/rex-hub/src/routes.rs`（注册查询路由）
- `crates/rex-hub/src/db.rs`（添加审计日志写入和查询方法）
- `crates/rex-hub/src/env.rs`（操作后写入审计日志）
- `crates/rex-hub/src/resource.rs`（操作后写入审计日志）
- `crates/rex-hub/src/auth.rs`（登录写入审计日志）
- `crates/rex-hub/src/main.rs`（添加 mod audit）

不得包含：

- 前端审计日志页面。
- 审计日志导出。
- 审计日志自动清理。
- 审计日志修改/删除能力。

### 27.4 里程碑 2：Agent 连接

目标：Agent 可以主动连接 Hub，并维持心跳。

子任务：

1. Agent 配置加载（子任务 2.1）
2. Agent 身份持久化（子任务 2.2）
3. Agent 注册 API（子任务 2.3）
4. Agent WebSocket 连接 + 心跳（子任务 2.4）
5. Agent 在线状态管理（子任务 2.5）

完成标准：

- Agent 启动后能连接 Hub。
- Hub 页面能看到 Agent 在线。
- Agent 重启后保持同一 Agent ID。
- token 错误时无法注册。
- 版本不匹配时返回需要更新或拒绝连接。

#### 子任务 2.1：Agent 配置加载

##### 功能目标

Agent worker 启动时加载配置文件 `agent.yaml`，支持环境变量覆盖。配置用于连接 Hub 服务器、身份认证和基本信息。

##### 产品边界

- 配置文件路径默认为当前目录 `agent.yaml`，可通过 `--config` 参数指定。
- 环境变量格式为 `REX_<FIELD>`，例如 `REX_SERVER`、`REX_TOKEN`。
- 环境变量优先级高于配置文件。
- 本子任务只实现配置加载，不实现连接 Hub。
- 配置字段：`server`（Hub 地址）、`token`（Agent token）、`name`（Agent 名称）、`data_dir`（数据目录）。

##### 文件结构

```text
crates/rex-agent/
└── src/
    ├── main.rs       修改：worker 模式加载配置
    └── config.rs     新增：AgentConfig 结构体和加载逻辑
```

根 `Cargo.toml` 已有 `serde` 和 `serde_yaml` 依赖（Hub 已添加）。

##### 接口设计

```rust
// crates/rex-agent/src/config.rs
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AgentConfig {
    pub server: String,
    pub token: String,
    pub name: String,
    pub data_dir: PathBuf,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server: "http://localhost:3000".to_string(),
            token: String::new(),
            name: "unnamed-agent".to_string(),
            data_dir: PathBuf::from("./data"),
        }
    }
}

impl AgentConfig {
    pub fn load(config_path: Option<&str>) -> anyhow::Result<Self> {
        // 1. 读取配置文件（如果存在）
        // 2. 应用环境变量覆盖
        // 3. 返回最终配置
    }
}
```

环境变量覆盖：

| 环境变量 | 配置字段 | 说明 |
|----------|----------|------|
| `REX_SERVER` | `server` | Hub 服务器地址 |
| `REX_TOKEN` | `token` | Agent token |
| `REX_NAME` | `name` | Agent 名称 |
| `REX_DATA_DIR` | `data_dir` | 数据目录 |

##### CLI 扩展

Agent 的 `Cli` 结构已在 `rex-common` 中定义，包含 `config` 字段。`AgentConfig::load(cli.config.as_deref())` 即可。

##### 后端流程

```text
parse CLI
  ↓
AgentConfig::load(cli.config.as_deref())
  ↓
返回 AgentConfig
  ↓
run_worker_async(config)
  ↓
log config (server, name, data_dir)
```

worker 模式下打印配置信息并退出（本子任务不连接 Hub）。

##### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

测试用例：

1. 默认配置值正确
2. 加载不存在的配置文件使用默认值
3. 环境变量覆盖 server
4. 环境变量覆盖 token
5. 环境变量覆盖 name
6. 环境变量覆盖 data_dir

##### 提交边界

```text
feat: add AgentConfig loading with env override
```

提交内容：

- `crates/rex-agent/src/config.rs`
- `crates/rex-agent/src/main.rs`（加载配置、打印信息）
- `crates/rex-hub/Cargo.toml`（添加 rex-agent workspace member，如未添加）

不得包含：

- Agent 连接 Hub（子任务 2.3 实现）。
- Agent 身份持久化（子任务 2.2 实现）。
- WebSocket 通信（子任务 2.4 实现）。

#### 子任务 2.3：Agent 注册 API

##### 功能目标

Agent 启动后向 Hub 发送注册请求，Hub 验证 token、创建/更新 agent 记录并返回注册结果。本子任务只实现 HTTP 注册端点，不实现 WebSocket 连接和心跳。

##### 产品边界

- Agent 注册端点 `POST /api/agents/register` 为公开端点，不需要 JWT 认证。
- Hub 通过 `environments.agent_token_hash` 验证 Agent 提供的 token。
- token 匹配到某个环境后，Agent 归属到该环境。
- 同一 Agent ID 重复注册时更新记录（幂等）。
- token 不存在或不匹配时返回 401。
- 本子任务不实现 WebSocket 连接（子任务 2.4）。
- 本子任务不实现心跳和在线状态管理（子任务 2.5）。
- 本子任务不实现 Agent 二进制下载端点。

##### 文件结构

```text
crates/rex-hub/src/
├── main.rs           修改：添加 mod agent
├── routes.rs         修改：注册 /api/agents/register 路由
├── agent.rs          新增：Agent 注册 handler 和验证逻辑
└── db.rs             修改：添加 Agent CRUD 方法

crates/rex-agent/src/
├── main.rs           修改：worker 模式调用注册 API
└── client.rs         新增：Agent → Hub HTTP 客户端
```

##### 接口设计

**请求：**

```text
POST /api/agents/register
Content-Type: application/json

{
    "id": "agt_a1b2c3d4",
    "token": "env注册令牌原文",
    "name": "prod-server",
    "version": "0.1.0",
    "os": "linux",
    "arch": "amd64",
    "hostname": "prod-server",
    "os_version": "Ubuntu 22.04"
}
```

**成功响应：**

```text
HTTP/1.1 200 OK

{
    "agent_id": "agt_a1b2c3d4",
    "environment_id": "env_12345678",
    "status": "online"
}
```

**失败响应：**

```text
HTTP/1.1 401 Unauthorized

{
    "error": "invalid_token"
}
```

##### 数据模型

Agent 注册涉及的数据库操作复用已有的 `agents` 表和 `environments` 表。

db.rs 新增方法：

```rust
// 查询 agent_token_hash 匹配的环境
pub fn find_environment_by_token_hash(&self, token_hash: &str) -> Option<EnvironmentRow>

// 插入或更新 agent（upsert）
pub fn upsert_agent(&self, id: &str, environment_id: &str, name: &str,
    token_hash: &str, version: &str, os: &str, arch: &str,
    hostname: Option<&str>, os_version: Option<&str>, now: &str)
```

##### 后端流程

```text
Agent 启动
  ↓
加载配置（token）+ 加载身份（id）
  ↓
POST /api/agents/register { id, token, name, version, os, arch, ... }
  ↓
Hub 接收请求
  ↓
SHA256(token) → token_hash
  ↓
查询 environments 表，找到 agent_token_hash == token_hash 的环境
  ↓
未找到 → 返回 401 { error: "invalid_token" }
  ↓
找到 → upsert_agent（INSERT OR REPLACE）
  ↓
返回 200 { agent_id, environment_id, status: "online" }
```

##### Agent 端 client 设计

```rust
// crates/rex-agent/src/client.rs

pub struct HubClient {
    server: String,
    token: String,
}

impl HubClient {
    pub fn new(server: &str, token: &str) -> Self { ... }

    pub async fn register(&self, id: &str, name: &str, version: &str,
        os: &str, arch: &str, hostname: Option<&str>, os_version: Option<&str>)
        -> anyhow::Result<RegisterResponse> { ... }
}

pub struct RegisterResponse {
    pub agent_id: String,
    pub environment_id: String,
    pub status: String,
}
```

Agent 端使用 `tokio::reqwest` 发送 HTTP 请求。

##### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

Hub 端测试用例：

1. token 匹配环境 → 返回 200 + agent_id + environment_id
2. token 不匹配任何环境 → 返回 401
3. 同一 agent_id 重复注册 → 更新记录（幂等）
4. 空 token → 返回 401
5. db.rs find_environment_by_token_hash 找到/找不到
6. db.rs upsert_agent 插入/更新

Agent 端测试用例：

7. HubClient::new 构造正确
8. RegisterResponse 反序列化

##### 提交边界

```text
feat: add agent registration API
```

提交内容：

- `crates/rex-hub/src/agent.rs`（新增）
- `crates/rex-hub/src/db.rs`（新增 Agent CRUD 方法）
- `crates/rex-hub/src/routes.rs`（注册路由）
- `crates/rex-hub/src/main.rs`（添加 mod agent）
- `crates/rex-agent/src/client.rs`（新增）
- `crates/rex-agent/src/main.rs`（调用注册 API）
- `crates/rex-agent/Cargo.toml`（添加 reqwest 依赖）
- `Cargo.toml`（添加 reqwest workspace 依赖）

不得包含：

- WebSocket 连接和心跳（子任务 2.4 实现）。
- Agent 在线状态管理（子任务 2.5 实现）。
- Agent 二进制下载端点。

#### 子任务 2.4：Agent WebSocket 连接 + 心跳

##### 功能目标

Agent 注册成功后，建立 WebSocket 长连接到 Hub。双方通过 JSON 消息通信，Agent 定期发送心跳，Hub 通过心跳更新 Agent 在线状态和元信息。本子任务不实现资源通道（`resource.connect`）。

##### 产品边界

- Agent 启动后先注册（子任务 2.3），注册成功后建立 WebSocket 连接到 `WS /ws/agent`。
- WebSocket 连接建立时 Agent 发送身份消息（agent_id + token）。
- Hub 验证身份，拒绝非法连接。
- Agent 每 30 秒发送心跳消息（版本、sha256、运行时间、metrics）。
- Hub 收到心跳后更新 `agents` 表的 `last_seen_at` 和 `status`。
- Agent 断开后 Hub 将状态更新为 `offline`。
- 本子任务不实现资源通道协议（`resource.connect` / `resource.connected`）。
- 本子任务不实现终端/文件传输等业务通道。

##### 文件结构

```text
crates/rex-common/src/
└── protocol.rs      新增：WebSocket 消息类型定义

crates/rex-hub/src/
├── main.rs          修改：添加 mod ws
├── routes.rs        修改：注册 /ws/agent 路由
├── ws.rs            新增：WebSocket handler（Agent 连接、身份验证、心跳处理）
└── db.rs            修改：添加 update_agent_heartbeat / update_agent_status 方法

crates/rex-agent/src/
├── main.rs          修改：注册后建立 WebSocket 连接
└── ws.rs            新增：Agent WebSocket 客户端（连接、心跳、重连）
```

##### 接口设计

**WebSocket 消息格式（JSON）：**

所有消息共享统一信封：

```rust
// crates/rex-common/src/protocol.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage {
    pub msg_type: String,  // "auth" | "heartbeat" | "heartbeat_ack" | "disconnect"
    pub payload: serde_json::Value,
}
```

**Agent → Hub 消息：**

1. 身份认证（连接后第一条消息）：

```json
{
    "msg_type": "auth",
    "payload": {
        "agent_id": "agt_a1b2c3d4",
        "token": "env注册令牌原文"
    }
}
```

2. 心跳（每 30 秒）：

```json
{
    "msg_type": "heartbeat",
    "payload": {
        "version": "0.1.0",
        "sha256": "",
        "os": "linux",
        "arch": "amd64",
        "hostname": "prod-server",
        "os_version": "Ubuntu 22.04",
        "uptime_secs": 3600,
        "metrics": {
            "latency_ms": 12,
            "rx_bytes": 1024,
            "tx_bytes": 2048
        }
    }
}
```

**Hub → Agent 消息：**

1. 心跳确认：

```json
{
    "msg_type": "heartbeat_ack",
    "payload": {}
}
```

2. 断开连接（token 重置时）：

```json
{
    "msg_type": "disconnect",
    "payload": {
        "reason": "token_revoked"
    }
}
```

##### 数据模型

db.rs 新增方法：

```rust
pub fn update_agent_heartbeat(&self, agent_id: &str, version: &str, sha256: &str,
    os: &str, arch: &str, hostname: Option<&str>, os_version: Option<&str>,
    last_seen_at: &str)

pub fn update_agent_status(&self, agent_id: &str, status: &str)
```

##### 后端流程（Hub 端）

```text
Agent WebSocket 连接到 /ws/agent
  ↓
Hub 接受 WebSocket 升级
  ↓
等待 Agent 发送 auth 消息
  ↓
验证 agent_id + token（SHA256 后查 environments.agent_token_hash）
  ↓
验证失败 → 发送 disconnect → 关闭连接
  ↓
验证成功 → 记录 Agent 连接映射 → 发送 auth_ack
  ↓
进入消息循环
  ↓
收到 heartbeat → 更新 agents 表 last_seen_at/status → 发送 heartbeat_ack
  ↓
收到其他消息 → 未来子任务处理
  ↓
连接断开 → 更新 agents 表 status = "offline"
```

##### 后端流程（Agent 端）

```text
注册成功（子任务 2.3）
  ↓
建立 WebSocket 连接到 Hub /ws/agent
  ↓
发送 auth 消息（agent_id + token）
  ↓
等待 auth_ack
  ↓
启动心跳定时器（每 30 秒发送 heartbeat）
  ↓
进入消息循环
  ↓
收到 heartbeat_ack → 记录延迟
  ↓
收到 disconnect → 退出
  ↓
连接断开 → 等待后重连（指数退避，最大 60 秒）
```

##### 关键设计

**连接管理（Hub）：**

- 使用 `tokio::sync::RwLock<HashMap<String, AgentConnection>>` 存储在线 Agent 连接。
- `AgentConnection` 包含 WebSocket sender 和元信息。
- 连接断开时自动从 map 中移除。

**重连（Agent）：**

- 断开后指数退避重连：1s → 2s → 4s → 8s → 16s → 32s → 60s（上限）。
- 重连成功后重置退避。
- 重连需要重新发送 auth 消息。

**心跳间隔：**

- Agent 端：30 秒。
- Hub 端：超过 90 秒未收到心跳标记为 offline（3 倍心跳间隔）。

##### 测试 / QA 标准

测试覆盖率要求：**100% 代码覆盖率**。

必须通过：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

测试用例：

Hub 端（ws.rs）：

1. 正常 auth → 连接建立
2. 错误 token → disconnect
3. 空 token → disconnect
4. heartbeat → 更新 db + heartbeat_ack
5. 连接断开 → status 变为 offline

Agent 端（ws.rs）：

6. WsMessage 序列化/反序列化
7. heartbeat 消息构造正确

protocol.rs：

8. WsMessage 序列化/反序列化（auth、heartbeat、heartbeat_ack、disconnect）

db.rs：

9. update_agent_heartbeat 更新字段
10. update_agent_status 更新状态

##### 提交边界

```text
feat: add agent WebSocket connection and heartbeat
```

提交内容：

- `crates/rex-common/src/protocol.rs`（新增）
- `crates/rex-hub/src/ws.rs`（新增）
- `crates/rex-hub/src/routes.rs`（注册 WS 路由）
- `crates/rex-hub/src/main.rs`（添加 mod ws）
- `crates/rex-hub/src/db.rs`（新增 heartbeat/status 方法）
- `crates/rex-agent/src/ws.rs`（新增）
- `crates/rex-agent/src/main.rs`（注册后建 WebSocket）
- `crates/rex-common/Cargo.toml`（添加 serde_json 依赖）
- `Cargo.toml`（添加 tungstenite/tokio-tungstenite 依赖）

不得包含：

- 资源通道协议（`resource.connect`）。
- 终端/文件传输通道。
- Agent 在线状态 API（子任务 2.5 实现）。

#### 子任务 2.5：Agent 在线状态管理

##### 功能目标

提供 API 查询 Agent 列表及在线状态，前端可在环境卡片和 Agent 页面展示 Agent 状态信息。

##### 产品边界

- 每个环境下的 Agent 列表，含名称、版本、平台、在线/离线状态。
- 在线判断依据：WebSocket 连接存在且 `last_seen_at` 在 90 秒内。
- Agent 列表 API 无需鉴权（第一阶段单用户），后续里程碑添加鉴权。
- 本子任务只实现查询 API，不实现前端页面（前端在里程碑 4+ 实现）。
- 不实现 Agent 详情/编辑/删除（如需后续补充）。

##### 文件结构

```text
crates/rex-hub/src/
├── agent.rs     修改：添加 list_agents handler
├── db.rs        修改：添加 list_agents_by_environment 方法
└── routes.rs    修改：添加 GET /api/environments/:env_id/agents 路由
```

##### 接口设计

```
GET /api/environments/:env_id/agents
```

Response 200:

```json
{
  "agents": [
    {
      "id": "agt_xxx",
      "environment_id": "env_xxx",
      "name": "prod-agent",
      "version": "0.1.0",
      "os": "linux",
      "arch": "amd64",
      "hostname": "prod-server",
      "os_version": "Ubuntu 22.04",
      "status": "online",
      "last_seen_at": "2025-01-01T12:00:00Z"
    }
  ]
}
```

##### 数据模型

复用已有 `AgentRow`，无需新建表或字段。

##### 在线判定逻辑

```rust
fn is_online(last_seen_at: &str) -> bool {
    // 解析 RFC3339 时间，判断距今是否 < 90 秒
    chrono::DateTime::parse_from_rfc3339(last_seen_at)
        .map(|t| chrono::Utc::now().signed_duration_since(t).num_seconds() < 90)
        .unwrap_or(false)
}
```

##### 测试 / QA 标准

- `list_agents_empty` — 环境无 Agent 返回空列表
- `list_agents_returns_agents` — 返回 Agent 列表含在线状态
- `list_agents_nonexistent_env_404` — 不存在的环境返回 404
- `is_online_recent` — 90 秒内标记在线
- `is_online_stale` — 超过 90 秒标记离线
- `is_online_unparseable` — 无法解析时间标记离线

##### 提交边界

单个 commit：`feat: add agent list API with online status`

不得包含：

- 前端 Agent 页面。
- Agent 详情/编辑/删除 API。
- Agent 统计/历史数据。

### 27.5 里程碑 3：SSH 终端 MVP

目标：实现第一个核心操作场景。

子任务：

1. SSH 客户端 crate（子任务 3.1）
2. SSH 密码登录 + key 登录（子任务 3.2）
3. Terminal session 模型（子任务 3.3）
4. WebSocket 终端数据通道（子任务 3.4）
5. Agent SSH 代理通道（子任务 3.5）
6. 前端 terminal 页面 + xterm.js（子任务 3.6）
7. 前端移动端方向键 + bash 历史（子任务 3.7）
8. 后端审计 SSH 连接/断开（子任务 3.8）

完成标准：

- 用户可以在浏览器连接 SSH。
- 输入命令有响应。
- 支持复制/粘贴基础能力。
- 移动端方向键可用。
- 断开连接有确认。

#### 子任务 3.1：SSH 客户端 crate

##### 功能目标

创建 `rex-ssh` crate，封装 SSH 连接、认证、PTY 分配和数据读写。为后续 terminal session 和 WebSocket 数据通道提供底层能力。

##### 产品边界

- 使用 `russh` 库（纯 Rust、tokio 异步、无 C 依赖）。
- 支持密码认证和公钥认证。
- 支持 PTY 分配（`request_pty`）。
- 支持 shell 执行（`request_shell`）。
- 支持窗口大小调整（`window_change`）。
- 本子任务不实现 WebSocket 通道（子任务 3.4）。
- 本子任务不实现前端（子任务 3.6）。
- 不实现跳板机/代理跳转。

##### 文件结构

```text
crates/rex-ssh/
├── Cargo.toml       新增：russh、tokio、anyhow 依赖
└── src/
    ├── lib.rs       新增：导出模块
    ├── client.rs    新增：SshClient 封装
    └── auth.rs      新增：认证方式定义
```

根 `Cargo.toml` 添加 `russh`、`russh-keys` 依赖。

##### 接口设计

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

##### 测试 / QA 标准

- `auth_method_password` — Password 变体构造正确
- `auth_method_key` — Key 变体构造正确
- `ssh_event_struct` — SshEvent 字段可访问
- 连接测试标记为 `#[ignore]`（需要真实 SSH 服务器，不在 CI 运行）

##### 提交边界

单个 commit：`feat: add rex-ssh client crate with PTY support`

不得包含：

- WebSocket 终端通道。
- Agent SSH 代理通道。
- 前端 terminal 页面。
- Terminal session 模型。

#### 子任务 3.2：SSH 资源配置验证与凭据加密

##### 功能目标

为 SSH 资源添加配置验证和敏感凭据加密。前端提交的 SSH 资源配置（密码、私钥密码）在存入数据库前加密，读取时解密。API 层对 SSH 资源的 `config_json` 进行结构化验证。

##### 产品边界

- 实现 AES-256-GCM 加密/解密工具函数，密钥从 `REX_SECRET_KEY` 派生。
- 实现 SSH 资源配置 DTO 和验证逻辑。
- 添加「测试连接」API 端点，验证 SSH 凭据可达性。
- 加密仅影响 SSH 资源的敏感字段（密码、私钥密码），不涉及其他协议。
- 本子任务不实现 WebSocket 终端通道（子任务 3.4）。
- 本子任务不实现 terminal session 模型（子任务 3.3）。
- 本子任务不实现后端审计（子任务 3.8）。
- 不实现 known_hosts 校验（沿用子任务 3.1 的 IgnoreHostKey）。
- 不实现私钥文件上传存储（密钥路径指向 Hub 文件系统上的路径）。

##### 文件结构

```text
crates/rex-ssh/
├── Cargo.toml           修改：添加 aes-gcm、sha2、hex 依赖
└── src/
    ├── lib.rs           修改：导出 crypto 模块
    ├── auth.rs          不变
    ├── client.rs        不变
    └── crypto.rs        新增：加密/解密工具函数

crates/rex-hub/
├── Cargo.toml           修改：添加 rex-ssh 依赖
└── src/
    ├── resource.rs      修改：添加 SSH 配置验证逻辑
    ├── ssh_config.rs    新增：SSH 配置 DTO 和验证
    └── routes.rs        修改：注册测试连接路由
```

##### 数据模型

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
    "passwordEncrypted": "base64(nonce || ciphertext)"
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

解密时从 base64 拆分 nonce（前 12 字节）和密文+tag（剩余部分），用同一派生密钥解密。

##### 接口设计

```rust
// crates/rex-ssh/src/crypto.rs

/// 从密钥字符串派生 AES-256 密钥
pub fn derive_key(secret_key: &str) -> [u8; 32];

/// AES-256-GCM 加密，返回 base64 编码的 nonce + ciphertext + tag
pub fn encrypt(plaintext: &str, secret_key: &str) -> String;

/// AES-256-GCM 解密，输入 base64 编码的 nonce + ciphertext + tag
pub fn decrypt(ciphertext: &str, secret_key: &str) -> anyhow::Result<String>;
```

```rust
// crates/rex-hub/src/ssh_config.rs

/// SSH 认证配置（来自前端）
#[derive(Debug, Deserialize)]
pub struct SshAuthConfig {
    #[serde(rename = "type")]
    pub auth_type: String,         // "password" | "key"
    pub password: Option<String>,  // 密码模式
    pub password_encrypted: Option<String>,  // 加密后密码
    pub private_key_path: Option<String>,    // 密钥模式
    pub passphrase: Option<String>,          // 密钥密码
    pub passphrase_encrypted: Option<String>, // 加密后密钥密码
}

/// SSH 终端配置
#[derive(Debug, Deserialize)]
pub struct SshTerminalConfig {
    pub encoding: Option<String>,           // 默认 "utf-8"
    pub keep_alive_seconds: Option<u32>,    // 默认 60
}

/// SSH 资源完整配置
#[derive(Debug, Deserialize)]
pub struct SshResourceConfig {
    pub host: String,
    pub port: Option<u16>,           // 默认 22
    pub username: String,
    pub auth: SshAuthConfig,
    pub terminal: Option<SshTerminalConfig>,
}

impl SshResourceConfig {
    /// 从 JSON 字符串解析并验证 SSH 配置
    pub fn from_json(json: &str) -> anyhow::Result<Self>;

    /// 将敏感字段加密，返回加密后的 JSON 字符串
    pub fn encrypt_sensitive(&self, secret_key: &str) -> anyhow::Result<String>;

    /// 从存库的 JSON 解析，解密敏感字段
    pub fn from_encrypted_json(json: &str, secret_key: &str) -> anyhow::Result<Self>;

    /// 转换为 rex_ssh::AuthMethod
    pub fn to_auth_method(&self, secret_key: &str) -> anyhow::Result<rex_ssh::auth::AuthMethod>;
}
```

```rust
// crates/rex-hub/src/resource.rs 或 routes.rs

/// POST /api/environments/:env_id/resources/:id/test-connection
/// 测试 SSH 连接可达性和认证
pub async fn test_ssh_connection(/* ... */) -> Result<Json<TestConnectionResponse>, (StatusCode, Json<ErrorResponse>)>;
```

```rust
// 测试连接响应
#[derive(Debug, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub latency_ms: u64,
    pub message: String,
}
```

##### 后端流程

**资源创建/更新时加密流程：**

```text
前端提交 config_json（含明文密码/密钥密码）
  ↓
解析为 SshResourceConfig
  ↓
验证字段（host 非空、port 范围、auth.type 合法）
  ↓
encrypt_sensitive(secret_key) → 加密敏感字段
  ↓
存入数据库 config_json
```

**SSH 资源读取时解密流程：**

```text
从数据库读取 config_json
  ↓
SshResourceConfig::from_encrypted_json(json, secret_key)
  ↓
解密 passwordEncrypted / passphraseEncrypted
  ↓
返回给前端（仅 API 层解密，前端不接触密文）
```

**测试连接流程：**

```text
接收 resource_id
  ↓
从数据库读取资源 config_json
  ↓
SshResourceConfig::from_encrypted_json(json, secret_key)
  ↓
to_auth_method(secret_key) → AuthMethod
  ↓
SshClient::connect(host, port, username, auth) + disconnect
  ↓
返回延迟和成功/失败状态
```

##### 测试 / QA 标准

**rex-ssh/crypto.rs 单元测试：**

- `derive_key_deterministic` — 相同输入产生相同密钥
- `derive_key_different_inputs` — 不同输入产生不同密钥
- `encrypt_decrypt_roundtrip` — 加密后解密还原原文
- `decrypt_invalid_input` — 无效输入返回错误
- `decrypt_wrong_key` — 用错误密钥解密返回错误
- `encrypt_empty_string` — 空字符串可正常加密解密

**rex-hub/ssh_config.rs 单元测试：**

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
- `to_auth_method_key_with_passphrase` — 带密码的密钥认证转换正确

**资源 API 集成测试（resource.rs 扩展）：**

- `create_ssh_resource_validates_config` — 无效 SSH 配置返回 400
- `create_ssh_resource_encrypts_password` — 存库后密码已加密

##### 提交边界

单个 commit：`feat: add SSH config validation and credential encryption`

不得包含：

- WebSocket 终端通道。
- Terminal session 模型。
- Agent SSH 代理通道。
- 前端 terminal 页面。
- 后端审计 SSH 连接/断开。

#### 子任务 3.3：Terminal session 模型

##### 功能目标

实现终端会话生命周期管理。每个 SSH 连接对应一个 TerminalSession，管理连接状态、PTY 尺寸、超时检测和资源清理。Session 存储在内存中（Arc<Mutex<HashMap>>），不持久化到数据库。

##### 产品边界

- TerminalSession 封装 SshClient，管理 connect → ready → active → closed 状态机。
- Session 超时检测：空闲超时（默认 15 分钟）自动断开。
- Session 管理器（SessionManager）提供创建、获取、关闭、清理能力。
- 本子任务不实现 WebSocket 数据通道（子任务 3.4）。
- 本子任务不实现前端（子任务 3.6）。
- 本子任务不实现 Agent 代理通道（子任务 3.5）。
- 不实现会话持久化（进程重启后会话丢失，用户需重新连接）。

##### 文件结构

```text
crates/rex-hub/
└── src/
    └── terminal.rs    新增：TerminalSession + SessionManager
```

##### 数据模型

```rust
// crates/rex-hub/src/terminal.rs

/// 终端会话状态
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// 正在连接
    Connecting,
    /// 已连接，等待 PTY/shell
    Connected,
    /// PTY 已分配，shell 已打开，可收发数据
    Active,
    /// 已断开
    Closed,
}

/// 终端会话
pub struct TerminalSession {
    pub id: String,                    // sess_xxxxxxxx
    pub resource_id: String,           // 关联的资源 ID
    pub state: SessionState,
    pub cols: u32,
    pub rows: u32,
    pub created_at: Instant,
    pub last_active_at: Instant,
    client: Option<SshClient>,         // 拥有 SshClient 的所有权
}

/// 会话管理器
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, TerminalSession>>>,
    max_idle_secs: u64,                // 默认 900 (15 分钟)
}
```

##### 接口设计

```rust
impl TerminalSession {
    /// 创建新会话（不立即连接）
    pub fn new(resource_id: &str, cols: u32, rows: u32) -> Self;

    /// 完成连接，设置 SshClient
    pub fn set_client(&mut self, client: SshClient) -> anyhow::Result<()>;

    /// 请求 PTY 和 shell
    pub async fn init_shell(&mut self) -> anyhow::Result<()>;

    /// 发送数据
    pub async fn send_data(&mut self, data: &[u8]) -> anyhow::Result<()>;

    /// 接收数据
    pub async fn recv(&mut self) -> anyhow::Result<SshEvent>;

    /// 窗口大小调整
    pub async fn resize(&mut self, cols: u32, rows: u32) -> anyhow::Result<()>;

    /// 关闭会话
    pub async fn close(&mut self) -> anyhow::Result<()>;

    /// 更新活跃时间
    pub fn touch(&mut self);

    /// 是否空闲超时
    pub fn is_idle_timeout(&self, max_idle_secs: u64) -> bool;
}

impl SessionManager {
    pub fn new(max_idle_secs: u64) -> Self;

    /// 创建并注册新会话
    pub fn create_session(&self, resource_id: &str, cols: u32, rows: u32) -> String;

    /// 获取会话引用
    pub fn get(&self, session_id: &str) -> Option<TerminalSession>;

    /// 移除并关闭会话
    pub async fn remove_session(&self, session_id: &str) -> anyhow::Result<()>;

    /// 清理所有空闲超时的会话
    pub async fn cleanup_idle(&self);

    /// 当前会话数量
    pub fn count(&self) -> usize;
}
```

##### 后端流程

```text
前端请求创建终端会话
  ↓
POST /api/ssh/sessions { resource_id, cols, rows }
  ↓
SessionManager::create_session()
  ↓
返回 session_id
  ↓
（子任务 3.4：前端通过 WebSocket 发起连接、PTY、shell）
```

##### 测试 / QA 标准

**单元测试：**

- `session_new_state_is_connecting` — 新建会话状态为 Connecting
- `session_set_client_transitions_to_connected` — 设置 client 后状态为 Connected
- `session_init_shell_transitions_to_active` — init_shell 后状态为 Active（标记 ignore，需要真实 SSH）
- `session_send_data_without_client_fails` — 无 client 时 send_data 返回错误
- `session_touch_updates_last_active` — touch 更新 last_active_at
- `session_is_idle_timeout` — 空闲超时检测正确
- `session_is_not_idle_when_active` — 活跃会话不超时
- `manager_create_and_get` — 创建并获取会话
- `manager_remove_session` — 移除会话
- `manager_cleanup_idle` — 清理空闲会话
- `manager_count` — 会话计数正确
- `manager_get_nonexistent_returns_none` — 获取不存在的会话返回 None

##### 提交边界

单个 commit：`feat: add terminal session model`

不得包含：

- WebSocket 终端数据通道。
- Agent SSH 代理通道。
- 前端 terminal 页面。
- 后端审计 SSH 连接/断开。

#### 子任务 3.4：WebSocket 终端数据通道

##### 功能目标

实现前端通过 WebSocket 与后端 TerminalSession 之间的实时数据通道。前端通过 WebSocket 发送终端输入、接收终端输出、发送窗口大小调整命令。后端在收到 WebSocket 连接后，自动完成 SSH 连接、PTY 分配、shell 初始化。

##### 产品边界

- WebSocket 端点：`/ws/terminal/:session_id`
- 消息协议：JSON 格式，统一 `type` + `payload` 结构
- 支持的消息类型：`terminal.input`、`terminal.resize`、`terminal.output`、`terminal.closed`、`terminal.error`
- 后端在 WebSocket 连接时自动创建 SSH 连接、PTY、shell（如果 session 是新创建的）
- 前端通过 REST API 创建 session，获得 session_id，然后通过 WebSocket 连接
- 本子任务不实现 Agent 代理通道（子任务 3.5）。
- 本子任务不实现前端（子任务 3.6）。
- 本子任务不实现后端审计（子任务 3.8）。

##### 文件结构

```text
crates/rex-hub/src/
├── ws_terminal.rs    新增：WebSocket 终端数据通道处理
├── terminal.rs       不变
├── routes.rs         修改：注册 WebSocket 路由和 REST API
```

##### 消息协议

```json
// 前端 → 后端
{ "type": "terminal.input", "payload": { "data": "base64编码的输入" } }
{ "type": "terminal.resize", "payload": { "cols": 120, "rows": 40 } }

// 后端 → 前端
{ "type": "terminal.output", "payload": { "data": "base64编码的输出" } }
{ "type": "terminal.closed", "payload": { "exit_status": 0 } }
{ "type": "terminal.error", "payload": { "message": "错误信息" } }
```

##### 接口设计

```rust
// crates/rex-hub/src/ws_terminal.rs

/// WebSocket 终端消息
#[derive(Debug, Deserialize)]
pub struct TerminalMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: serde_json::Value,
}

/// WebSocket 终端处理器
pub struct TerminalWsHandler {
    session_manager: Arc<SessionManager>,
    db: Arc<Database>,
}

impl TerminalWsHandler {
    pub fn new(session_manager: Arc<SessionManager>, db: Arc<Database>) -> Self;

    /// 处理 WebSocket 连接
    pub async fn handle_ws(
        &self,
        session_id: String,
        ws: WebSocket,
    ) -> anyhow::Result<()>;
}
```

##### 后端流程

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
TerminalSession::set_client()
  ↓
TerminalSession::init_shell()
  ↓
开始循环：
  前端发消息 → 后端处理（input/resize）
  后端 recv → 前端输出（output/closed/error）
```

##### REST API

```text
POST /api/ssh/sessions
Request: { resource_id, cols, rows }
Response: { session_id }

DELETE /api/ssh/sessions/:session_id
Response: 204 No Content
```

##### 测试 / QA 标准

**单元测试：**

- `terminal_message_deserialize` — 消息反序列化正确
- `terminal_output_message_serialize` — 输出消息序列化正确
- `handler_new` — 处理器创建成功

**集成测试（标记 ignore，需要真实 SSH）：**

- `ws_terminal_connect_and_send` — WebSocket 连接并发送数据
- `ws_terminal_receive_output` — 接收终端输出
- `ws_terminal_resize` — 窗口大小调整

##### 提交边界

单个 commit：`feat: add WebSocket terminal data channel`

不得包含：

- Agent SSH 代理通道。
- 前端 terminal 页面。
- 后端审计 SSH 连接/断开。

### 27.6 里程碑 4：文件管理与跨连接传输 MVP

#### 子任务 4.1：FileConnector trait

##### 功能目标

定义文件传输 MVP 的统一文件访问接口，供后续 SSH/SFTP connector、本地文件 connector 和传输任务模型复用。该 trait 只声明文件能力边界，不实现真实传输。

##### 产品边界

- 只实现 `FileConnector` trait、文件路径模型、文件元数据模型和错误结果类型。
- 本子任务不实现 SSH/SFTP connector（子任务 4.2）。
- 本子任务不实现本地文件 connector（子任务 4.3）。
- 本子任务不实现 `/api/transfers`、`/ws/transfers`、临时文件、SHA256、冲突策略或审计日志。
- 不引入多用户、RBAC、企业协作或外部云存储概念。
- 文件传输数据不经过浏览器这一原则由后续 connector / transfer 层实现，本 trait 只保证接口可表达“后端直接读写文件”。

##### 文件结构

```text
crates/
└── rex-transfer/
    ├── Cargo.toml
    └── src/
        └── lib.rs
```

根 workspace：

```text
Cargo.toml    修改：加入 crates/rex-transfer，并声明共享依赖
```

##### 接口设计

```rust
// crates/rex-transfer/src/lib.rs

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub file_type: FileType,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRead {
    pub entry: FileEntry,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileWrite {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

#[async_trait::async_trait]
pub trait FileConnector: Send + Sync {
    fn connector_name(&self) -> &'static str;

    async fn list(&self, path: &Path) -> Result<Vec<FileEntry>>;

    async fn metadata(&self, path: &Path) -> Result<FileEntry>;

    async fn read(&self, path: &Path) -> Result<FileRead>;

    async fn write(&self, path: &Path, bytes: &[u8]) -> Result<()>;
}
```

说明：

- `connector_name()` 用于后续传输任务展示来源/目标类型。
- `list()` 用于文件列表和“发送到…”目标选择。
- `metadata()` 用于文件大小、类型校验和冲突策略。
- `read()` / `write()` 是后端直接文件读写抽象，不经过浏览器。
- 使用 `PathBuf` 保存路径，后续 connector 负责把资源路径映射到真实远端/本地路径。

##### 数据模型

| 类型 | 字段 | 说明 |
|------|------|------|
| `FileType` | `File` / `Directory` | 文件或目录 |
| `FileEntry` | `name`, `path`, `file_type`, `size` | 文件列表项和元数据 |
| `FileRead` | `entry`, `bytes` | 读取结果，包含元数据和内容 |
| `FileWrite` | `path`, `bytes` | 写入请求，后续传输任务复用 |

##### 状态流转

```text
定义 FileConnector trait
  ↓
定义 FileEntry/FileType/FileRead/FileWrite
  ↓
后续 connector 实现 trait
  ↓
后续 TransferTask 调用 trait 完成读取/写入
```

##### 前端交互

本子任务不实现前端。前端仍使用现有原型作为参考。

##### 后端流程

本子任务只提供 trait 和模型，不接入 Hub API。后续 `rex-transfer` 任务模型会根据源/目标 connector 调用：

```text
source.metadata()
source.read()
target.write()
```

##### 测试 / QA 标准

**单元测试：**

- `file_type_serializes_as_expected`
- `file_entry_roundtrips_json`
- `file_read_carries_entry_and_bytes`
- `file_write_carries_path_and_bytes`
- `file_connector_trait_is_object_safe`

**集成测试：**

- 不需要真实 SSH/SFTP、不需要本地文件系统真实 connector。

##### 提交边界

单个 commit：`feat: add file connector trait`

不得包含：

- SSH/SFTP connector。
- 本地文件 connector。
- `/api/transfers` 或 `/ws/transfers`。
- 临时文件写入、SHA256、冲突策略、审计日志。
- 前端文件页面或“发送到…”交互。

目标：实现文件操作和“发送到…”。

任务：

1. 实现 `FileConnector` trait。
2. 实现 SSH/SFTP connector。
3. 实现本地文件 connector。
4. 实现 `rex-transfer` 任务模型。
5. 实现 `/api/transfers`。
6. 实现 `/ws/transfers`。
7. 实现临时文件写入和 SHA256 校验。
8. 实现冲突策略。
9. 前端实现文件列表。
10. 前端实现右键“发送到…”。
11. 前端实现传输队列。

完成标准：

- 文件数据不经过浏览器。
- 可以从一个 SFTP 连接传到另一个 SFTP 连接。
- 传输进度实时展示。
- 可以取消传输。
- 文件操作写入审计日志。

### 27.6 里程碑 5：SQL 控制台 MVP

目标：实现数据库查询能力。

任务：

1. 实现 `rex-mysql` 查询接口。
2. 实现 `rex-postgresql` 查询接口。
3. 实现数据库列表查询。
4. 实现表结构查询。
5. 实现 SQL 执行分页结果。
6. 前端实现 `features/sql`。
7. 前端实现查询标签。
8. 前端实现 SQL 编辑器。
9. 前端实现结果表格。
10. 前端实现高危 SQL 确认。

完成标准：

- 可以连接 MySQL/PostgreSQL。
- 可以执行 SELECT。
- 结果分页展示。
- 查询记录进入审计日志。
- AI 生成 SQL 不自动执行。

### 27.7 里程碑 6：前端正式工程

目标：把原型交互迁移为 Vue 功能域。

任务：

1. 初始化 `packages/rex-console-web`。
2. 建立 `features/` 目录结构。
3. 实现通用组件：`ContextMenu`、`ConfirmDialog`、`Toast`、`DataTable`。
4. 实现主题系统。
5. 实现 i18n。
6. 实现 `AppLayout`、`FullScreenLayout`、`WorkspaceLayout`。
7. 迁移登录页。
8. 迁移仪表盘。
9. 迁移环境管理。
10. 迁移 Agent 管理。
11. 迁移设置页。
12. 迁移工作区、终端、SQL、文件页。

完成标准：

- 页面按功能域组织。
- `pages/` 只保留路由入口。
- 通用组件不混入业务 feature。
- 原型中的关键交互在正式前端中复现。

### 27.8 里程碑 7：自动更新第一阶段

目标：Hub 能提示更新，但不自动替换。

任务：

1. Hub worker 定期查询 GitHub Releases。
2. 解析当前平台对应 release asset。
3. 比较当前版本和最新版本。
4. 设置页显示“有新版本”。
5. Agent 页面显示 Hub/Agent 版本总览。
6. Agent 心跳上报版本和 SHA256。
7. Hub 判断 Agent 是否需要更新。

完成标准：

- Hub 能检测新版本。
- 设置页展示更新提示。
- Agent 页面展示版本总览。
- 不自动下载或替换二进制。

### 27.9 里程碑 8：自动更新第二阶段

目标：Hub 和 Agent 自动更新并可回滚。

任务：

1. 实现 `update-state.json` schema。
2. 实现状态原子写入。
3. 实现 staging 下载目录。
4. 实现 rollback 备份目录。
5. 实现 SHA256 校验。
6. 实现 Unix/Linux/macOS 原子替换。
7. 实现 worker 退出码：`10` 请求更新，`11` 健康失败，`12` 崩溃。
8. 实现 `REX_UPDATE_PENDING`。
9. Hub 新 worker 健康检查 `/healthz`。
10. Agent 新 worker 健康检查连接 Hub。
11. 连续 3 次失败回滚。
12. 实现 Windows supervisor 副本逻辑。

完成标准：

- 新版本健康时完成更新。
- 新版本启动失败可回滚旧版。
- 不会进入无限更新循环。
- Agent 更新后保持同一 Agent ID。
- Docker 内不需要 s6-overlay。

### 27.10 里程碑 9：打包与发布

目标：形成可发布产物。

任务：

1. 配置 GitHub Actions 构建 Linux/macOS/Windows 二进制。
2. 构建 Hub 二进制并嵌入前端静态资源。
3. 构建 Agent 二进制。
4. 生成 SHA256SUMS。
5. Hub 部署包包含同版本 Agent 二进制。
6. 构建 Docker 镜像。
7. 编写 Release 说明。
8. 编写用户升级说明。

完成标准：

- Release 中包含所有平台 Hub 二进制。
- Release 中包含所有平台 Agent 二进制。
- Release 中包含 SHA256SUMS。
- Docker 镜像可启动。
- Agent 页面下载按钮能根据平台推荐命令。

---

## 28. 推荐开发顺序

第一阶段建议不要同时做所有协议。推荐顺序：

1. 项目骨架和 supervisor。
2. Hub API、SQLite、登录。
3. Agent 注册和心跳。
4. SSH 终端。
5. SSH/SFTP 文件传输。
6. 跨连接传输。
7. SQL 控制台。
8. 前端功能域重构。
9. 自动更新检测。
10. 自动更新替换和回滚。

这样每个阶段都能形成一个可运行闭环，而不是等所有协议都完成后才能验证。
