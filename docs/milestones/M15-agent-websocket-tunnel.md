# M15: Agent WebSocket 隧道

## Context

M0–M14 完成了从设计系统到管理页面的全部功能开发，包括 Agent 注册/心跳 API 和管理页面。但 Agent 的核心价值——**通过 WebSocket 隧道代理内网资源连接**——尚未实现。当前 Agent 模式环境只是数据壳，无法实际连接内网资源。

M15 实现 Agent ↔ Hub 的 WebSocket 隧道，让 Agent 模式的环境真正可用。这是产品的核心差异化功能："一个服务访问公网 + 内网资源"。

本里程碑版本类型：minor（新功能），版本号 0.15.0 → 0.16.0。

## 产品边界

**本阶段做：**
- Agent WebSocket 隧道协议（Hub 侧 WebSocket 服务器）
- Agent WebSocket 客户端（Agent 侧连接 Hub）
- 前端连接路由（通过 Agent 代理连接内网资源）
- Agent 部署指南文档
- 端到端测试

**本阶段不做：**
- Agent 自动更新机制（M16）
- TLS/HTTPS（M17）
- Agent 日志远程查看
- Agent 配置远程修改

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Hub Agent WebSocket 服务器（隧道协议 + channel 多路复用） | ⬜ |
| 2 | Agent WebSocket 客户端（连接 Hub + 处理 channel 请求） | ⬜ |
| 3 | 前端连接路由改造（Agent 模式资源通过隧道连接） | ⬜ |
| 4 | Agent 部署指南文档 | ⬜ |
| 5 | 端到端测试 | ⬜ |

## 子任务详细设计

### 1 Hub Agent WebSocket 服务器

**功能目标**

Hub 端实现 WebSocket 服务器，接受 Agent 连接，管理 Agent 生命周期（在线/离线），并为每个资源连接请求创建独立的 channel，通过 WebSocket 帧双向转发数据。

**文件结构**

新建：
- `crates/rex-hub/src/agent_ws.rs` — Agent WebSocket 隧道处理

修改：
- `crates/rex-hub/src/rex-hub.rs` — 添加 `/ws/agent` 路由
- `crates/rex-hub/src/agent_api.rs` — 添加 `list_online_agents()` 辅助方法
- `crates/rex-hub/src/app.rs` — 添加 `agent_channels: Arc<RwLock<HashMap<String, AgentChannel>>>` 到 AppState

**接口设计**

```
GET /ws/agent?token=<agent_token>

消息协议（JSON）：
→ Hub: { "type": "auth", "payload": { "agent_id": "...", "token": "..." } }
← Hub: { "type": "auth_ok" } | { "type": "auth_fail", "payload": { "reason": "..." } }

→ Hub: { "type": "heartbeat" }
← Hub: { "type": "heartbeat_ack" }

← Hub: { "type": "resource.connect", "payload": { "requestId": "...", "resourceId": "...", "protocol": "ssh", "config": {...} } }
→ Hub: { "type": "resource.connected", "payload": { "requestId": "...", "channelId": "..." } }
→ Hub: { "type": "resource.error", "payload": { "requestId": "...", "error": "..." } }

← Hub: { "type": "channel.data", "payload": { "channelId": "...", "data": "base64..." } }
→ Hub: { "type": "channel.data", "payload": { "channelId": "...", "data": "base64..." } }

← Hub: { "type": "channel.close", "payload": { "channelId": "..." } }
→ Hub: { "type": "channel.close", "payload": { "channelId": "..." } }
```

**后端流程**

1. Agent 通过 `GET /ws/agent?token=<token>` 建立 WebSocket 连接
2. Hub 验证 token → 更新 Agent 状态为 online → 回复 `auth_ok`
3. Hub 维护 Agent 连接池（HashMap<agent_id, WebSocket>）
4. 当用户从前端发起 Agent 模式资源连接时：
   a. Hub 查找目标 Agent 的 WebSocket 连接
   b. 发送 `resource.connect` 消息（含 requestId, resourceId, protocol, config）
   c. Agent 响应 `resource.connected`（含 channelId）或 `resource.error`
   d. Hub 将 channel 注册到连接池，后续 `channel.data` 帧双向转发
5. Agent 断开时：标记 offline，关闭所有关联 channel，通知前端
6. 30 秒无心跳 → 标记 offline

**数据模型**

```rust
pub struct AgentConnection {
    pub agent_id: String,
    pub ws_sender: mpsc::Sender<ws::Message>,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

pub struct Channel {
    pub channel_id: String,
    pub agent_id: String,
    pub resource_id: String,
    pub request_id: String,
}
```

**测试标准**

- Agent 连接 → auth_ok → 状态 online
- 发送 resource.connect → Agent 响应 connected → channel.data 双向转发
- Agent 断开 → 状态 offline → 所有 channel 关闭
- 无效 token → auth_fail
- cargo clippy + cargo test 通过

**提交信息**

```
feat(agent): add Agent WebSocket tunnel server with channel multiplexing
```

### 2 Agent WebSocket 客户端

**功能目标**

Agent 端实现 WebSocket 客户端，连接 Hub，处理认证、心跳、资源连接请求。收到 `resource.connect` 后，通过协议 crate 建立到内网资源的实际连接，然后通过 WebSocket channel 双向转发数据。

