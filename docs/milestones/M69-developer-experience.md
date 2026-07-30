# M69: Developer Experience

## Context
M68 完成性能优化。本里程碑聚焦开发体验，提供调试工具和错误追踪能力。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段做什么：
- 前端调试面板（环境变量、连接状态、性能指标）
- 错误追踪（全局错误边界 + 错误上报）
- 后端请求日志增强（响应时间、错误详情）
- 开发者友好的错误消息

本阶段不做什么：
- 不修改生产环境日志输出（使用 tracing，已完善）
- 不引入外部 APM 服务
- 不修改认证/授权逻辑

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 前端调试面板 | ⬜ |
| 2 | 错误追踪增强 | ⬜ |
| 3 | 后端日志增强 | ⬜ |
| 4 | 错误消息国际化 | ⬜ |

## 子任务详细设计

### 1 前端调试面板

- **功能目标**：提供开发者友好的调试信息面板
- **文件结构**（创建）：
  - `packages/rex-console-web/src/components/DebugPanel.vue` — 调试面板组件
- **功能**：
  - 环境信息（版本号、Node 环境、构建时间）
  - 连接状态（Hub URL、Agent 状态、WebSocket 连接数）
  - 性能指标（内存使用、FPS、API 响应时间）
  - 快捷键 `Ctrl+Shift+D` 切换显示
- **测试标准**：
  - 生产环境默认隐藏
  - 快捷键切换正常
- **提交信息**：`feat(dx): add developer debug panel`

### 2 错误追踪增强

- **功能目标**：增强全局错误处理和错误信息展示
- **文件结构**（修改）：
  - `packages/rex-console-web/src/components/ErrorBoundary.vue` — 增强错误展示
  - `packages/rex-console-web/src/api/client.ts` — API 错误详情记录
- **功能**：
  - ErrorBoundary 显示更详细的错误信息（文件名、行号、堆栈）
  - API 错误记录到 console（开发模式）
  - 错误计数和最近错误列表
- **提交信息**：`feat(dx): enhance error tracking and display`

### 3 后端日志增强

- **功能目标**：增强后端日志的可读性和调试价值
- **文件结构**（修改）：
  - `crates/rex-hub/src/middleware.rs` — 请求日志增强
- **增强内容**：
  - 请求日志包含请求体大小（POST/PUT）
  - 响应日志包含响应体大小
  - 错误响应记录完整错误链
  - 慢请求警告（> 1s）
- **提交信息**：`feat(dx): enhance backend request logging`

### 4 错误消息国际化

- **功能目标**：确保所有错误消息都支持 i18n
- **文件结构**（修改）：
  - `packages/rex-console-web/src/i18n/locales/en.json` — 英文错误消息
  - `packages/rex-console-web/src/i18n/locales/zh.json` — 中文错误消息
- **内容**：
  - API 错误消息翻译
  - 网络错误消息翻译
  - 表单验证错误消息翻译
- **提交信息**：`feat(dx): add i18n for error messages`

## 设计核对点

- 调试面板不泄露敏感信息
- 错误追踪不降低用户体验
- 日志增强不影响性能
- i18n 覆盖所有用户可见错误消息

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

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
