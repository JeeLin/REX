# M44: 后端操作日志补全

## Context

M43 完成前端交互修复和 WebSocket 鉴权。M36 建立了基础日志框架（请求中间件 + SSH/SQL/File/Auth 审计日志），M43 增强了 SSH 终端日志。但 Redis API、环境/资源 CRUD、文件传输大部分操作、Settings、Agent 管理等模块仍无任何结构化日志，排障和审计盲区较多。本里程碑将后端日志覆盖补齐到所有关键操作。

版本类型：minor（新增审计日志条目和 tracing 覆盖），版本号 0.38.2 → 0.39.0。

## 产品边界

**本阶段做：**
- Redis 操作日志（connect / disconnect / scan / key CRUD / TTL / command）
- 环境/资源 CRUD 审计日志（create / update / delete / test_connection）
- 文件传输操作日志补全（connect / disconnect / list / rename / mkdir / download / read_for_edit / save_from_edit / ACL）
- Settings 变更日志 + Agent token 重置日志
- Agent 隧道数据量与持续时间统计日志
- 日志级别规范化审查（敏感信息不入日志）

**本阶段不做：**
- 日志持久化到外部系统（ELK、Loki、Grafana）
- 实时日志流 UI
- 日志级别运行时动态调整
- 前端日志查看器增强
- 自动更新模块（update_api.rs）日志

## 子任务清单

| # | 内容 | 预计文件 | 状态 |
|---|------|----------|------|
| 1 | Redis 操作日志 | `redis_api.rs` | ⬜ |
| 2 | 环境/资源 CRUD 审计日志 | `env_api.rs`, `resource_api.rs` | ⬜ |
| 3 | 文件传输操作日志补全 | `file_api.rs` | ⬜ |
| 4 | Settings 变更 + Agent token 日志 | `settings_api.rs`, `agent_api.rs` | ⬜ |
| 5 | Agent 隧道统计日志 | `tunnel_ws.rs`, `agent_ws.rs` | ⬜ |
| 6 | 日志级别规范化 + 敏感信息审查 | 全局 | ⬜ |

## 子任务详细设计

### 1 Redis 操作日志

**目标**

Redis 控制台的所有操作都有结构化日志，与 SQL/SSH 日志格式对齐。

**修改文件**

`crates/rex-hub/src/redis_api.rs`

**审计点**

| 操作 | action | 关键字段 | 日志级别 |
|------|--------|----------|----------|
| Redis 连接 | `REDIS_CONNECT` | host, port, db | info (成功) / warn (失败) |
| Redis 断开 | `REDIS_DISCONNECT` | session_id | info |
| DB 切换 | `REDIS_SELECT` | session_id, db | info |
| Key 查询 | — | session_id, pattern, count | debug |
| Key 读取 | — | session_id, key | debug |
| Key 写入 | — | session_id, key, type | info |
| Key 删除 | `REDIS_DEL` | session_id, keys (count) | info |
| TTL 设置 | — | session_id, key, ttl | info |
| 命令执行 | `REDIS_COMMAND` | session_id, command (不含密码) | info |
> **说明**：本子任务为 Redis 操作补充 tracing 日志。对于关键操作（connect/disconnect/select_db/command），同时补充 write_audit_log 以在前端审计日志页面展示。

**敏感信息处理**
- 连接密码不写入日志（仅 `has_password: bool`）
- SCAN pattern 可写入日志（调试有价值）
- COMMAND 命令中 AUTH/密码参数脱敏

**实现要点**

```rust
// connect handler 示例
async fn connect(State(state): State<AppState>, Json(body): Json<ConnectBody>) -> Response {
    tracing::info!(
        action = "REDIS_CONNECT",
        host = %body.host,
        port = body.port,
        db = body.db,
        has_password = body.password.is_some(),
        "Redis connecting"
    );
    match RedisConnectorImpl::connect(req).await {
        Ok(conn) => {
            // ...insert session...
            tracing::info!(action = "REDIS_CONNECT", session_id = %session_id, "Redis connected");
            // 审计日志写入
        }
        Err(e) => {
            tracing::warn!(action = "REDIS_CONNECT", host = %body.host, error = %e, "Redis connect failed");
        }
    }
}
```

