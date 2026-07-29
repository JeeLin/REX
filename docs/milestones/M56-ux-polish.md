# M56: UX polish & stability

## Context
M55 完成了 Agent 注册流程。本里程碑继续打磨用户体验，完善 Agent 管理页面、补全 i18n、修复已知问题。

版本类型：patch（bug 修复 + UX 打磨）

## 产品边界
本阶段打磨现有功能的 UX，不涉及新功能模块。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Agent 管理页面：部署指南显示注册令牌、配置弹窗改进 | ⬜ |
| 2 | 前端 i18n 补全：新增键的上下文完善 | ⬜ |
| 3 | 已知 bug 修复与 UX 打磨 | ⬜ |

## 子任务详细设计

### 1 Agent 管理页面改进

- **功能目标**：部署指南中显示实际注册令牌（而非占位符），配置弹窗信息更完整
- **文件结构**（修改）：
  - `packages/rex-console-web/src/pages/AgentsPage.vue` — 部署指南和配置弹窗
  - `packages/rex-console-web/src/pages/EnvironmentDetailPage.vue` — Agent 面板信息
- **交互设计**：
  - 部署指南中 `YOUR_REGISTRATION_TOKEN` 替换为实际令牌
  - 配置弹窗显示环境名、Agent ID、版本、状态等完整信息
- **提交信息**：`feat(agents): show registration token in deploy guide`

### 2 前端 i18n 补全

- **功能目标**：补全新增和缺失的 i18n 键，确保中英文一致
- **文件结构**（修改）：
  - `packages/rex-console-web/src/i18n/locales/en.json`
  - `packages/rex-console-web/src/i18n/locales/zh.json`
- **提交信息**：`fix(i18n): complete missing translation keys`

### 3 已知 bug 修复与 UX 打磨

- **功能目标**：修复测试中发现的问题
- **提交信息**：`fix: ux polish and bug fixes`

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
