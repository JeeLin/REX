# M9: 环境 + 资源管理

## Context

M8 建立了基础设施层：SQLite 数据库 + 认证系统 + 路由框架 + 前端 auth store。环境/资源表已建好，db.rs 已有环境 CRUD 方法，但无 API handler，无前端管理页面（现有 EnvironmentsPage 是假数据）。

M9 是工作区的数据来源——连接树侧栏需要从 API 读取环境→资源数据，Tab 系统需要资源信息来建立连接。

本里程碑版本类型：minor（新功能），版本号 0.9.0 → 1.0.0。

## 产品边界

**本阶段做：**
- 环境 CRUD API（列表/详情/创建/编辑/删除）
- 资源 CRUD API（列表/详情/创建/编辑/删除）+ 测试连接
- 环境管理页（卡片网格，对接真实 API）
- 环境详情页（Agent 面板占位 + 资源表格）
- 资源创建向导（4 步：协议→基本信息→连接详情→完成）
- 前端 API 层（environments.ts + resources.ts）
- 侧栏连接树对接 API

**本阶段不做：**
- Agent 注册/心跳/WebSocket 隧道（M12）
- 工作区 Tab 嵌入终端/SQL/Redis/文件组件（M10）
- Dashboard/审计日志/设置页改造（M13）
- 凭据 AES 加密（M14，M9 先在 config_json 中明文存储密码）
- 资源属性对话框（深度配置，M10+）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 环境 CRUD API（env_api.rs） | ✅ |
| 2 | 资源 CRUD API + 测试连接（resource_api.rs） | ✅ |
| 3 | 前端 API 层 + 环境 store | ✅ |
| 4 | 环境管理页重写（真实数据） | ✅ |
| 5 | 环境详情页 + 路由 | ✅ |
| 6 | 资源创建向导（4 步） | ✅ |
| 7 | 侧栏连接树对接 API | ✅ |

## 子任务详细设计

### 1 环境 CRUD API

**功能目标**

提供环境的完整 CRUD REST API，复用 db.rs 已有的方法。

**文件结构**

新建：
- `crates/rex-hub/src/env_api.rs` — 环境 API handler + 路由

修改：
- `crates/rex-hub/src/db.rs` — 添加 `list_environments_with_stats` 方法（含资源数）
- `crates/rex-hub/src/models.rs` — 添加 `EnvironmentDetail`（含 resource_count、agent_status）
- `crates/rex-hub/src/lib.rs` — 添加 `pub mod env_api;`
- `crates/rex-hub/src/rex-hub.rs` — 注册 `/api/environments` 路由

**接口设计**

```
GET    /api/environments                 → EnvironmentDetail[]
GET    /api/environments/:id             → EnvironmentDetail
POST   /api/environments                 → Environment    { name, description?, connection_mode? }
PUT    /api/environments/:id             → Environment    { name?, description?, connection_mode? }
DELETE /api/environments/:id             → { ok: true }
```

**数据模型**

```rust
// EnvironmentDetail 在 Environment 基础上增加关联查询字段
pub struct EnvironmentDetail {
    #[serde(flatten)]
    pub environment: Environment,
    pub resource_count: i64,
    pub agent_status: Option<String>,  // "online" | "offline" | null（无 agent）
}
```

**后端流程**

1. `env_routes()` 返回 `Router<AppState>`
2. 每个 handler 通过 `State(state)` 提取 AppState，调用 `state.db` 方法
3. 删除环境时，由于外键 ON DELETE CASCADE，资源和 agent 自动删除
4. 创建/更新后写审计日志

**测试标准**

- curl 测试：创建 → 列表（含 resource_count）→ 编辑 → 删除
- 删除不存在的环境返回 404
- 创建重名环境返回 409

**提交信息**

```
feat(env): add environment CRUD API with resource count
```

### 2 资源 CRUD API + 测试连接

**功能目标**

提供资源的完整 CRUD REST API，含测试连接端点。

**文件结构**

新建：
- `crates/rex-hub/src/resource_api.rs` — 资源 API handler + 路由

修改：
- `crates/rex-hub/src/db.rs` — 添加资源 CRUD 方法（list_resources_by_env、get_resource、create_resource、update_resource、delete_resource）
- `crates/rex-hub/src/models.rs` — 添加 `NewResource`、`UpdateResource`、`ResourceDetail`（含 environment_name）
- `crates/rex-hub/src/lib.rs` — 添加 `pub mod resource_api;`
- `crates/rex-hub/src/rex-hub.rs` — 注册 `/api/environments/:env_id/resources` 和 `/api/resources/test-connection` 路由

