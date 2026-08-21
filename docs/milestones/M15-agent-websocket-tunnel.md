# M15: Agent WebSocket 隧道

## Context

M0–M14 完成了从设计系统到管理页面的全部功能开发。但 Agent 的核心价值——**通过 WebSocket 隧道代理内网资源连接**——尚未实现。当前 Agent 模式环境只是数据壳，无法实际连接内网资源。

M15 实现 Agent ↔ Hub 的 WebSocket 隧道。**所有 Agent 通信走单一 WebSocket 连接**，不使用 REST API。这是产品的核心差异化功能："一个服务访问公网 + 内网资源"。

本里程碑版本类型：minor（新功能），版本号 0.15.0 → 0.16.0。

## 产品边界

**本阶段做：**
- Agent WebSocket 协议（认证、心跳、资源连接、数据转发全部走 WebSocket）
- Hub 侧 WebSocket 服务器（接受 Agent 连接 + channel 多路复用）
- Agent 侧 WebSocket 客户端（连接 Hub + 处理 channel 请求）
- 前端连接路由（Agent 模式资源通过隧道连接）
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
| 1 | Hub Agent WebSocket 服务器（单一连接：认证 + 心跳 + 隧道 + channel 多路复用） | ✅ |
| 2 | Agent WebSocket 客户端（连接 Hub + 处理 channel 请求） | ✅ |
| 3 | 前端连接路由改造（Agent 模式资源通过隧道连接） | ✅ |
| 4 | Agent 部署指南文档 | ✅ |
| 5 | 端到端测试 | ✅ |

## 子任务详细设计

### 1 Hub Agent WebSocket 服务器

**功能目标**

Hub 端实现 WebSocket 服务器。Agent 通过单一 WebSocket 连接完成所有操作：认证、心跳、资源连接请求、数据转发。不使用 REST API。

**文件结构**

新建：
- `crates/rex-hub/src/agent_ws.rs` — Agent WebSocket 隧道处理

修改：
- `crates/rex-hub/src/rex-hub.rs` — 添加 `/ws/agent` 路由
- `crates/rex-hub/src/agent_api.rs` — 移除 REST 端点（register/heartbeat/reset-token），保留只读查询供前端管理页使用
- `crates/rex-hub/src/app.rs` — 添加 `agent_connections: Arc<RwLock<HashMap<String, AgentConnection>>>` 到 AppState

**WebSocket 协议（单一连接，全功能）**

```
GET /ws/agent?token=<agent_token>

连接建立后的消息流（JSON 文本帧）：

═══ 认证 ═══
→ Agent: { "type": "auth", "payload": { "agent_id": "...", "token": "..." } }
← Hub:   { "type": "auth_ok", "payload": { "agent_id": "..." } }
      or { "type": "auth_fail", "payload": { "reason": "..." } }

═══ 心跳 ═══
→ Agent: { "type": "heartbeat", "payload": { "version": "...", "os": "...", "arch": "...", "hostname": "..." } }
← Hub:   { "type": "heartbeat_ack" }

═══ 资源连接（Hub → Agent 请求，Agent → Hub 响应）═══
← Hub:   { "type": "connect", "payload": { "requestId": "req_1", "resourceId": "res_1", "protocol": "ssh", "config": { "host": "...", "port": 22, "username": "...", "password": "..." } } }
→ Agent: { "type": "connected", "payload": { "requestId": "req_1", "channelId": "ch_1" } }
      or { "type": "connect_error", "payload": { "requestId": "req_1", "error": "connection refused" } }

═══ 数据转发（二进制帧，前 4 字节为 channelId）═══
→ Agent: [4B channelId][raw data]
← Hub:   [4B channelId][raw data]

═══ 关闭 channel ═══
← Hub:   { "type": "close", "payload": { "channelId": "ch_1" } }
→ Agent: { "type": "closed", "payload": { "channelId": "ch_1" } }
```

**关键设计决策**

1. **文本帧 vs 二进制帧分离**：控制消息（auth/heartbeat/connect）用 JSON 文本帧；数据转发用二进制帧（前 4 字节 channelId + 原始数据），避免 base64 编码开销
2. **Agent 主动注册**：Agent 连接 WebSocket 后发送 `auth` 消息完成注册，不需要单独的 REST register 接口
3. **心跳双向**：Agent 定期发送 heartbeat（含设备信息），Hub 回复 ack。30 秒无心跳 → 标记 offline
4. **channel 多路复用**：多个资源连接复用同一条 WebSocket，通过 channelId 区分

**后端流程**

