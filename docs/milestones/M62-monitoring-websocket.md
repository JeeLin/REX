# M62: 健康监控与 WebSocket 增强

## Context
M61 完成了主题优化和通知系统。本里程碑聚焦系统可观测性和连接可靠性提升。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段做什么：
- 暴露 Prometheus 格式系统指标
- Agent WebSocket 断线重连机制
- 心跳间隔可配置

本阶段不做什么：
- 不新增功能模块
- 不引入多用户/RBAC 概念

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 健康监控：Prometheus 指标端点 | ✅ |
| 2 | WebSocket 增强：连接重连 + 心跳优化 | ✅ |
| 3 | SQLite 新建资源路径错误修复 | ✅ |
| 4 | Agent Token 按钮 i18n 和重置功能 | ✅ |
| 5 | 直连环境隐藏 Agent Token 相关 UI | ⬜ |
| 6 | 新建资源连接方式由环境决定 | ⬜ |
| 7 | 新建资源向导合并为单页 | ⬜ |

## 子任务详细设计

### 1 健康监控：Prometheus 指标端点

- **功能目标**：暴露 Prometheus 格式的系统指标
- **文件结构**（创建）：
  - `crates/rex-hub/src/metrics.rs` — 指标模块
  - `crates/rex-hub/src/rex-hub.rs` — 注册端点
- **接口设计**：`GET /metrics` 返回 Prometheus 格式指标
- **指标内容**：
  - `rex_hub_requests_total` — 请求总数
  - `rex_hub_request_duration_seconds` — 请求延迟
  - `rex_hub_agent_connections` — Agent 连接数
  - `rex_hub_uptime_seconds` — 运行时间
- **后端流程**：axum 中间件收集指标，返回 Prometheus 格式
- **测试标准**：指标端点返回正确格式，数值准确
- **提交信息**：`feat(metrics): add Prometheus metrics endpoint`

### 2 WebSocket 增强：连接重连 + 心跳优化

- **功能目标**：Agent 断线自动重连，心跳间隔可配置
- **文件结构**（修改）：
  - `crates/rex-agent/src/agent_ws.rs` — 重连逻辑
  - `crates/rex-agent/src/rex-agent.rs` — 心跳配置
- **交互设计**：
  - Agent 断线后指数退避重连（1s → 2s → 4s → 8s → 最大 30s）
  - 心跳间隔可配置（默认 30s）
- **后端流程**：tokio-tungstenite 断线检测，自动重连
- **测试标准**：断线后自动重连，心跳正常发送
- **提交信息**：`feat(ws): add exponential backoff reconnect and configurable heartbeat`

### 3 SQLite 新建资源路径错误修复

- **功能目标**：修复 SQLite 资源创建时 host 字段硬编码为 localhost 的问题
- **文件结构**（修改）：
  - `packages/rex-console-web/src/features/resource/WizardModal.vue` — 修复路径处理
- **问题描述**：用户输入的文件路径被忽略，host 字段总是 localhost
- **测试标准**：SQLite 资源创建时使用用户输入的文件路径
- **提交信息**：`fix: use user input file path for SQLite resources`

### 4 Agent Token 按钮 i18n 和重置功能

- **功能目标**：添加国际化支持和 Token 重置按钮
- **文件结构**（修改）：
  - `packages/rex-console-web/src/features/agents/AgentTokenSection.vue` — 添加 i18n 和重置按钮
  - `packages/rex-console-web/src/i18n/locales/zh.json` — 中文翻译
  - `packages/rex-console-web/src/i18n/locales/en.json` — 英文翻译
- **交互设计**：Token 显示区域添加"重置"按钮，点击弹出确认对话框
- **测试标准**：按钮文字支持中英文切换，重置功能正常
- **提交信息**：`feat(agent): add i18n and token reset button`

### 5 直连环境隐藏 Agent Token 相关 UI

- **功能目标**：直连模式下不显示 Agent Token 相关 UI
- **文件结构**（修改）：
  - `packages/rex-console-web/src/features/agents/AgentTokenSection.vue` — 条件渲染
  - `packages/rex-console-web/src/features/resource/WizardModal.vue` — 隐藏 Token 输入
- **问题描述**：直连环境不需要 Agent，不应显示 Token 相关 UI
- **测试标准**：直连模式下隐藏所有 Agent Token 相关元素
- **提交信息**：`fix: hide agent token UI for direct connection environments`

### 6 新建资源连接方式由环境决定

- **功能目标**：向导中连接方式继承环境设置，不让用户选择
- **文件结构**（修改）：
  - `packages/rex-console-web/src/features/resource/WizardModal.vue` — 移除连接方式选择
- **问题描述**：资源连接方式应由所属环境决定，而非用户在向导中选择
- **测试标准**：新建资源时自动继承环境的连接方式
- **提交信息**：`fix: inherit connection type from environment in resource wizard`

### 7 新建资源向导合并为单页

- **功能目标**：将多步向导合并为单页表单
- **文件结构**（修改）：
  - `packages/rex-console-web/src/features/resource/WizardModal.vue` — 重写为单页布局
- **问题描述**：当前向导分为多步，操作繁琐
- **交互设计**：
  - 单页布局，左侧协议选择，右侧表单
  - 所有字段（协议、名称、颜色、连接详情）在同一页面
  - 底部"测试连接"和"创建"按钮
- **测试标准**：单页表单可正常创建资源
- **提交信息**：`refactor(resource): merge wizard into single page form`

## 设计核对点

- 符合产品文档描述
- 不引入多用户/RBAC 概念
- 后端依赖使用 `workspace = true`
- 前端组件按功能域组织

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
| ⬜ | 🔴 | M62: SQLite 新建资源路径错误 | 用户反馈 | host 字段硬编码为 localhost 而非用户输入的文件路径 |
| ⬜ | 🟡 | M62: Agent Token 按钮缺 i18n 和重置 | 用户反馈 | 按钮文字硬编码，缺少重置按钮 |
| ⬜ | 🟡 | M62: 直连环境不需要 Agent Token | 用户反馈 | 直连模式下不应显示 Agent Token 相关 UI |
| ⬜ | 🟡 | M62: 新建资源连接方式应由环境决定 | 用户反馈 | 向导中不应让用户选择连接方式，应继承环境设置 |
| ⬜ | 🟡 | M62: 新建资源向导应合并为单页 | 用户反馈 | 所有步骤（协议选择、名称/颜色、连接详情、确认）应合并为一个页面 |
| ⬜ | 🟡 | 连接树资源项状态列显示为空 | 用户反馈 | 连接树中资源项有名称但状态列（cs/cm/csc）全部显示为"-"，应该是显示环境/协议/状态等信息 |
