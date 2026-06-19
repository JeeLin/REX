# M2: Agent 连接

## Context

M0 交付了 Rust workspace 骨架（rex-common、rex-hub、rex-agent）。M1 实现了 Hub HTTP API 服务、配置加载、SQLite 存储、环境/资源 CRUD、登录认证、审计日志，以及对应的前端管理页面。M2 在此基础上实现 Agent 与 Hub 的连接能力：Agent 配置加载、身份持久化、注册、WebSocket 心跳、在线状态管理，以及前端的 Agent 管理页面。

## 产品边界

**做什么：**
- Agent 配置加载（agent.yaml + 环境变量覆盖）
- Agent 身份持久化（agent.json）
- Agent 注册 API（Hub 端验证 token + Agent 端发送注册请求）
- Agent WebSocket 连接 + 心跳（Hub 端 WS handler + Agent 端 WS 客户端）
- Agent 在线状态管理（Agent 列表 API）
- 前端：Agent API client + agent store
- 前端：环境详情页显示 Agent 状态
- 前端：Agent 管理页（列表 + 状态 + 部署指南）

**不做什么：**
- 资源通道协议（resource.connect / resource.connected）— M3+
- 终端/文件传输/SQL 代理通道 — M3+
- Agent 二进制下载端点 — M7+
- Agent 更新检测和自更新 — M7/M8
- Agent 日志查看器（WebSocket 实时推送）— 后续补充
- Agent 配置弹窗、重置令牌弹窗 — 后续补充

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 2.1 | Agent 配置加载 | 后端 | ✅ |
| 2.2 | Agent 身份持久化 | 后端 | ✅ |
| 2.3 | Agent 注册 API | 后端 | ✅ |
| 2.4 | Agent WebSocket 连接 + 心跳 | 后端 | ✅ |
| 2.5 | Agent 在线状态管理 API | 后端 | ✅ |
| 2.6 | Agent API client + agent store | 前端 | ✅ |
| 2.7 | 环境详情页 Agent 状态面板 | 前端 | ✅ |
| 2.8 | Agent 管理页 | 前端 | ✅ |

---

## 子任务 2.1：Agent 配置加载 ✅

### 功能目标

Agent worker 启动时加载配置文件 `agent.yaml`，支持 `--config` 参数和 `REX_*` 环境变量覆盖。配置字段：`server`（Hub 地址）、`token`（Agent token）、`name`（Agent 名称）、`data_dir`（数据目录）。

### 文件结构

```text
crates/rex-agent/src/
├── main.rs       修改：worker 模式加载配置、打印配置信息
└── config.rs     新增：AgentConfig 结构体和加载逻辑
```

根 `Cargo.toml` 已有 `serde` 和 `serde_yaml` 依赖。

### 接口设计

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AgentConfig {
    pub server: String,
    pub token: String,
    pub name: String,
    pub data_dir: PathBuf,
}
```

环境变量覆盖：

| 环境变量 | 配置字段 | 说明 |
|----------|----------|------|
| `REX_SERVER` | `server` | Hub 服务器地址 |
| `REX_TOKEN` | `token` | Agent token |
| `REX_NAME` | `name` | Agent 名称 |
| `REX_DATA_DIR` | `data_dir` | 数据目录 |

### 提交边界

```text
feat: add AgentConfig loading with env override
```

---

## 子任务 2.2：Agent 身份持久化 ✅

### 功能目标

Agent 首次启动时生成唯一 ID 并保存到 `agent.json`，后续启动时加载已有身份。保证 Agent 重启后保持同一 ID。

### 文件结构

```text
crates/rex-agent/src/
├── main.rs       修改：加载或创建身份
└── identity.rs   新增：AgentIdentity 结构体 + load/create
```

### 数据模型

`{data_dir}/agent.json`：

```json
{
  "id": "agt_a1b2c3d4",
  "name": "内网 Agent",
  "token": "rex_env_xxx",
  "created_at": "2026-06-17T12:00:00Z"
}
```

### 提交边界

```text
feat: add agent identity persistence
```

---

## 子任务 2.3：Agent 注册 API ✅

### 功能目标

Agent 启动后向 Hub 发送注册请求。Hub 验证 token、创建/更新 agent 记录并返回注册结果。本子任务只实现 HTTP 注册端点。

### 文件结构

```text
crates/rex-hub/src/
├── main.rs           修改：添加 mod agent
├── routes.rs         修改：注册 POST /api/agents/register（公开端点）
├── agent.rs          新增：Agent 注册 handler
└── db.rs             修改：添加 Agent CRUD 方法

