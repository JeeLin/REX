# Step 5 — 代码审查报告

## 审查范围

本里程碑涉及 22 个文件变更，核心模块：

| 模块 | 文件 | 类型 |
|------|------|------|
| MySQL connector | `crates/rex-mysql/src/connector.rs` | 修改 |
| PostgreSQL connector | `crates/rex-postgresql/src/connector.rs` | 修改 |
| MySQL WS handler | `crates/rex-hub/src/ws_mysql.rs` | 新增 |
| PostgreSQL WS handler | `crates/rex-hub/src/ws_postgresql.rs` | 新增 |
| Agent reset token | `crates/rex-hub/src/agent.rs` | 修改 |
| Routes | `crates/rex-hub/src/routes.rs` | 修改 |
| Settings cert parsing | `crates/rex-hub/src/settings.rs` | 修改 |
| 前端 agent API | `packages/.../api/agent.ts` | 修改 |
| AgentResetTokenModal | `packages/.../AgentResetTokenModal.vue` | 修改 |
| Files.vue 目标选择 | `packages/.../Files.vue` | 修改 |
| WorkspaceSql.vue | `packages/.../WorkspaceSql.vue` | 修改 |
| WorkspaceTerminal.vue | `packages/.../WorkspaceTerminal.vue` | 修改 |

---

## 审查维度

### 1. 正确性

| 项目 | 状态 | 说明 |
|------|------|------|
| MySQL connector 实现 | ✅ | `sqlx::query` + `fetch_all` 正确执行，列/行映射完整 |
| PostgreSQL connector 实现 | ✅ | 同上，`information_schema` 查询正确使用参数化绑定 |
| WebSocket 消息协议 | ✅ | 与 SQLite/Redis handler 一致，序列化/反序列化测试覆盖 |
| reset_token 逻辑 | ✅ | 正确通过 agent_id → environment_id → UPDATE token_hash |
| cert 过期解析 | ✅ | 手动 DER 解析 UTCTime/GeneralizedTime，逻辑正确 |
| 前端 Modal 调用 | ✅ | `resetAgentToken(agent.id)` 正确传参，emit success/close |
| Files 目标选择 | ✅ | 单目标直接发送，多目标弹出对话框，逻辑完整 |

### 2. 安全性

| 项目 | 状态 | 说明 |
|------|------|------|
| SQL 注入防护（MySQL） | ✅ | `list_tables`/`list_columns` 使用反引号转义 `replace('`', "``")` |
| SQL 注入防护（PostgreSQL） | ✅ | 使用 `$1` 参数化绑定 |
| 密码日志泄露 | ✅ | `info!` 宏仅记录 host/port/user，不记录 password |
| WebSocket 鉴权 | ✅ | 通过 query token 验证 JWT，与现有 handler 一致 |
| reset_token 权限 | ✅ | 受 auth_middleware 保护，需 Bearer token |

### 3. 架构一致性

| 项目 | 状态 | 说明 |
|------|------|------|
| WS handler 模式 | ✅ | ws_postgresql.rs 完全复用 ws_mysql.rs 的架构 |
| SqlConnector trait 实现 | ✅ | 与 SQLite connector 接口一致 |
| 路由注册 | ✅ | 遵循现有 `ws/:type/:id` 命名规范 |
| 依赖管理 | ✅ | 所有新依赖通过 `workspace = true` 引入 |

### 4. 错误处理

| 项目 | 状态 | 说明 |
|------|------|------|
| 连接失败 | ✅ | 返回 WebSocket Error 消息，前端可展示 |
| 配置解析失败 | ✅ | `from_json` 失败返回错误消息并关闭 WS |
| 未连接状态调用 | ✅ | `execute`/`list_*` 检查 `pool.is_some()` |
| reset_token 404 | ✅ | agent 不存在返回 `AGENT_NOT_FOUND` |

### 5. 测试覆盖

| 项目 | 状态 | 说明 |
|------|------|------|
| MySQL/PG connector 单元测试 | ✅ | 配置反序列化、from_json、未连接状态错误 |
| WS 消息协议测试 | ✅ | 序列化/反序列化覆盖 Command/Ping/Response/Error/Connected/Pong |
| reset_token HTTP 测试 | ⚠️ | 缺少 handler 级别测试（🟢 可选改进） |

---

## 发现汇总

| 级别 | 编号 | 文件 | 问题 |
|------|------|------|------|
| 🟡 | 1 | ws_mysql.rs / ws_postgresql.rs | 配置解析冗余：`from_json()` 解析一次后立即丢弃，又手动解析 JSON。功能正确但有冗余 |
| 🟡 | 2 | agent.rs | `reset_token` 缺少 HTTP handler 测试 |
| 🟢 | 3 | connector.rs (mysql/pg) | `try_get_json_value` 类型尝试顺序中 i32/i64 在 PostgreSQL 中可能冲突，但实际影响极小 |
| 🟢 | 4 | Dockerfile.hub | 基础镜像从 debian:bookworm-slim 改为 ubuntu:24.04，属于 CI 修复，不在本次里程碑范围内 |

## 结论

**无 🔴 必须修复项。** 所有 🟡 项为改进建议，不影响功能正确性和安全性。

🟢 #4（Dockerfile 变更）不属于本里程碑范围，步骤8提交时不应包含此文件。