**接口设计**

```
GET    /api/environments/:env_id/resources           → Resource[]
POST   /api/environments/:env_id/resources           → Resource    { name, protocol, host, port?, username?, config_json?, color?, sort_order? }
GET    /api/environments/:env_id/resources/:id        → Resource
PUT    /api/environments/:env_id/resources/:id        → Resource    { name?, protocol?, host?, port?, ... }
DELETE /api/environments/:env_id/resources/:id        → { ok: true }
POST   /api/resources/test-connection                 → { ok: true, latency_ms: u64 } | { ok: false, error: string }
```

**数据模型**

```rust
pub struct NewResource {
    pub name: String,
    pub protocol: String,       // ssh|mysql|postgresql|redis|sqlite|sftp|s3
    pub host: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub config_json: Option<String>,  // JSON: { password, private_key, database_name, ... }
    pub color: Option<String>,
    pub sort_order: Option<i64>,
}

pub struct TestConnectionRequest {
    pub protocol: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub config_json: Option<String>,
}
```

**config_json 结构（按协议）**

```jsonc
// SSH / SFTP
{ "password": "...", "private_key": "...", "passphrase": "...", "encoding": "utf-8" }

// MySQL / PostgreSQL
{ "password": "...", "database_name": "mydb" }

// Redis
{ "password": "...", "db": 0 }

// SQLite
{ "file_path": "/path/to/db.sqlite" }

// S3
{ "access_key": "...", "secret_key": "...", "bucket": "...", "region": "...", "endpoint": "..." }
```

**测试连接流程**

1. 接收 TestConnectionRequest
2. 根据 protocol 尝试连接：
   - SSH: `tokio::net::TcpStream::connect` → 超时 5s
   - MySQL: `rex_mysql::MySqlConnector::connect` → 超时 5s
   - PostgreSQL: `rex_postgresql::PgConnector::connect` → 超时 5s
   - Redis: `redis::Cmd::send_packed_command` → PING
   - SQLite: `rusqlite::Connection::open` + `SELECT 1`
   - S3: `aws_sdk_s3::Client::list_buckets` → 超时 5s
3. 返回 { ok, latency_ms } 或 { ok: false, error }

**测试标准**

- 资源 CRUD 完整流程
- 测试连接：至少 SQLite 和 Redis 可本地验证
- 删除环境时级联删除资源

**提交信息**

```
feat(resource): add resource CRUD API with connection testing
```

### 3 前端 API 层 + 环境 store

**功能目标**

建立前端与后端环境/资源 API 的对接层，以及 Pinia store 管理状态。

**文件结构**

新建：
- `packages/rex-console-web/src/api/environments.ts` — 环境 API 封装
- `packages/rex-console-web/src/api/resources.ts` — 资源 API 封装
- `packages/rex-console-web/src/stores/environments.ts` — 环境+资源 Pinia store

**接口设计**

```typescript
// api/environments.ts
import { api } from './client'

export interface Environment {
  id: string
  name: string
  description: string
  connection_mode: string
  resource_count: number
  agent_status: string | null
  created_at: string
  updated_at: string
}

export interface NewEnvironment {
  name: string
  description?: string
  connection_mode?: string
}

export const environmentsApi = {
  list: () => api.get<Environment[]>('/environments'),
  get: (id: string) => api.get<Environment>(`/environments/${id}`),
  create: (data: NewEnvironment) => api.post<Environment>('/environments', data),
  update: (id: string, data: Partial<NewEnvironment>) => api.put<Environment>(`/environments/${id}`, data),
  delete: (id: string) => api.delete(`/environments/${id}`),
}

// api/resources.ts
export interface Resource {
  id: string
  environment_id: string
  name: string
  protocol: string
  host: string
  port: number | null
  username: string
  config_json: string
  color: string | null
  sort_order: number
  created_at: string
  updated_at: string
}

export interface TestConnectionResult {
  ok: boolean
  latency_ms?: number
  error?: string
}

export const resourcesApi = {
  listByEnv: (envId: string) => api.get<Resource[]>(`/environments/${envId}/resources`),
  get: (envId: string, id: string) => api.get<Resource>(`/environments/${envId}/resources/${id}`),
  create: (envId: string, data: NewResource) => api.post<Resource>(`/environments/${envId}/resources`, data),
  update: (envId: string, id: string, data: Partial<NewResource>) => api.put<Resource>(`/environments/${envId}/resources/${id}`, data),
  delete: (envId: string, id: string) => api.delete(`/environments/${envId}/resources/${id}`),
  testConnection: (data: TestConnectionRequest) => api.post<TestConnectionResult>('/resources/test-connection', data),
}
```

