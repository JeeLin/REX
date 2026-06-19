# M1: Hub 管理基础

## Context

M0 交付了 Rust workspace 骨架（rex-common、rex-binaries）和 Vue 3 前端工程（路由、主题、i18n、布局骨架）。M1 在此基础上实现 Hub 的核心管理能力：HTTP API 服务、配置加载、SQLite 存储、环境/资源 CRUD、登录认证、审计日志，以及对应的前端管理页面。

## 产品边界

**做什么：**
- Axum HTTP 服务框架 + `/healthz`
- hub.yaml 配置加载 + 环境变量覆盖
- SQLite 数据库初始化 + 表结构
- 环境 CRUD API
- 资源 CRUD API
- 单用户登录 API（JWT）
- 基础认证中间件
- 审计日志写入 + 查询 API
- 前端：登录页、仪表盘、侧边栏导航、环境管理页、资源创建向导

**不做什么：**
- Agent 注册/心跳/在线状态（M2）
- SSH/SFTP/SQL/文件传输协议（M3-M6）
- WebSocket 隧道（M2）
- TLS 配置
- 自动更新（M7-M8）
- Docker 构建（M9）

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 1.1 | Cargo workspace 版本号收敛 | 后端 | ✅ |
| 1.2 | Axum HTTP 框架 + `/healthz` | 后端 | ✅ |
| 1.3 | 配置加载（hub.yaml） | 后端 | ✅ |
| 1.4 | SQLite 初始化 | 后端 | ✅ |
| 1.5 | 环境 CRUD API | 后端 | ✅ |
| 1.6 | 资源 CRUD API | 后端 | ✅ |
| 1.7 | 登录 API（JWT） | 后端 | ✅ |
| 1.8 | 基础认证中间件 | 后端 | ✅ |
| 1.9 | 审计日志写入 + 查询 API | 后端 | ✅ |
| 1.10 | API client + auth store | 前端 | ✅ |
| 1.11 | 登录页实现 | 前端 | ✅ |
| 1.12 | 侧边栏导航 + 顶栏 | 前端 | ✅ |
| 1.13 | 仪表盘页面 | 前端 | ✅ |
| 1.14 | 环境管理页（列表 + 创建 + 详情） | 前端 | ✅ |
| 1.15 | 资源创建向导 | 前端 | ✅ |

---

## 子任务 1.1：Cargo workspace 版本号收敛 ✅

根 `Cargo.toml` 添加 `[workspace.package]`，子 crate 使用 `version.workspace = true`、`edition.workspace = true`。

---

## 子任务 1.2：Axum HTTP 框架 + `/healthz`

### 功能目标

Hub worker 启动后监听 HTTP 端口，提供 `/healthz` 健康检查端点。

### 文件结构

```text
crates/rex-binaries/src/bin/rex-hub.rs   修改：worker 模式启动 HTTP
crates/rex-hub/                          新增 crate
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── routes.rs     路由定义 + /healthz handler
    └── main.rs       HTTP 服务启动
```

### 接口设计

```rust
// rex-hub/src/routes.rs
pub fn app() -> Router {
    Router::new().route("/healthz", get(healthz))
}

async fn healthz() -> &'static str {
    "ok"
}
```

### 测试标准

```bash
cargo fmt --check && cargo clippy --workspace --all-targets && cargo test --workspace
```

验证点：`/healthz` 返回 HTTP 200 + "ok"。

### 提交边界

```text
feat: add axum HTTP server and /healthz endpoint
```

---

## 子任务 1.3：配置加载

### 功能目标

Hub worker 启动时加载 `hub.yaml`，支持 `--config` 参数指定路径，环境变量 `REX_*` 覆盖。

### 文件结构

```text
crates/rex-common/src/
└── config.rs         新增：通用配置加载逻辑

crates/rex-hub/src/
├── config.rs         新增：HubConfig 结构体
└── main.rs           修改：解析 --config，加载配置
```

### 接口设计

