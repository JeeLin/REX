# M40: 工作区快捷键补全 + Agent 日志查看

## Context

M39 完成了集成完善和会话管理。本里程碑补全 PRODUCT.md §5 中定义但尚未实现的工作区快捷键，并为 Agent 管理页增加日志查看功能（基于审计日志），提升运维效率。

版本类型：minor（新功能），版本号 0.36.0 → 0.37.0。

## 产品边界

**本阶段做：**
- 工作区缺失的快捷键（Ctrl+N、F11、Alt+6-9）
- Agent 日志查看（从审计日志中筛选 agent 相关记录）
- Agent 注册/断开事件写入审计日志

**本阶段不做：**
- Agent 实时日志流（需要 Agent 端日志上报协议，复杂度高）
- 新 UI 组件或页面
- 性能优化

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 工作区快捷键补全（Ctrl+N、F11、Alt+6-9） | ✅ |
| 2 | Agent 审计日志写入（注册/心跳/断开事件） | ✅ |
| 3 | Agent 日志查看（审计日志按 agent 筛选 + 前端查看器） | ✅ |

## 子任务详细设计

### 1 工作区快捷键补全

**功能目标**

补全 PRODUCT.md §5 中定义但尚未实现的工作区快捷键：
- `Ctrl+N`：新建连接（打开 Quick Connect 或导航到资源创建向导）
- `F11`：全屏切换
- `Alt+6~9`：跳转到第 6-9 个标签页

**文件结构**

修改：
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — 添加快捷键处理器

**快捷键设计**

```typescript
// Ctrl+N: 新建连接
{ key: 'n', ctrl: true, handler: () => { router.push('/resource-new') } },

// F11: 全屏切换
{ key: 'F11', handler: () => { fullscreen.value = !fullscreen.value } },

// Alt+6-9: 跳转到第 6-9 个标签
{ key: '6', alt: true, handler: () => switchToTabByIndex(5) },
{ key: '7', alt: true, handler: () => switchToTabByIndex(6) },
{ key: '8', alt: true, handler: () => switchToTabByIndex(7) },
{ key: '9', alt: true, handler: () => switchToTabByIndex(8) },
```

注意：Alt+1-5 已被布局预设占用，Alt+6-9 仅用于标签跳转。

**测试标准**

- Ctrl+N 打开资源创建页面
- F11 切换全屏模式
- Alt+6-9 跳转到对应标签页
- `bun run type-check` + `bun run lint` 通过

**提交信息**: `feat(workspace): add missing keyboard shortcuts (Ctrl+N, F11, Alt+6-9)`

### 2 Agent 审计日志写入

**功能目标**

在 Agent 注册、心跳、断开连接时写入审计日志，为子任务 3 的日志查看提供数据。

**文件结构**

修改：
- `crates/rex-hub/src/agent_api.rs` — 注册时写入审计日志
- `crates/rex-hub/src/agent_ws.rs` — 心跳/断开时写入审计日志

**审计日志条目**

```rust
// Agent 注册
NewAuditEntry {
    action: "AGENT_REGISTER".into(),
    target: Some(agent_name),
    environment_id: Some(env_id),
    agent_id: Some(agent_id),
    result: "success".into(),
    detail: format!("{} {} {}", version, os, arch),
}

// Agent 心跳（仅首次上线时记录，不记录每次心跳）
NewAuditEntry {
    action: "AGENT_ONLINE".into(),
    target: Some(agent_name),
    agent_id: Some(agent_id),
    result: "success".into(),
    detail: format!("ip={}", ip),
}

// Agent 断开
NewAuditEntry {
    action: "AGENT_OFFLINE".into(),
    target: Some(agent_name),
    agent_id: Some(agent_id),
    result: "success".into(),
    detail: String::new(),
}
```

**测试标准**

- Agent 注册后审计日志中有 AGENT_REGISTER 记录
- Agent 上线后有 AGENT_ONLINE 记录
- Agent 断开后有 AGENT_OFFLINE 记录
- `cargo test --workspace` 通过

**提交信息**: `feat(agent): write audit log entries for agent register, online, offline`

### 3 Agent 日志查看

**功能目标**

在 Agent 管理页增加日志查看功能，从审计日志中筛选指定 agent 的操作记录。

**文件结构**

修改：
- `packages/rex-console-web/src/api/agents.ts` — 新增 `getAgentLogs` API 方法
- `packages/rex-console-web/src/pages/AgentsPage.vue` — 新增日志查看弹窗
- `packages/rex-console-web/src/i18n/locales/zh.json` — 添加日志相关翻译
- `packages/rex-console-web/src/i18n/locales/en.json` — 添加日志相关翻译

**后端 API**

审计日志 API 已存在（`GET /api/audit-log?agent_id=xxx`），无需新增后端端点。只需前端调用时传入 `agent_id` 参数。

```typescript
// api/agents.ts
getAgentLogs: (agentId: string) =>
  api.get<AuditEntry[]>(`/audit-log?agent_id=${agentId}`),
```

**前端交互**

- Agent 卡片新增「日志」按钮
- 点击打开日志查看弹窗（Modal）
- 弹窗内显示该 agent 的审计日志列表（时间、操作、结果、详情）
- 支持按操作类型筛选（AGENT_REGISTER / AGENT_ONLINE / AGENT_OFFLINE）
- 最多显示最近 100 条

**测试标准**

- 点击 Agent 日志按钮打开弹窗
- 显示该 agent 的审计日志记录
- 操作类型筛选正常工作
- `bun run type-check` + `bun run lint` 通过

**提交信息**: `feat(web): add agent log viewer in agents management page`

## 设计核对点

- ✅ 不引入多用户、RBAC、企业协作概念
- ✅ Agent 日志基于现有审计日志，不引入新数据源
- ✅ 快捷键遵循 PRODUCT.md §5 定义
- ✅ 前端命令使用 bun
- ✅ 依赖声明符合 workspace 规则

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

（打回时追加一条，创建里程碑文档时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
