# M58: Performance, Accessibility & Documentation

## Context
M57 完成了稳定性、移动端和安全加固。本里程碑从性能优化、无障碍支持、文档完善三个维度进一步提升产品质量。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段提升现有功能的质量，不涉及新功能模块。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 性能：路由懒加载 + 代码分割 | ⬜ |
| 2 | 无障碍：ARIA 标签 + 键盘导航 | ⬜ |
| 3 | 文档：用户文档 + API 文档 + 部署指南 | ⬜ |

## 子任务详细设计

### 1 性能：路由懒加载 + 代码分割

- **功能目标**：减少首屏加载时间，按需加载页面组件
- **文件结构**（修改）：
  - `packages/rex-console-web/src/router/index.ts` — 路由懒加载
  - `packages/rex-console-web/vite.config.ts` — chunk 策略优化
- **交互设计**：无 UI 变化，仅加载性能提升
- **提交信息**：`perf: add route lazy loading and code splitting`

### 2 无障碍：ARIA 标签 + 键盘导航

- **功能目标**：提升残障用户的可访问性
- **文件结构**（修改）：
  - `packages/rex-console-web/src/components/` — 关键组件添加 ARIA
  - `packages/rex-console-web/src/layouts/AppLayout.vue` — 键盘导航支持
- **交互设计**：
  - 按钮和链接添加 aria-label
  - Tab 键在可交互元素间导航
  - 模态框 focus trap
- **提交信息**：`a11y: add ARIA labels and keyboard navigation`

### 3 文档：用户文档 + API 文档 + 部署指南

- **功能目标**：完善用户和开发者文档
- **文件结构**（创建）：
  - `docs/user-guide/` — 用户使用指南
  - `docs/api/` — REST API 文档
  - `docs/deployment/` — 部署指南
- **提交信息**：`docs: add user guide, API docs and deployment guide`

## 设计核对点

- 符合产品文档描述
- 不引入多用户/RBAC 概念
- 前端命令使用 `bun`

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