**stores/environments.ts**

```typescript
export const useEnvironmentsStore = defineStore('environments', () => {
  const environments = ref<Environment[]>([])
  const loading = ref(false)

  async function fetchEnvironments() { ... }
  async function createEnvironment(data: NewEnvironment) { ... }
  async function updateEnvironment(id: string, data: Partial<NewEnvironment>) { ... }
  async function deleteEnvironment(id: string) { ... }

  // 资源操作（嵌套在环境内）
  async function fetchResources(envId: string) { ... }
  async function createResource(envId: string, data: NewResource) { ... }
  async function deleteResource(envId: string, id: string) { ... }
  async function testConnection(data: TestConnectionRequest) { ... }

  return { environments, loading, fetchEnvironments, ... }
})
```

**测试标准**

- API 层类型正确
- store 能正确调用 API 并更新状态
- `bun run type-check` 通过

**提交信息**

```
feat(web): add environment and resource API layer with Pinia store
```

### 4 环境管理页重写

**功能目标**

将 EnvironmentsPage 从假数据改为真实 API 数据，支持创建/编辑/删除环境。

**文件结构**

修改：
- `packages/rex-console-web/src/pages/EnvironmentsPage.vue` — 重写

**交互设计**

- 页面加载时 `fetchEnvironments()` 拉取数据
- 卡片网格展示：环境名、描述、Agent 状态（StatusDot）、资源数（Badge）、连接方式标签
- 点击卡片 → 跳转 `/environments/:id` 详情页
- 「+ New Environment」按钮 → 弹出创建对话框（名称、描述、连接方式下拉）
- 卡片右上角下拉菜单：编辑、删除（确认后删除）
- 空状态：显示引导创建界面
- 删除前二次确认

**后端流程**

无额外后端工作，复用子任务 1 的 API。

**测试标准**

- 创建环境 → 卡片出现
- 编辑环境 → 卡片更新
- 删除环境 → 卡片消失
- 空状态正确显示
- `bun run type-check` + `bun run lint` 通过

**提交信息**

```
feat(web): rewrite environments page with real API
```

### 5 环境详情页 + 路由

**功能目标**

新增环境详情页，展示单个环境的完整信息、Agent 状态、资源列表。

**文件结构**

新建：
- `packages/rex-console-web/src/pages/EnvironmentDetailPage.vue`

修改：
- `packages/rex-console-web/src/router/index.ts` — 添加 `/environments/:id` 路由

**交互设计**

页面布局：
- 顶部面包屑：Environments > {环境名}
- 环境信息区：名称（可编辑 inline）、描述（可编辑 inline）、连接方式标签、创建时间
- Agent 面板（占位）：显示「No agent registered」或 agent 状态卡片（名称、状态点、版本、IP、OS）—— M12 才完整实现
- 资源表格：当前环境的所有资源，列：协议图标+名称、主机、端口、用户名、状态、操作（编辑/删除）
- 「+ Add Resource」按钮 → 打开资源创建向导（子任务 6）

**路由**

```typescript
{ path: 'environments/:id', name: 'environment-detail', component: () => import('../pages/EnvironmentDetailPage.vue') },
```

**测试标准**

- 从环境管理页点击卡片 → 进入详情页
- 详情页显示正确的环境信息
- 资源表格显示该环境的资源
- 返回按钮回到环境列表
- `bun run type-check` + `bun run lint` 通过

**提交信息**

```
feat(web): add environment detail page with resource table
```

### 6 资源创建向导

**功能目标**

4 步向导，引导用户创建资源。参考 PRODUCT.md §3.4。

**文件结构**

