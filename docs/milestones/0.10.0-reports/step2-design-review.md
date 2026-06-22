# 步骤 2：设计核对报告（0.10.0 SQLite 协议支持）

## 审查框架维度

### 1. 产品定位一致性 ✅

- SQLite 定位为"本地数据库"（PRODUCT.md 第 94 行），里程碑文档的"本地 SQLite 数据库"描述一致
- 单用户、自托管：✅ 未引入多用户/RBAC
- 文件传输不经过浏览器：✅ SQLite 无文件传输需求

### 2. 功能边界 ✅

| PRODUCT.md 要求 | 里程碑覆盖 | 状态 |
|-----------------|-----------|------|
| SQLite 协议支持 | rex-sqlite crate + WebSocket + 前端 | ✅ |
| 本地数据库 | db_path 文件路径连接模型 | ✅ |
| SQL 控制台 | 复用 features/sql/ | ✅ |

### 3. 架构一致性 ✅

- stub 模式：与 rex-redis、rex-docker 一致（trait 定义 + stub 实现）
- WebSocket 消息协议：遵循 command/response/error/connected/disconnected 模式
- 依赖规则：使用 `workspace = true`，`rusqlite` 已在 workspace 中
- 路由模式：`/ws/sqlite/:resource_id`，与 `/ws/redis/:resource_id`、`/ws/docker/:resource_id` 一致

### 4. 前端复用合理性 ✅

- `useTabs.ts` 已有 `sqlite: 'sql'` 映射（之前里程碑已添加）
- SQL 控制台组件（SqlEditor、SqlResults、SqlSidebar、SqlTopbar）可复用
- SQLite 特有适配点明确：单数据库（无多库切换）、显示文件路径

### 5. 子任务拆分 ✅

3 个子任务粒度合理：
1. 后端 crate（独立提交）
2. 后端 WebSocket handler（独立提交）
3. 前端适配 + 表单 + 面板（独立提交）

### 6. 接口设计 ✅

- `SqliteConfig`：`db_path` + `name`，简洁明确
- `SqliteConnector` trait：5 个方法覆盖基本操作
- WebSocket actions：`execute`/`tables`/`columns`，覆盖 SQL 控制台核心需求
- 前端表单：`db_path` + `name`，字段清晰

### 7. 已实现部分确认

- `useTabs.ts` 已有 `sqlite: 'sql'` 映射 ✅
- 无需修改 `useTabs.ts`（已在之前里程碑完成）
- 子任务 3 中"修改 useTabs.ts"应移除（已实现）

## 发现

### 🟡 应该修复

| # | 问题 | 说明 |
|---|------|------|
| 1 | 子任务 3 文件结构中列出了 `useTabs.ts` 修改，但该文件已有 `sqlite: 'sql'` 映射 | 应从文件列表中移除，避免混淆 |

## 结论

✅ **设计核对通过。** 仅 1 个小问题（子任务 3 文件列表多列了 useTabs.ts），直接修正。
