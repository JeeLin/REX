# M12: Agent 管理

## Context

M8-M11 完成了基础设施、环境/资源管理、工作区核心和 SQL 控制台。Agent 管理页面目前是假数据。M12 实现 Agent 注册、心跳、状态跟踪和管理页面。

本里程碑版本类型：minor（新功能），版本号 0.12.0 → 0.13.0。

## 产品边界

**本阶段做：**
- Agent 注册 API（POST /api/agents/register）
- Agent 心跳 API（POST /api/agents/:id/heartbeat）
- Agent 在线/离线判定（3 分钟无心跳 → offline）
- Agent 管理页重写（真实数据）
- 令牌重置

**本阶段不做：**
- Agent WebSocket 隧道（后续里程碑）
- Agent 二进制下载/部署指南
- Agent 配置/日志查看

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Agent 注册 + 心跳 API | ⬜ |
| 2 | Agent 管理页重写（真实数据） | ⬜ |

## 子任务详细设计

### 1 Agent 注册 + 心跳 API

**功能目标**

提供 Agent 注册、心跳、状态查询 API。

**文件结构**

新建：
- `crates/rex-hub/src/agent_api.rs`

修改：
- `crates/rex-hub/src/rex-hub.rs` — 注册路由
- `crates/rex-hub/src/lib.rs` — 添加模块

**接口设计**

```
POST   /api/agents/register           → { agent_id }   { token, name, version, os, arch, hostname }
POST   /api/agents/:id/heartbeat      → { ok }          { version, ip }
GET    /api/environments/:env_id/agents → Agent[]
GET    /api/agents/:id                → Agent
POST   /api/agents/:id/reset-token    → { token }
```

**后端流程**

- 注册：验证 token hash → 创建 agent 记录 → 返回 agent_id
- 心跳：更新 last_seen_at 和 ip → 标记 status 为 online
- 离线判定：查询时检查 last_seen_at，超过 3 分钟标记 offline

**提交信息**

```
feat(agent): add agent registration and heartbeat API
```

### 2 Agent 管理页重写

**功能目标**

将 AgentsPage 从假数据改为真实 API 数据。

**文件结构**

修改：
- `packages/rex-console-web/src/pages/AgentsPage.vue` — 重写
- `packages/rex-console-web/src/api/agents.ts` — 新建

**交互设计**

- 卡片网格展示：名称、状态（StatusDot）、环境、版本、设备信息
- 令牌重置弹窗
- 空状态引导

**提交信息**

```
feat(web): rewrite agents management page with real API
```

## 设计核对点

- [ ] Agent 注册验证 token hash
- [ ] 心跳更新 last_seen_at
- [ ] 在线/离线判定逻辑正确
- [ ] 管理页显示真实数据
- [ ] type-check + cargo check 通过

## Flow Status

- [ ] 步骤1：编写里程碑文档
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
