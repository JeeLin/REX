# M55: Agent 注册流程修复

## Context
M54 修复了 Agent Token 显示问题，但调查发现更深层的架构缺陷：当前 Agent 注册令牌只在"重置"时生成，环境创建时无令牌可显示。`create_agent` 没有 API endpoint，Agent 记录无法通过正常流程创建。用户无法获取令牌来部署 Agent。

版本类型：minor（新功能，完整的 Agent 注册流程）

## 产品边界
本阶段实现完整的 Agent 注册令牌生成和管理流程。不涉及 Agent 自动更新、Agent 隧道等其他功能。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 后端：环境创建时自动生成注册令牌 | ⬜ |
| 2 | 后端：Agent WebSocket 认证时自动创建 Agent 记录 | ⬜ |
| 3 | 前端：环境详情页正确显示和管理注册令牌 | ⬜ |
| 4 | 数据库迁移：清理未使用的 `agent_token` 列 | ⬜ |

## 子任务详细设计

### 1 后端：环境创建时自动生成注册令牌

- **功能目标**：创建环境时自动生成 UUID v4 令牌并存储，确保环境详情页始终可显示令牌
- **文件结构**（修改）：
  - `crates/rex-hub/src/db.rs` — `create_environment` 方法生成令牌
  - `crates/rex-hub/src/migrations.sql` — `environments` 表新增 `registration_token` 列
- **接口设计**：`POST /api/environments` 返回的 Environment 对象新增 `registration_token` 字段
- **后端流程**：
  1. 创建环境时生成 `uuid::Uuid::new_v4().to_string()` 作为注册令牌
  2. 存储到 `environments.registration_token` 列
  3. 返回给前端
- **测试标准**：创建环境后，`registration_token` 字段非空
- **提交信息**：`feat(agent): generate registration token on environment creation`

### 2 后端：Agent WebSocket 认证时自动创建 Agent 记录

- **功能目标**：Agent 用令牌连接 Hub 时，如不存在对应 Agent 记录则自动创建
- **文件结构**（修改）：
  - `crates/rex-hub/src/agent_ws.rs` — 认证逻辑增加自动创建
  - `crates/rex-hub/src/db.rs` — 新增 `find_or_create_agent_by_token` 方法
- **接口设计**：无新增 API，修改现有 WebSocket 认证流程
- **后端流程**：
  1. Agent 发送 auth 消息（包含 token）
  2. Hub 通过 token 查找 `environments.registration_token`
  3. 如找到环境，检查该环境下是否有匹配 token 的 Agent 记录
  4. 如无，自动创建 Agent 记录（name 从 auth 消息获取）
  5. 返回 auth_ok
- **测试标准**：Agent 用有效令牌连接时自动注册成功
- **提交信息**：`feat(agent): auto-create agent record on WebSocket auth`

### 3 前端：环境详情页正确显示和管理注册令牌

- **功能目标**：环境详情页始终显示注册令牌（从 `environments.registration_token` 获取），支持复制和重置
- **文件结构**（修改）：
  - `packages/rex-console-web/src/pages/EnvironmentDetailPage.vue` — 令牌显示逻辑
  - `packages/rex-console-web/src/api/environments.ts` — 类型定义
- **接口设计**：环境详情 API 返回 `registration_token` 字段
- **交互设计**：
  - 令牌始终可见（从环境表获取，不再依赖 Agent 记录）
  - 复制按钮始终可用
  - 重置按钮：调用新 API 重新生成令牌
- **提交信息**：`feat(agent): display registration token from environment`

### 4 数据库迁移：清理未使用的 `agent_token` 列

- **功能目标**：移除 `agents` 表中未使用的 `agent_token` 列，重命名 `token_hash` 为 `registration_token`（如在 environments 表中已实现则跳过）
- **文件结构**（修改）：
  - `crates/rex-hub/src/migrations.sql` — 新增迁移
- **提交信息**：`chore(db): cleanup unused agent_token column`

## 设计核对点

- 符合产品文档 3.10 节 Agent 管理流程
- 不引入多用户/RBAC 概念
- 前端命令使用 `bun`
- 后端依赖使用 `workspace = true`

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
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

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
