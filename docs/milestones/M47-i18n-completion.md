# M47：i18n 全面补全

## 概述

PRODUCT.md §3.14 要求「中/英双语，新增功能必须同步补翻译」。M47 审计并补全所有仍有硬编码英文的组件，确保切换到中文时全站无英文遗漏。

## 范围

### 文件管理模块（4 个组件）
- `FilesPage.vue`：Connect to Server, Protocol, Left/Right, Upload/Download/Refresh/Rename/Delete
- `FileEditorDialog.vue`：Close
- `FolderSyncDialog.vue`：Folder Sync, Direction, Compare By, Include, Exclude 等
- `MobileFilesBar.vue`：Upload, Download, Refresh, Rename, Delete 等

### SQL 控制台模块（7 个组件）
- `SqlPage.vue`：Run All/Current/Selected, SQL Console, No clipboard history
- `ExportWizard.vue`：Export Results, Format, File name, Options 等
- `TableDesigner.vue`：No indexes, Columns, Unique, Type 等
- `GlobalQueryModal.vue`：Global Query, Select All, Clear, Cancel
- `AiAssistantDrawer.vue`：AI Assistant
- `SqlResultGrid.vue`：No results
- `ColumnEditor.vue`：Name, Type

### Redis 模块（1 个组件）
- `RedisStatus.vue`：Server, Version, Memory, Used, Peak, Keys, Expired 等

### 设置模块（1 个组件）
- `SettingsPage.vue`：English（语言选择器选项文本）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 文件管理 i18n（FilesPage / FileEditorDialog / FolderSyncDialog / MobileFilesBar） | ⬜ |
| 2 | SQL 控制台 i18n（SqlPage / ExportWizard / TableDesigner / GlobalQueryModal / AiAssistantDrawer / SqlResultGrid / ColumnEditor） | ⬜ |
| 3 | Redis Status i18n（RedisStatus.vue） | ⬜ |
| 4 | 设置页 i18n（SettingsPage.vue 语言选择器） | ⬜ |
| 5 | locale 文件同步 + 验证（zh.json / en.json 对称、type-check、lint） | ⬜ |

## 依赖

M46（上下文菜单补全）

## 版本类型

minor

## 版本号

0.40.0

## 设计核对点

- 所有用户可见字符串必须使用 `t()` 调用
- locale 文件 zh.json 和 en.json 必须完全对称（相同 key 数、相同 section）
- 不引入多用户/RBAC 相关概念
- 技术术语（MySQL、SSH、Redis 等）保持不翻译
- placeholder 变量格式 `{name}` 在 zh/en 中保持一致

## Flow Status

- [x] 步骤1：编写里程碑文档
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