crates/rex-agent/src/
├── main.rs           修改：worker 模式调用注册 API
└── client.rs         新增：Agent → Hub HTTP 客户端（reqwest）
```

### 接口设计

**请求：**

```http
POST /api/agents/register
Content-Type: application/json

{
  "id": "agt_a1b2c3d4",
  "token": "env注册令牌原文",
  "name": "prod-server",
  "version": "0.1.0",
  "sha256": "",
  "os": "linux",
  "arch": "amd64",
  "hostname": "prod-server",
  "os_version": "Ubuntu 22.04"
}
```

**成功响应（200）：**

```json
{
  "data": {
    "id": "agt_a1b2c3d4",
    "environment_id": "env_12345678",
    "status": "online"
  }
}
```

**失败响应（401）：**

```json
{
  "error": {
    "code": "INVALID_TOKEN",
    "message": "注册令牌无效"
  }
}
```

### 后端流程

```text
Agent POST /api/agents/register
  ↓
SHA256(token) → token_hash
  ↓
查询 environments 表，找到 agent_token_hash == token_hash 的环境
  ↓
未找到 → 返回 401
  ↓
找到 → INSERT OR REPLACE INTO agents
  ↓
返回 200 { id, environment_id, status: "online" }
```

### 测试标准

Hub 端：
1. token 匹配环境 → 200
2. token 不匹配 → 401
3. 同一 agent_id 重复注册 → 幂等更新

Agent 端：
4. AgentConfig 默认值正确
5. AgentIdentity 加载/创建

### 提交边界

```text
feat: add agent registration API
```

---

## 子任务 2.4：Agent WebSocket 连接 + 心跳 ✅

### 功能目标

Agent 注册成功后建立 WebSocket 长连接。双方通过 JSON 消息通信，Agent 每 30 秒发送心跳，Hub 更新在线状态。

### 文件结构

```text
crates/rex-common/src/
└── protocol.rs      新增：WebSocket 消息类型定义

crates/rex-hub/src/
├── main.rs          修改：添加 mod ws
├── routes.rs        修改：注册 /ws/agent 路由
├── ws.rs            新增：WebSocket handler
└── db.rs            修改：添加 heartbeat/status 更新方法

crates/rex-agent/src/
├── main.rs          修改：注册后建立 WebSocket 连接
└── ws.rs            新增：Agent WebSocket 客户端
```

### 消息协议

所有消息统一格式 `{ "msg_type": "...", "payload": {...} }`。

**Agent → Hub：**
- `auth`：连接后第一条，携带 agent_id + token
- `heartbeat`：每 30 秒，携带 version、os、arch、hostname、os_version、uptime、metrics

**Hub → Agent：**
- `heartbeat_ack`：心跳确认
- `disconnect`：通知断开（token 重置时）

### 后端流程（Hub 端）

```text
Agent WS 连接到 /ws/agent
  ↓
等待 auth 消息
  ↓
验证 agent_id + token → 失败则 disconnect
  ↓
成功 → 记录连接 → 进入消息循环
  ↓
收到 heartbeat → 更新 agents 表 → 发送 heartbeat_ack
  ↓
连接断开 → status = "offline"
```

### 后端流程（Agent 端）

```text
注册成功 → 建立 WS 连接
  ↓
发送 auth → 等待 auth_ack
  ↓
启动 30s 心跳定时器 → 进入消息循环
  ↓
连接断开 → 指数退避重连（1s → 2s → ... → 60s 上限）
```

### 关键设计

- Hub 用 `tokio::sync::RwLock<HashMap<String, AgentConnection>>` 存在线 Agent
- Agent 断开自动从 map 移除
- 超过 90 秒未收到心跳标记为 offline

### 提交边界

```text
feat: add agent WebSocket connection and heartbeat
```

---

## 子任务 2.5：Agent 在线状态管理 API ✅

### 功能目标

提供 API 查询 Agent 列表及在线状态，前端可展示 Agent 状态信息。

### 文件结构

```text
crates/rex-hub/src/
├── agent.rs     修改：添加 list_agents handler
├── db.rs        修改：添加 list_agents_by_environment 方法
└── routes.rs    修改：添加路由
```

### 接口设计

```http
GET /api/environments/:env_id/agents
```

响应：

```json
{
  "data": [
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
      "last_seen_at": "2026-06-17T12:00:00Z"
    }
  ]
}
```

### 在线判定逻辑

`last_seen_at` 在 90 秒内 → online，否则 offline。

### 提交边界

```text
feat: add agent list API with online status
```

---

## 子任务 2.6：Agent API client + agent store ✅

### 功能目标

前端 API 请求封装（Agent 相关接口）。agent store 管理 Agent 列表状态。

### 文件结构

```text
packages/rex-console-web/src/
├── api/
│   └── agent.ts        新增：Agent API 调用函数
└── stores/
    └── agent.ts        新增：Agent 列表 + 状态管理