新建：
- `packages/rex-console-web/src/features/resource/WizardModal.vue` — 向导主组件
- `packages/rex-console-web/src/features/resource/StepProtocol.vue` — 步骤 1：选择协议
- `packages/rex-console-web/src/features/resource/StepBasic.vue` — 步骤 2：基本信息
- `packages/rex-console-web/src/features/resource/StepConnection.vue` — 步骤 3：连接详情
- `packages/rex-console-web/src/features/resource/StepConfirm.vue` — 步骤 4：确认完成
- `packages/rex-console-web/src/features/resource/connectionForms.ts` — 各协议的表单定义

**交互设计**

向导流程（Modal 形式，宽 640px）：

**步骤 1 — 选择协议**
- 7 个协议卡片网格（SSH / SFTP / MySQL / PostgreSQL / Redis / SQLite / S3）
- 每个卡片：协议图标 + 名称 + 色值标签
- 点击选中 → 高亮边框 → 下一步

**步骤 2 — 基本信息**
- 名称（必填）
- 连接方式：直连 / Agent（radio）
- 颜色标记（8 色圆点选择器）

**步骤 3 — 连接详情（按协议动态表单）**

| 协议 | 字段 |
|------|------|
| SSH | 主机、端口(22)、用户名、认证方式(密码/私钥)、密码/私钥内容、编码 |
| SFTP | 同 SSH |
| MySQL | 主机、端口(3306)、用户名、密码、默认数据库 |
| PostgreSQL | 主机、端口(5432)、用户名、密码、默认数据库 |
| Redis | 主机、端口(6379)、密码、默认 DB(0) |
| SQLite | 文件路径 |
| S3 | Endpoint URL、Access Key、Secret Key、Bucket、Region |

每个表单底部有「测试连接」按钮，点击后调用 `testConnection` API，显示结果（成功延迟 / 失败原因）。

**步骤 4 — 确认完成**
- 配置摘要：协议、主机、端口、用户名
- 「创建」按钮 → 调用 createResource API → 关闭向导 → 刷新资源列表

**向导 props**

```typescript
interface Props {
  visible: boolean
  environmentId: string
  editResourceId?: string  // 编辑模式
}
```

**测试标准**

- 完整流程：选协议 → 填信息 → 填连接 → 测试连接 → 确认 → 创建成功
- 编辑模式：预填已有数据
- 表单验证：必填字段、端口范围
- `bun run type-check` + `bun run lint` 通过

**提交信息**

```
feat(web): add 4-step resource creation wizard
```

### 7 侧栏连接树对接 API

**功能目标**

将侧栏 ResourcePanel（连接树）从硬编码数据改为从 API 读取。

**文件结构**

修改：
- `packages/rex-console-web/src/features/resource-panel/ResourcePanel.vue` — 重写数据源

**交互设计**

- 从 `environmentsStore.fetchEnvironments()` 拉取数据
- 树结构：环境节点（可折叠）→ 资源节点（按协议图标着色）
- 搜索框：实时过滤环境名/资源名
- 单击资源：选中高亮（后续 M10 触发 Tab 打开）
- 右键菜单：打开 / 属性 / 删除
- 双击资源：打开（emit 事件，M10 实现）
- 空状态：引导用户创建环境

**数据模型**

```typescript
interface EnvNode {
  id: string
  name: string
  type: 'environment'
  children: ResourceNode[]
  expanded: boolean
}

interface ResourceNode {
  id: string
  environmentId: string
  name: string
  protocol: 'ssh' | 'mysql' | 'postgresql' | 'redis' | 'sqlite' | 'sftp' | 's3'
  host: string
  color?: string
}
```

**测试标准**

- 侧栏显示真实环境→资源树
- 创建资源后侧栏自动刷新
- 搜索过滤正常
- `bun run type-check` + `bun run lint` 通过

**提交信息**

```
feat(web): wire sidebar connection tree to real API
```

## 设计核对点

- [ ] 单用户模型，无多用户/RBAC 概念
- [ ] 数据库 schema 与 models.rs 一致
- [ ] API 路由遵循 RESTful 约定
- [ ] config_json 结构支持 7 种协议
- [ ] 测试连接超时合理（5s）
- [ ] 删除环境级联删除资源
- [ ] 前端使用 REX 设计系统组件
- [ ] 前端 API 调用通过统一 client.ts（自动注入 auth header）
- [ ] 侧栏连接树数据来自 API，非硬编码
- [ ] 资源密码在 config_json 中明文（M14 加密）

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
