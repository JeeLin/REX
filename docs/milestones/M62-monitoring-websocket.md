# M62: Health Monitoring & WebSocket Enhancement

## Context
M61 完成了主题优化和通知系统。本里程碑从健康监控和 WebSocket 增强两个维度提升系统可观测性和连接可靠性。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段提升系统可观测性和连接可靠性，不涉及新功能模块。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 健康监控：Prometheus 指标端点 | ⬜ |
| 2 | WebSocket 增强：连接重连 + 心跳优化 | ⬜ |

## 子任务详细设计

### 1 健康监控：Prometheus 指标端点

- **功能目标**：暴露 Prometheus 格式的系统指标
- **文件结构**（修改）：
  - `crates/rex-hub/src/metrics.rs` — 新增指标模块
  - `crates/rex-hub/src/rex-hub.rs` — 注册 /metrics 端点
- **接口设计**：`GET /metrics` 返回 Prometheus 格式指标
- **指标内容**：
  - `rex_hub_requests_total` — 请求总数
  - `rex_hub_request_duration_seconds` — 请求延迟
  - `rex_hub_agent_connections` — Agent 连接数
  - `rex_hub_uptime_seconds` — 运行时间
- **提交信息**：`feat(metrics): add Prometheus metrics endpoint`

### 2 WebSocket 增强：连接重连 + 心跳优化

- **功能目标**：Agent 断线自动重连，心跳间隔可配置
- **文件结构**（修改）：
  - `crates/rex-agent/src/agent_ws.rs` — 重连逻辑
  - `crates/rex-agent/src/rex-agent.rs` — 心跳配置
- **交互设计**：
  - Agent 断线后指数退避重连（1s → 2s → 4s → 8s → 最大 30s）
  - 心跳间隔可配置（默认 30s）
- **提交信息**：`feat(ws): add exponential backoff reconnect and configurable heartbeat`

## 设计核对点

- 符合产品文档描述
- 不引入多用户/RBAC 概念
- 后端依赖使用 `workspace = true`

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

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
| [ ] | 🟡 | 直连环境不需要 Agent Token | 用户反馈 | 直连模式下不应显示 Agent Token 相关 UI |
| [ ] | 🟡 | Agent Token 按钮缺 i18n 和重置 | 用户反馈 | 按钮文字硬编码，缺少重置按钮 |
| [ ] | 🔴 | SQLite 新建资源路径错误 | 用户反馈 | host 字段硬编码为 localhost 而非用户输入的文件路径 |
| ⬜ | 🟡 | 新建资源连接方式应由环境决定 | 用户反馈 | 向导中不应让用户选择连接方式，应继承环境设置 |
| ⬜ | 🟢 | 新建资源向导步骤可合并 | 用户反馈 | 名称/颜色（步骤2）与连接详情（步骤3）可合并为一步 |