**提交信息**: `feat(hub): add structured logging for Redis operations`

### 2 环境/资源 CRUD 审计日志

**目标**

所有配置变更操作写入审计日志，可在审计日志页面按环境/资源筛选查看。

**修改文件**

- `crates/rex-hub/src/env_api.rs`
- `crates/rex-hub/src/resource_api.rs`

**审计点**

| 操作 | action | 关键字段 | 现有日志 | 新增日志 |
|------|--------|----------|----------|----------|
| 创建环境 | `ENV_CREATE` | env_id, name, connection_mode | write_audit_log | tracing |
| 更新环境 | `ENV_UPDATE` | env_id, name, changed_fields | write_audit_log | tracing |
| 删除环境 | `ENV_DELETE` | env_id, name, resource_count | write_audit_log | tracing |
| 导入环境 | `ENV_IMPORT` | count, imported_count | 无 | tracing + write_audit_log |
| 创建资源 | `RESOURCE_CREATE` | resource_id, env_id, protocol, name | write_audit_log | tracing |
| 更新资源 | `RESOURCE_UPDATE` | resource_id, env_id, protocol, name | 无 | tracing + write_audit_log |
| 删除资源 | `RESOURCE_DELETE` | resource_id, env_id, protocol, name | write_audit_log | tracing |
| 测试连接 | `TEST_CONNECTION` | protocol, host, result | 无 | tracing |

> **说明**：M36 建立了双层日志架构：
> - `tracing::info!()` → 结构化日志（stdout/journald/日志文件）
> - `write_audit_log()` → 审计日志表（前端审计日志页面展示）
> 本子任务为以上所有操作补充 tracing 日志，且对缺失的操作补充 write_audit_log。

**实现要点**

```rust
// create_resource handler
tracing::info!(
    action = "RESOURCE_CREATE",
    resource_id = %resource.id,
    env_id = %env_id,
    protocol = %body.protocol,
    name = %body.name,
    "resource created"
);
```

**提交信息**: `feat(hub): add audit logging for environment and resource CRUD`

### 3 文件传输操作日志补全

**目标**

补齐 file_api.rs 中缺失的日志覆盖。当前仅 upload 和 delete 有日志，connect / disconnect / list / rename / mkdir / download / read_for_edit / save_from_edit / presigned_url / ACL 操作均无日志。

**修改文件**

`crates/rex-hub/src/file_api.rs`

**审计点**

| 操作 | action | 关键字段 | 日志级别 |
|------|--------|----------|----------|
| 文件连接 | `FILE_CONNECT` | session_id, protocol (sftp/s3) | info |
| 文件断开 | `FILE_DISCONNECT` | session_id | info |
| 文件列表 | — | session_id, path | debug |
| 文件重命名 | `FILE_RENAME` | session_id, from, to | info |
| 创建文件夹 | `FILE_MKDIR` | session_id, path | info |
| 文件下载 | `FILE_OP` (op=download) | session_id, path, size | info |
| 文件编辑保存 | `FILE_OP` (op=save_edit) | session_id, path | info |
| ACL 操作 | `FILE_ACL` | session_id, path, action (get/put) | info |
| Presigned URL | — | session_id, path, expires | debug |

**已有的保持不变**（upload / delete 已有日志）。
> **说明**：本子任务为所有文件操作补充 tracing 日志。对于关键操作（connect/disconnect/rename/mkdir/download/save_edit/ACL），同时补充 write_audit_log 以在前端审计日志页面展示。

**提交信息**: `feat(hub): complete logging for file transfer operations`

### 4 Settings 变更 + Agent token 日志

**目标**

记录设置变更（可追溯谁在何时改了什么）和 Agent token 重置（安全审计）。

