# 0.86.0 设计核对报告

## 背景

0.79.0（Redis 增强）和 0.81.0（SQL 增强）已将 PRODUCT.md 中 SQL 控制台和 Redis 管理的大部分功能实现。本次里程碑聚焦两个方向：

1. **文件传输并发控制**（本次主要新增功能）
2. **Redis/SQL 已有功能的体验细节完善**

## 现有实现分析

### 文件传输并发控制
- **后端**：`TransferManager` 使用 `Semaphore` 控制并发，当前硬编码 `with_concurrency(3)`，只有 getter（`max_concurrent()`、`available_permits()`），无 setter
- **前端**：`TransferQueuePanel` 仅显示 `stats.max_concurrent` 统计值，无修改入口
- **设置页面**：`Settings.vue` 包含 Profile/Appearance/Terminal/Security/TLS/Backup/Update 七个区块，**无传输配置区块**
- **设置存储**：`useSettingsStore` 使用 localStorage + 后端同步，结构支持扩展

### 设计决策

#### 并发控制方案
- **后端 API**：新增 `GET /api/transfer/concurrency` 和 `PUT /api/transfer/concurrency`
- **并发动态调整**：`TransferManager` 新增 `set_max_concurrent`，通过重建 Semaphore 或调整许可数实现
- **前端配置**：在设置页面新增 `FilesSection.vue`，包含并发数滑块（1-10）
- **持久化**：并发配置保存到后端 `user_settings` 表（需确认是否有对应字段，否则用独立存储）

#### Redis 体验优化
- 搜索历史：localStorage 存储，独立于现有设置系统
- Key 类型统计：复用 `RedisMonitor` 的 INFO 数据
- 批量导入：新增 API `POST /api/redis/:id/import`

#### SQL 体验优化
- 查询历史筛选：复用现有 `SqlHistoryPanel`，增加筛选 UI
- 结果表格固定列/列宽：修改 `SqlResults.vue` 表格组件
- 快捷键：在 `SqlEditor.vue` 的 CodeMirror keymap 增加

## 风险评估

1. **Semaphore 动态调整**：tokio Semaphore 不支持直接 resize，需要新建 Semaphore 并迁移。解决方案：保存当前 `available_permits` 和 `max_concurrent`，新建 Semaphore 时补偿正在运行的任务
2. **设置字段扩展**：若 `user_settings` 表无 `transfer_concurrency` 字段，需要新增迁移或独立存储

## 结论

设计方案可行。建议优先实现子任务 1（并发控制），子任务 2/3 作为体验完善按需实现。
