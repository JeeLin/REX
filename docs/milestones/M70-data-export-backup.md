# M70: Data Export & Backup

## Context
M69 完成开发体验优化。本里程碑聚焦数据导出和备份能力，提升自托管运维体验。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段做什么：
- 审计日志 CSV/JSON 导出
- 环境配置导出（JSON 格式）
- 数据库备份 API
- 定时备份配置

本阶段不做什么：
- 不修改数据库 schema（已有足够表）
- 不实现跨实例迁移
- 不修改 Agent 端逻辑

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 审计日志导出（CSV/JSON） | ⬜ |
| 2 | 环境配置导出 | ⬜ |
| 3 | 数据库备份 API | ⬜ |
| 4 | 前端导出功能集成 | ⬜ |

## 子任务详细设计

### 1 审计日志导出

- **功能目标**：支持审计日志导出为 CSV 和 JSON 格式
- **文件结构**（修改）：
  - `crates/rex-hub/src/audit_api.rs` — 新增导出端点
- **接口设计**：
  - `GET /api/audit/export?format=csv` — CSV 导出
  - `GET /api/audit/export?format=json` — JSON 导出
  - 支持时间范围筛选
- **提交信息**：`feat(export): add audit log CSV/JSON export`

### 2 环境配置导出

- **功能目标**：导出环境和资源配置为 JSON 文件
- **文件结构**（修改）：
  - `crates/rex-hub/src/env_api.rs` — 新增导出端点
- **接口设计**：
  - `GET /api/environments/export` — 导出所有环境和资源（不含密码）
  - `POST /api/environments/import` — 导入环境配置
- **安全**：导出时排除敏感字段（密码、token）
- **提交信息**：`feat(export): add environment config export/import`

### 3 数据库备份 API

- **功能目标**：提供数据库备份和恢复能力
- **文件结构**（修改）：
  - `crates/rex-hub/src/backup_api.rs` — 新建备份模块
- **接口设计**：
  - `POST /api/backup/create` — 创建备份（返回下载链接）
  - `GET /api/backup/list` — 列出备份
  - `POST /api/backup/restore` — 恢复备份
- **实现**：SQLite VACUUM INTO 创建备份文件
- **提交信息**：`feat(backup): add database backup/restore API`

### 4 前端导出功能集成

- **功能目标**：在审计日志和设置页面添加导出按钮
- **文件结构**（修改）：
  - `packages/rex-console-web/src/pages/AuditLogPage.vue` — 添加导出按钮
  - `packages/rex-console-web/src/pages/SettingsPage.vue` — 添加备份管理
- **交互设计**：
  - 审计日志页：工具栏「导出」按钮，选择 CSV/JSON
  - 设置页：备份管理区域（创建备份、查看备份列表、恢复）
- **提交信息**：`feat(dx): add export buttons to audit log and settings`

## 设计核对点

- 导出不泄露敏感信息（密码、token）
- 备份文件存储在 REX_DATA_DIR/backups/
- 导出操作记录到审计日志

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