**修改文件**

- `crates/rex-hub/src/settings_api.rs`
- `crates/rex-hub/src/agent_api.rs`

**审计点**

| 操作 | action | 关键字段 |
|------|--------|----------|
| 设置更新 | `SETTINGS_UPDATE` | changed_keys (如 "theme", "language") |
| Agent token 重置 | `AGENT_TOKEN_RESET` | agent_id |
> **说明**：本子任务为设置变更和 Agent token 重置操作同时补充 tracing 日志和 write_audit_log，以支持审计日志页面展示。

**实现要点**

```rust
// update_settings handler
tracing::info!(
    action = "SETTINGS_UPDATE",
    keys = ?changed_keys, // ["theme", "terminal_font"]
    "settings updated"
);
```

**提交信息**: `feat(hub): add audit logging for settings and agent token operations`

### 5 Agent 隧道统计日志

**目标**

增强 tunnel_ws.rs 和 agent_ws.rs 的日志，记录隧道数据量、持续时间、错误详情。

**修改文件**

- `crates/rex-hub/src/tunnel_ws.rs`
- `crates/rex-hub/src/agent_ws.rs`

**新增日志点**

| 位置 | 日志内容 | 级别 |
|------|----------|------|
| tunnel_ws 数据转发循环 | 每条 WebSocket 消息的 bytes 转发量（聚合后定期输出，如每 1000 条或连接关闭时） | debug |
| tunnel_ws 连接关闭 | total_bytes_forwarded, duration_ms, error_count | info |
| tunnel_ws 转发错误 | channel_id, error, direction (hub→agent / agent→hub) | warn |
| agent_ws 数据转发 | channel_id, bytes_forwarded, direction | debug |
> **说明**：本子任务为隧道统计补充 tracing 日志。隧道统计不写入审计日志表（write_audit_log），仅通过 tracing 输出到日志文件，用于运维监控和故障排查。

**实现要点**

```rust
// tunnel_ws 连接关闭时
tracing::info!(
    action = "TUNNEL_CLOSE",
    channel_id = %channel_id,
    agent_id = %agent_id,
    protocol = %protocol,
    duration_ms = start.elapsed().as_millis() as u64,
    bytes_forwarded = total_bytes,
    error_count = errors,
    "tunnel closed"
);
```

**提交信息**: `feat(hub): add tunnel duration and data volume logging`

### 6 日志级别规范化 + 敏感信息审查

**目标**

全局审查所有 tracing 调用，确保：
1. 日志级别一致性：info = 业务操作成功/失败，warn = 可恢复异常，error = 不可恢复失败，debug = 详细诊断信息
2. 敏感信息（密码、token、private_key 内容、SQL 查询明文中的数据值）不写入日志
3. 错误日志包含足够上下文（resource_id / session_id / agent_id）便于排障
4. 所有 audit log 写入点使用一致的 action 字段命名约定

**修改文件**

全局扫描所有 `tracing::` 调用点

**检查清单**

- [ ] SQL 查询日志不包含查询返回的数据值
- [ ] Redis 命令日志中 AUTH 密码脱敏
- [ ] 文件上传日志不包含文件内容
- [ ] SSH 连接日志不包含密码/私钥内容
- [ ] 错误路径都有 resource_id/session_id 上下文
- [ ] action 字段命名统一（`大写前缀_操作`）

**提交信息**: `fix(hub): standardize log levels and sanitize sensitive data`

## 设计核对点

- ✅ 不引入外部日志系统依赖（ELK / Loki / Grafana）
- ✅ 使用 tracing 宏（已有依赖）
- ✅ 日志级别遵循 info → warn → error → debug 约定
- ✅ 敏感信息（密码、token、私钥内容）不写入日志
- ✅ 审计日志 action 字段使用 `大写前缀_操作` 命名约定
- ✅ 错误日志包含足够上下文字段便于排障
- ✅ 无前端改动（纯后端可观测性改善）

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