**文件结构**

修改：
- `crates/rex-agent/src/main.rs` — 添加 WebSocket 客户端逻辑

**接口设计**

Agent 启动流程：
1. 读取配置（hub_url, agent_token, agent_id）
2. 通过 WebSocket 连接 Hub（`ws://<hub>/ws/agent?token=<token>`）
3. 发送 `auth` 消息 → 等待 `auth_ok`
4. 启动心跳循环（30 秒间隔）
5. 监听 `resource.connect` 消息 → 建立本地连接 → 回复 `resource.connected`
6. 双向转发 `channel.data`

**本地连接建立**

根据 protocol 字段分发：
- `ssh` → TCP 连接 + SSH 握手
- `mysql` → TCP 连接 + MySQL 握手
- `postgresql` → TCP 连接 + PostgreSQL 握手
- `redis` → TCP 连接 + Redis PING
- `sftp` → 复用 SSH 连接
- `s3` → HTTP 连接（通过 Agent 代理）

**测试标准**

- Agent 启动 → 连接 Hub → auth_ok → 状态 online
- 收到 resource.connect → 建立本地连接 → channel.data 双向转发
- 心跳正常发送
- 断开重连

**提交信息**

```
feat(agent): add WebSocket client with resource connection proxying
```

### 3 前端连接路由改造

**功能目标**

前端在打开 Agent 模式资源时，通过 Hub 的 Agent 隧道代理连接，而非直连。对用户透明——打开 SSH 资源的体验与直连完全一致。

**文件结构**

修改：
- `packages/rex-console-web/src/features/workspace/WorkspacePage.vue` — 资源连接时判断 connection_mode
- `packages/rex-console-web/src/api/resources.ts` — 添加 `connectViaAgent(resourceId)` 方法

**接口设计**

```
POST /api/resources/:id/connect-agent
Body: { protocol: "ssh", config: {...} }
Response: { tunnelId: "...", status: "connected" }

// WebSocket 连接到隧道
GET /ws/tunnel/:tunnelId?token=<jwt>
```

**前端流程**

1. 用户在连接树点击资源
2. 前端检查资源所属环境的 `connection_mode`
3. 如果 `agent`：调用 `connectViaAgent(resourceId)` → 获取 tunnelId → 建立 WebSocket 到 `/ws/tunnel/:tunnelId`
4. 如果 `direct`：现有直连流程不变
5. 终端/SQL/Redis/文件组件无需修改——它们通过 WebSocket 收发数据，不关心底层是直连还是隧道

**测试标准**

- 创建 Agent 模式环境 + 资源 → 打开终端 → 通过 Agent 隧道连接
- 直连模式资源行为不变
- Agent 离线时显示"Agent 离线"错误
- type-check + lint + build 通过

**提交信息**

```
feat(web): route Agent-mode resource connections through WebSocket tunnel
```

### 4 Agent 部署指南文档

**功能目标**

提供 Agent 部署的完整文档，覆盖二进制、Docker、Docker Compose 三种方式。

**文件结构**

新建：
- `docs/agent-deploy.md` — Agent 部署指南

**内容**

1. 前置条件（Hub URL、注册令牌获取方式）
2. 二进制部署（下载 → 配置 → 启动 → systemd service）
3. Docker 部署（docker run 命令 + 环境变量）
4. Docker Compose 部署（compose.yml 示例）
5. 配置文件说明（hub_url, token, agent_id, 心跳间隔等）
6. 故障排查（连接失败、token 无效、防火墙）
7. 安全建议（token 保管、网络限制）

**测试标准**

- 文档步骤可执行
- 三种部署方式均有可用示例

**提交信息**

```
docs: add Agent deployment guide (binary, Docker, Compose)
```

### 5 端到端测试

**功能目标**

验证 Agent 隧道的完整流程：Agent 注册 → WebSocket 连接 → 资源连接 → 数据转发 → 断开。

**测试矩阵**

| 场景 | 验证点 |
|------|--------|
| Agent 注册 + 连接 | token 验证、状态 online |
| SSH 隧道连接 | 终端可交互、命令执行返回结果 |
| MySQL 隧道连接 | SQL 查询返回结果 |
| Redis 隧道连接 | PING/PONG 正常 |
| Agent 断开 | 状态 offline、前端显示断开 |
| Agent 重连 | 自动恢复连接 |
| 并发连接 | 多个资源同时通过 Agent 连接 |

**测试标准**

- `cargo test --workspace` 全部通过
- `bun run type-check` + `bun run lint` 通过
- 手动验证（或自动化）SSH/MySQL/Redis 隧道连接

**提交信息**

```
test: add Agent WebSocket tunnel integration tests
```

## 设计核对点

- [ ] Agent 连接 Hub → 状态显示 online
- [ ] Agent 断开 → 状态显示 offline
- [ ] Agent 模式 SSH 资源 → 通过隧道连接 → 终端可交互
- [ ] Agent 模式 MySQL 资源 → 通过隧道连接 → SQL 查询正常
- [ ] Agent 模式 Redis 资源 → 通过隧道连接 → PING/PONG 正常
- [ ] 直连模式资源行为不变
- [ ] Agent 离线时前端显示明确错误
- [ ] cargo test 通过
- [ ] type-check + lint + build 通过

## Flow Status

- [x] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
