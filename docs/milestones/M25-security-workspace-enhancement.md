# M25: 安全加固 + 工作区增强（SQL 注入修复 + 全局搜索）

## Context

M0–M24 完成了从项目骨架到 SFTP 文件管理增强的全部开发。代码审查发现 SQL 注入风险（MySQL/PostgreSQL connector 使用 `format!()` 拼接标识符）、缺少查询超时、连接池无清理等问题。本里程碑修复安全问题并增强工作区功能。

本里程碑版本类型：patch（安全修复 + 增强），版本号 0.25.0 → 0.25.1。

## 产品边界

**本阶段做：**
- SQL 注入修复：MySQL/PostgreSQL connector 标识符转义
- 查询超时和行数限制：防止长时间查询阻塞
- 连接池清理：定期清理空闲连接
- 全局搜索（Ctrl+K）：命令面板功能

**本阶段不做：**
- 移除 Quick Connect（需要产品决策，后续里程碑）
- 高级 FormatViewer（Msgpack/PHPSerialize 等）
- Stream 消费组表格

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | SQL 注入修复（标识符转义） | ✅ |
| 2 | 查询超时和行数限制 | ✅ |
| 3 | 全局搜索（Ctrl+K 命令面板） | ✅ |

## 子任务详细设计

### 1 SQL 注入修复（标识符转义）

**功能目标**

修复 MySQL/PostgreSQL connector 中的 SQL 注入风险，对标识符进行正确转义。

**文件结构**

修改：
- `crates/rex-mysql/src/lib.rs` — 标识符使用反引号包裹
- `crates/rex-postgresql/src/lib.rs` — 标识符使用双引号包裹

**实现流程**

1. MySQL: 将 `'{db}'` 改为 `` `{db}` ``，并转义内部反引号
2. PostgreSQL: 将 `'{db}'` 改为 `"{db}"`，并转义内部双引号
3. 添加辅助函数 `escape_identifier`

**测试标准**

- 包含特殊字符的表名/库名可正常查询
- cargo clippy 无 warnings

**提交信息**: `fix(sql): add identifier escaping to prevent SQL injection`

### 2 查询超时和行数限制

**功能目标**

防止长时间查询阻塞连接池，限制返回行数。

**文件结构**

修改：
- `crates/rex-common/src/sql.rs` — 添加查询超时配置
- `crates/rex-hub/src/sql_api.rs` — 应用超时和行数限制

**实现流程**

1. 添加查询超时（默认 30 秒）
2. 添加最大返回行数限制（默认 10000 行）
3. 超时或超限时返回错误信息

**测试标准**

- 长时间查询超时中断
- 大结果集被截断
- cargo clippy 无 warnings

**提交信息**: `fix(sql): add query timeout and row limit`

### 3 全局搜索（Ctrl+K 命令面板）

**功能目标**

提供全局搜索功能，快速访问资源、功能和设置。

**文件结构**

新建：
- `packages/rex-console-web/src/features/workspace/CommandPalette.vue` — 命令面板组件

修改：
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — 集成命令面板

**交互设计**

```
┌─ Command Palette ────────────────────────────────────┐
│ 🔍 Search resources, commands, settings...           │
│                                                      │
│ 📡 Resources                                         │
│   Redis Local (127.0.0.1:6379)                       │
│   PostgreSQL Production (10.0.0.1:5432)              │
│                                                      │
│ ⚡ Commands                                          │
│   New Connection (Ctrl+N)                            │
│   New Tab (Ctrl+T)                                   │
│   Settings                                           │
│                                                      │
│ ⚙️ Settings                                          │
│   Theme: Dark                                        │
│   Language: English                                  │
└──────────────────────────────────────────────────────┘
```

**实现流程**

1. 创建命令面板组件
2. 监听 Ctrl+K 快捷键
3. 搜索资源、命令、设置
4. 选择后执行对应操作

**测试标准**

- Ctrl+K 打开命令面板
- 搜索资源并打开
- 搜索命令并执行
- type-check + build 通过

**提交信息**: `feat(workspace): add command palette with Ctrl+K`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ 架构一致（前端组件 + 后端 API）
- ✅ 不引入多用户/RBAC 概念
- ✅ 不跳阶段实现
- ✅ 实现细节不污染产品文档

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

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