```

### 接口设计

```ts
// api/agent.ts
export function listAgents(envId: string): Promise<Agent[]>
```

```ts
// stores/agent.ts
interface Agent {
  id: string
  environment_id: string
  name: string
  version: string
  os: string
  arch: string
  hostname: string | null
  os_version: string | null
  status: 'online' | 'offline'
  last_seen_at: string | null
}

interface AgentStore {
  agentsByEnv: Record<string, Agent[]>
  fetchAgents(envId: string): Promise<void>
}
```

### 提交边界

```text
feat: add agent API client and agent store
```

---

## 子任务 2.7：环境详情页 Agent 状态面板 ✅

### 功能目标

在环境详情页 (`EnvironmentDetail.vue`) 增加 Agent 状态面板，显示当前环境下 Agent 的在线/离线状态、设备信息和基本元信息。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/EnvironmentDetail.vue   修改：添加 Agent 状态面板
├── features/agents/
│   └── AgentStatusPanel.vue      新增：Agent 状态展示组件
```

### 前端交互

参考 `prototype/agents.html` 和 `prototype/environment.html`：

- Agent 在线时显示 🟢 绿色状态点 + "在线"
- Agent 离线时显示 ⚫ 灰色状态点 + "离线"
- 设备信息：OS 图标（🐧 / 🍎 / 🪟）+ 系统版本 + 架构
- 元信息网格：版本、Agent ID、连接 IP、运行时间
- 无 Agent 时显示引导："尚未部署 Agent" + 部署指引链接

### 提交边界

```text
feat: add agent status panel to environment detail page
```

---

## 子任务 2.8：Agent 管理页 ✅

### 功能目标

参考 `prototype/agents.html`，实现 Agent 管理页面：Agent 卡片列表、设备信息、在线状态、部署指南。

### 文件结构

```text
packages/rex-console-web/src/
├── pages/Agents.vue              新增：Agent 管理页
├── features/agents/
│   ├── AgentCard.vue             新增：Agent 卡片组件
│   └── DeployGuide.vue           新增：部署指南组件
├── router.ts                     修改：添加 /agents 路由
├── i18n/zh.ts                    修改：添加 Agent 相关翻译
└── i18n/en.ts                    修改：添加 Agent 相关翻译
```

### 前端交互

参考 `prototype/agents.html`：

**Agent 卡片列表：**
- 每个 Agent 一张卡片，显示：名称、在线/离线状态、所属环境、设备信息（OS 图标 + 系统版本 + 架构）、版本号、Agent ID
- 离线 Agent 卡片降低透明度（opacity: 0.7）
- 空状态时显示 "暂无 Agent" + 部署引导

**部署指南：**
- 页面底部的内嵌部署教程
- 4 种部署方式标签页切换：二进制文件 / Docker / Docker Compose / 配置文件
- 每种方式提供命令行示例和复制按钮
- 展示注册令牌获取说明

### 提交边界

```text
feat: implement agent management page with deploy guide
```

---

## 设计核对点

- [x] 单用户、自托管定位：不引入多用户、RBAC
- [x] Agent 主动出站连接 Hub，内网不开放入站端口
- [x] Hub 和 Agent 版本必须一致，不存在跨版本兼容
- [x] WebSocket 心跳间隔 30 秒，90 秒超时标记离线
- [x] Agent 注册端点为公开端点（不需要 JWT 认证）
- [x] Agent 身份（agent.json）持久化，重启后保持同一 ID
- [x] 不引入 Agent 日志实时推送、重置令牌等超前功能
- [x] 前端 Agent 管理页与 prototype/agents.html 交互一致

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交
