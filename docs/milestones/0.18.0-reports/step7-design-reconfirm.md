# 步骤7：设计再确认报告

## 核对对象

- 代码实现：`crates/rex-hub/src/backup.rs`、`packages/rex-console-web/src/api/backup.ts`、`packages/rex-console-web/src/features/settings/BackupSection.vue`
- 里程碑文档：`docs/milestones/0.18.0-配置备份与恢复.md`

## 设计核对点逐项确认

### 1. 备份文件不暴露 Agent token 原始值 ✅

- `BackupEnvironment` 结构体只包含 `agent_token_hash: Option<String>`，不包含原始 token
- 导出 SQL 查询只 SELECT `agent_token_hash` 字段

### 2. 加密使用 PBKDF2 + AES-256-GCM ✅

- `derive_key()` 实现 PBKDF2-SHA256（100000 轮迭代）
- `encrypt_data()` / `decrypt_data()` 使用 AES-256-GCM
- 盐值 16 字节随机生成，nonce 12 字节随机生成

### 3. 导入操作有事务保护，失败时回滚 ✅

- `import_backup()` 使用 `conn.unchecked_transaction()` 创建事务
- 所有 INSERT/UPDATE 操作在事务内执行
- 成功时 `tx.commit()`，失败时事务自动回滚

### 4. 不备份审计日志 ✅

- `BackupData` 只包含 environments、resources、settings，不包含审计日志
- 导出查询不涉及审计日志表

### 5. 导入时保持外键约束（先环境后资源） ✅

- 导入顺序：环境 → 资源 → 设置
- 与数据库外键约束一致

### 6. 备份格式有版本号，支持后续兼容性检查 ✅

- `BackupFile.version` 字段为 `"1.0"`
- `BackupFile.hub_version` 记录导出时的 Hub 版本
- 预览接口返回版本信息供前端展示

### 7. 导出/导入 API 需要认证 ✅

- 三个端点（export、preview、import）都注册在 `protected_routes` 中
- 需要 Bearer token 认证

### 8. 文件大小限制 ✅

- `MAX_BACKUP_SIZE` 常量限制为 50MB
- `check_file_size()` 在 preview 和 import handler 中调用
- 超限返回 `FILE_TOO_LARGE` 错误

## 产品边界确认

### 做什么 ✅

- ✅ 后端：配置导出 API，将环境、资源、设置导出为加密 JSON 文件
- ✅ 后端：配置导入 API，解析备份文件并合并到当前数据库
- ✅ 前端：设置页备份面板 UI（导出按钮、导入上传、导入预览）
- ✅ 支持可选密码加密备份文件
- ✅ 导入时预览差异，支持跳过/覆盖策略
- ✅ 支持选择性导出（按环境筛选）

### 不做什么 ✅

- ✅ 不做自动定时备份
- ✅ 不做远程存储备份
- ✅ 不备份审计日志
- ✅ 不备份 Agent 运行时状态
- ✅ 不做跨版本迁移
- ✅ 不做数据库迁移/升级

## 产品语义确认

- 单用户、自托管：✅ 无多用户/RBAC 概念
- 深色优先：✅ 前端组件使用 CSS 变量，继承主题
- 数据不经过浏览器中转：✅ 导出直接下载 JSON，导入上传文件

## 结论

**✅ 确认通过**

所有设计核对点均通过，产品边界和语义与里程碑文档一致。步骤7 发现的 2 个问题（事务保护、文件大小限制）已在本次确认中修复。
