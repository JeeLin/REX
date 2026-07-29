# M57: Stability, Mobile & Security

## Context
M56 完成了 UX 打磨。本里程碑从稳定性、移动端适配、安全加固三个维度全面提升产品质量。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段提升现有功能的质量，不涉及新功能模块。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 稳定性：全局错误边界 + 加载状态优化 | ⬜ |
| 2 | 移动端：响应式布局完善 + 交互优化 | ⬜ |
| 3 | 安全：CSP 头 + API 输入校验 | ⬜ |

## 子任务详细设计

### 1 稳定性：全局错误边界 + 加载状态优化

- **功能目标**：前端崩溃时不白屏，API 请求统一加载态
- **文件结构**（修改）：
  - `packages/rex-console-web/src/App.vue` — 全局 ErrorBoundary
  - `packages/rex-console-web/src/components/` — 新增 LoadingSpinner 组件
- **交互设计**：
  - 组件崩溃显示友好错误页（含重试按钮）
  - API 请求统一骨架屏/loading 指示器
- **提交信息**：`feat(stability): add global error boundary and loading states`

### 2 移动端：响应式布局完善

- **功能目标**：移动端可用性提升，关键页面适配小屏
- **文件结构**（修改）：
  - `packages/rex-console-web/src/layouts/AppLayout.vue` — 移动端底部导航
  - `packages/rex-console-web/src/pages/` — 各页面响应式断点
- **交互设计**：
  - 侧栏在移动端折叠为汉堡菜单
  - 表格在小屏切换为卡片布局
  - 触摸友好的按钮尺寸（≥44px）
- **提交信息**：`feat(mobile): improve responsive layout for small screens`

### 3 安全：CSP 头 + API 输入校验

- **功能目标**：加固 HTTP 安全头，后端 API 入参校验
- **文件结构**（修改）：
  - `crates/rex-hub/src/middleware.rs` — CSP / X-Frame-Options 等安全头
  - `crates/rex-hub/src/` — 各 API handler 增加入参校验
- **提交信息**：`feat(security): add CSP headers and API input validation`

## 设计核对点

- 符合产品文档描述
- 不引入多用户/RBAC 概念
- 前端命令使用 `bun`
- 后端依赖使用 `workspace = true`

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
