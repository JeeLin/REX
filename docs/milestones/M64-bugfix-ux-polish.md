# M64: Bug 修复与 UX 优化

## Context
M63 完成了性能优化与稳定性提升。本里程碑聚焦修复已知 bug、优化用户体验、完善交互细节。

版本类型：patch（bug 修复/重构）

## 产品边界
本阶段做什么：
- 修复已知 UI/UX bug
- 优化交互细节
- 完善 i18n 翻译

本阶段不做什么：
- 不新增功能模块
- 不引入新的外部依赖

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 右键菜单图标大小统一 | ✅ |
| 2 | 资源创建向导优化 | ✅ |
| 3 | 连接树状态显示优化 | ✅ |
| 4 | 前端组件样式一致性检查 | ✅ |
| 5 | i18n 翻译补全 | ✅ |

## 子任务详细设计

### 1 右键菜单图标大小统一

- **功能目标**：统一右键菜单中图标的大小和间距
- **文件结构**（修改）：
  - `packages/rex-console-web/src/components/ui/ContextMenu.vue` — 图标样式
- **交互设计**：所有图标统一为 16x16px，间距一致
- **测试标准**：视觉检查，所有右键菜单图标大小一致
- **提交信息**：`fix: unify context menu icon sizes`

### 2 资源创建向导优化

- **功能目标**：优化资源创建向导的视觉体验
- **文件结构**（修改）：
  - `packages/rex-console-web/src/features/resource/WizardModal.vue` — 协议图标、颜色选择器
- **交互设计**：
  - 协议图标使用彩色圆形背景
  - 颜色选择器添加选中边框
- **测试标准**：向导界面视觉效果一致
- **提交信息**：`fix: improve resource creation wizard visuals`

### 3 连接树状态显示优化

- **功能目标**：优化连接树中资源项的状态显示
- **文件结构**（修改）：
  - `packages/rex-console-web/src/features/resource-panel/ResourcePanel.vue` — 状态列样式
- **交互设计**：状态列使用彩色圆点表示连接状态
- **测试标准**：连接状态显示清晰
- **提交信息**：`fix: improve connection tree status display`

### 4 前端组件样式一致性检查

- **功能目标**：检查并统一前端组件样式
- **文件结构**（修改）：
  - `packages/rex-console-web/src/styles/` — 全局样式
- **交互设计**：按钮、输入框、卡片等组件样式统一
- **测试标准**：视觉一致性检查通过
- **提交信息**：`fix: unify frontend component styles`

### 5 i18n 翻译补全

- **功能目标**：补全缺失的 i18n 翻译
- **文件结构**（修改）：
  - `packages/rex-console-web/src/i18n/locales/zh.json` — 中文翻译
  - `packages/rex-console-web/src/i18n/locales/en.json` — 英文翻译
- **交互设计**：所有用户可见文本支持中英文
- **测试标准**：中英文切换无遗漏
- **提交信息**：`fix: complete missing i18n translations`

## 设计核对点

- 符合产品文档描述
- 不引入多用户/RBAC 概念
- 后端依赖使用 `workspace = true`
- 前端组件按功能域组织

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
| [x] | 🟡 | 右键菜单图标大小不一致 | 用户反馈 | 右键菜单中不同项目的图标大小不一致，影响视觉体验 |
