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

## 子任务

1. **文件管理 i18n**：为 files.* section 添加缺失 key，更新 4 个组件使用 t()
2. **SQL 控制台 i18n**：扩展 sql.* section，更新 7 个组件使用 t()
3. **Redis Status i18n**：扩展 redis.* section（Status 相关 key），更新 RedisStatus.vue
4. **设置页 i18n**：更新 SettingsPage.vue 语言选择器文本
5. **locale 文件同步**：确保 zh.json 和 en.json 完全对称
6. **验证与收尾**：type-check + lint + 手动切换中英文验证

## 依赖

M46（上下文菜单补全）

## 版本类型

minor

## 版本号

0.40.0
