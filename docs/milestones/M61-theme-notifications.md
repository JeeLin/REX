# M61: Theme Optimization & Notification System

## Context
M60 完成了国际化、数据导出和搜索增强。本里程碑从暗色主题优化和通知系统两个维度进一步提升用户体验。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段优化现有功能，不涉及新功能模块。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 暗色主题：对比度优化 + 高对比模式 | ⬜ |
| 2 | 通知系统：操作通知 + 错误通知 + 连接状态通知 | ⬜ |

## 子任务详细设计

### 1 暗色主题：对比度优化 + 高对比模式

- **功能目标**：优化暗色主题对比度，新增高对比模式选项
- **文件结构**（修改）：
  - `packages/rex-console-web/src/styles/` — CSS 变量优化
  - `packages/rex-console-web/src/pages/SettingsPage.vue` — 新增高对比模式开关
- **交互设计**：
  - 暗色主题文字对比度提升至 4.5:1 以上
  - 新增高对比模式：更粗的边框、更大的字体、更亮的文字
- **提交信息**：`feat(theme): improve dark mode contrast and add high contrast mode`

### 2 通知系统：操作通知 + 错误通知 + 连接状态通知

- **功能目标**：统一通知组件，支持成功/错误/连接状态通知
- **文件结构**（修改）：
  - `packages/rex-console-web/src/components/NotificationToast.vue` — 新增通知组件
  - `packages/rex-console-web/src/stores/notification.ts` — 通知状态管理
  - `packages/rex-console-web/src/App.vue` — 集成通知组件
- **交互设计**：
  - 操作通知：成功/错误/警告/信息，自动消失
  - 连接状态：Agent 连接/断开通知
  - 支持手动关闭和批量清除
- **提交信息**：`feat(notifications): add unified notification system`

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