```yaml
# hub.yaml
listen: ":3000"
data_dir: "./data"
secret_key: "${REX_SECRET_KEY}"
```

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HubConfig {
    pub listen: String,
    pub data_dir: PathBuf,
    pub secret_key: String,
}
```

### 提交边界

```text
feat: add hub.yaml config loading with env override
```

---

## 子任务 1.4：SQLite 初始化

### 功能目标

Hub worker 启动时初始化 SQLite 数据库 `{data_dir}/hub.db`，创建核心表。

### 文件结构

```text
crates/rex-hub/src/
├── db.rs             新增：Database 结构体 + 迁移
└── migrations.sql    新增：建表 SQL
```

### 数据模型

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

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### 提交边界

```text
feat: add SQLite database initialization with schema
```

---

## 子任务 1.5：环境 CRUD API

### 功能目标

实现环境的完整 CRUD：列表、创建、详情、更新、删除。

### 文件结构

```text
crates/rex-hub/src/
├── env.rs            新增：环境 handler
├── routes.rs         修改：注册环境路由
└── db.rs             修改：添加环境 CRUD 方法
```

### 接口设计

```http
GET    /api/environments
POST   /api/environments
GET    /api/environments/:id
PUT    /api/environments/:id
DELETE /api/environments/:id
```

ID 格式：`env_{8位hex}`。删除时检查关联资源/Agent，有则返回 409。

### 提交边界

```text
feat: add environment CRUD API
```

---

## 子任务 1.6：资源 CRUD API

### 功能目标

实现资源的完整 CRUD，资源归属于环境。

### 文件结构

```text
crates/rex-hub/src/
├── resource.rs       新增：资源 handler
├── routes.rs         修改：注册资源路由
└── db.rs             修改：添加资源 CRUD 方法
```

### 接口设计

```http
GET    /api/environments/:env_id/resources
POST   /api/environments/:env_id/resources
GET    /api/environments/:env_id/resources/:id
PUT    /api/environments/:env_id/resources/:id
DELETE /api/environments/:env_id/resources/:id
```

协议类型：ssh、sftp、mysql、postgresql、redis、docker、sqlite、s3。连接模式：`agent_proxy`、`direct`。

### 提交边界

```text
feat: add resource CRUD API
```

---

## 子任务 1.7：登录 API（JWT）

### 功能目标

单用户登录，密码验证后返回 JWT token（HS256，7 天有效）。

### 文件结构

```text
crates/rex-hub/src/
├── auth.rs           新增：登录 handler + JWT 生成/验证
├── routes.rs         修改：注册登录路由
└── db.rs             修改：添加 settings 查询
```

### 接口设计

```http
POST /api/auth/login
```

请求：`{ "password": "admin" }`。成功返回 `{ "token": "...", "expiresAt": "..." }`。失败返回 401 AUTH_INVALID。默认密码 `admin`。

### 提交边界

```text
feat: add login API with JWT token
```

---

## 子任务 1.8：基础认证中间件

### 功能目标

所有 `/api/*` 端点（除 `/api/auth/login`）需携带 Bearer token。`/healthz` 公开。

### 文件结构

```text
crates/rex-hub/src/
├── auth.rs           修改：添加 auth_middleware
└── routes.rs         修改：分层路由（公开 + 受保护）
```

### 提交边界

```text
feat: add JWT auth middleware for protected routes
```

---

## 子任务 1.9：审计日志写入 + 查询 API

### 功能目标

所有写操作自动记录审计日志。提供查询 API（时间范围 + 类型过滤 + 分页）。

### 文件结构

```text
crates/rex-hub/src/
├── audit.rs          新增：审计日志 handler + 写入函数
├── routes.rs         修改：注册审计日志路由
├── env.rs            修改：CRUD 后写入审计日志
├── resource.rs       修改：CRUD 后写入审计日志
└── auth.rs           修改：登录成功/失败写入审计日志
```

### 接口设计

```http
GET /api/audit-log?from=&to=&type=&page=1&pageSize=50
```

### 提交边界

```text
feat: add audit log writing and query API
```

---

## 子任务 1.10：API client + auth store

### 功能目标

前端 API 请求封装（axios/fetch + token 注入 + 401 跳转登录）。auth store 管理登录态。

### 文件结构

```text
packages/rex-console-web/src/
├── api/
│   ├── client.ts      新增：axios 实例 + 拦截器
│   └── auth.ts        新增：登录 API 调用
├── stores/
│   └── auth.ts        新增：token 管理 + 登录/登出
```

### 提交边界

```text
feat: add API client and auth store
```

---

## 子任务 1.11：登录页实现

### 功能目标

参考 `prototype/login.html`，实现完整登录页：CRT 扫描线效果、用户名密码表单、记住我、版本号 footer。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/Login.vue    重写：完整登录页
```

### 前端交互

- 全屏居中卡片，深色背景 + CRT 扫描线
- 用户名 + 密码输入框
- 「记住我」复选框
- 登录按钮，loading 状态
- 失败提示（红色 toast）
- 底部版本号 `REX Hub v0.1 · 自托管`
- 登录成功跳转仪表盘

### 提交边界

```text
feat: implement login page with CRT visual effect
```

---

## 子任务 1.12：侧边栏导航 + 顶栏

### 功能目标

填充 AppLayout 的侧边栏和顶栏内容。

### 文件结构

```text
packages/rex-console-web/src/
├── layouts/AppLayout.vue   重写：完整侧边栏 + 顶栏
```

### 前端交互

- 侧边栏：Logo + 导航链接（仪表盘、环境、Agent、设置、审计日志）
- 顶栏：当前页面标题 + 主题切换 + 语言切换 + 退出登录
- 路由守卫：未登录跳转 /login

### 提交边界

```text
feat: implement sidebar navigation and topbar
```

---

## 子任务 1.13：仪表盘页面

### 功能目标

参考 `prototype/dashboard.html`，实现仪表盘：统计卡片 + 快速连接 + 环境卡片网格。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/Dashboard.vue    重写：完整仪表盘
```

### 前端交互

- 4 个统计卡片（环境数、资源数、Agent 在线数、今日操作数）
- 快速连接列表（最近使用资源）
- 环境卡片网格（名称、Agent 状态、描述、资源类型统计）
- 「创建新环境」虚线卡片

### 提交边界

```text
feat: implement dashboard with stats and environment cards
```

---

## 子任务 1.14：环境管理页

### 功能目标

参考 `prototype/environments.html` 和 `environment-new.html`，实现环境列表、创建、详情页。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/
│   ├── Environments.vue      新增：环境列表
│   ├── EnvironmentNew.vue    新增：创建环境
│   └── EnvironmentDetail.vue 新增：环境详情
├── router.ts                 修改：添加路由
```

### 前端交互

- 环境列表：卡片网格 + 创建入口
- 创建环境：名称 + 描述 + 连接方式选择
- 环境详情：Agent 面板 + 资源列表表格

### 提交边界

```text
feat: implement environment management pages
```

---

## 子任务 1.15：资源创建向导

### 功能目标

参考 `prototype/resource-new.html`，实现 4 步资源创建向导。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/ResourceNew.vue     新增：资源创建向导
├── router.ts                 修改：添加路由
```

### 前端交互

- 4 步向导：选择协议 → 基本信息 → 连接详情 → 完成
- 协议网格（8 种协议卡片）
- 不同协议显示不同连接表单

### 提交边界

```text
feat: implement resource creation wizard
```

---

## 设计核对点

- [ ] 单用户、自托管定位：不引入多用户、RBAC
- [ ] 文件传输数据不经过浏览器（M1 不涉及）
- [ ] Hub 单二进制 + supervisor + worker 模型
- [ ] 配置文件密码/密钥通过环境变量传入
- [ ] 前端登录页与 prototype/login.html 交互一致
- [ ] 前端仪表盘与 prototype/dashboard.html 布局一致
- [ ] 前端环境管理与 prototype/environments.html 一致
- [ ] 审计日志记录所有写操作
- [ ] 不引入 Agent 注册、SSH、SQL 等超前功能

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交