1. Agent 通过 `GET /ws/agent?token=<token>` 建立 WebSocket 连接
2. Agent 发送 `auth` 消息 → Hub 验证 token → 更新 Agent 信息到 DB → 回复 `auth_ok`
3. Agent 定期发送 `heartbeat` → Hub 更新 last_seen_at + 设备信息
4. 当用户从前端发起 Agent 模式资源连接时：
   a. Hub 查找目标 Agent 的 WebSocket 连接
   b. 发送 `connect` 消息（含 requestId, resourceId, protocol, config）
   c. Agent 响应 `connected`（含 channelId）或 `connect_error`
   d. Hub 将 channel 注册，后续二进制帧通过 channelId 双向转发
5. Agent 断开 WebSocket → 标记 offline，关闭所有关联 channel，通知前端
6. 前端查询 Agent 状态（`GET /api/agents`）从 DB 读取（由 WebSocket 事件更新）

**数据模型**

```rust
pub struct AgentConnection {
    pub agent_id: String,
    pub ws_sender: mpsc::Sender<ws::Message>,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub hostname: String,
}

pub struct Channel {
    pub channel_id: String,
    pub agent_id: String,
    pub resource_id: String,
    pub request_id: String,
}
```

**测试标准**

- Agent 连接 → auth → auth_ok → DB 中状态 online
- heartbeat → DB 更新 last_seen_at + 设备信息
- connect → Agent 响应 connected → 二进制帧双向转发
- Agent 断开 → DB 状态 offline → 所有 channel 关闭
- 无效 token → auth_fail → 连接关闭
- cargo clippy + cargo test 通过

**提交信息**

```
feat(agent): add Agent WebSocket tunnel server with channel multiplexing
```

### 2 Agent WebSocket 客户端

**功能目标**

Agent 端实现 WebSocket 客户端，通过单一连接完成所有操作：认证、心跳上报、资源连接代理。不使用 REST API。

**文件结构**

修改：
- `crates/rex-agent/src/main.rs` — 添加 WebSocket 客户端逻辑

**Agent 启动流程**

1. 读取配置（hub_url, agent_token, agent_id, version, os, arch, hostname）
2. 通过 WebSocket 连接 Hub（`ws://<hub>/ws/agent?token=<token>`）
3. 发送 `auth` 消息（含 agent_id + token）→ 等待 `auth_ok`
4. 启动心跳循环（30 秒间隔，含 version/os/arch/hostname）
5. 监听 `connect` 消息 → 建立本地连接 → 回复 `connected`
6. 二进制帧双向转发（前 4 字节 channelId + 原始数据）
7. 断线自动重连（指数退避）

**本地连接建立**

根据 protocol 字段分发：
- `ssh` → TCP 连接 + SSH 握手
- `mysql` → TCP 连接 + MySQL 握手
- `postgresql` → TCP 连接 + PostgreSQL 握手
- `redis` → TCP 连接 + Redis PING
- `sftp` → 复用 SSH 连接
- `sqlite` → 本地文件打开

**测试标准**

- Agent 启动 → 连接 Hub → auth → auth_ok → heartbeat 正常
- 收到 connect → 建立本地连接 → 二进制帧双向转发
- 断线重连
- 无效 token → 退出

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
3. 如果 `agent`：调用 `connectViaAgent(resourceId)` → Hub 查找 Agent → 发送 connect → 获取 tunnelId → 建立 WebSocket 到 `/ws/tunnel/:tunnelId`
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

**提交信息**

```
docs: add Agent deployment guide (binary, Docker, Compose)
```

### 5 端到端测试

**功能目标**

验证 Agent 隧道的完整流程：WebSocket 连接 → 认证 → 心跳 → 资源连接 → 数据转发 → 断开。

**测试矩阵**

| 场景 | 验证点 |
|------|--------|
| WebSocket 连接 + 认证 | token 验证、状态 online |
| 心跳 | 设备信息写入 DB、last_seen_at 更新 |
| SSH 隧道连接 | 终端可交互、命令执行返回结果 |
| MySQL 隧道连接 | SQL 查询返回结果 |
| Redis 隧道连接 | PING/PONG 正常 |
| Agent 断开 | 状态 offline、channel 关闭、前端显示断开 |
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

- [ ] Agent WebSocket 连接 Hub → 认证成功 → 状态 online
- [ ] Agent 心跳 → 设备信息写入 DB
- [ ] Agent 断开 → 状态 offline
- [ ] Agent 模式 SSH 资源 → 通过隧道连接 → 终端可交互
- [ ] Agent 模式 MySQL 资源 → 通过隧道连接 → SQL 查询正常
- [ ] Agent 模式 Redis 资源 → 通过隧道连接 → PING/PONG 正常
- [ ] 直连模式资源行为不变
- [ ] Agent 离线时前端显示明确错误
- [ ] 无 REST API 用于 Agent 注册/心跳（全部走 WebSocket）
- [ ] cargo test 通过
- [ ] type-check + lint + build 通过

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
