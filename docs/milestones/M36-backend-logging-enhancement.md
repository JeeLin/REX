# M36: 后端日志增强

## Context

M35 完成了工作区 Xshell 体验补全（快捷键面板、编码子菜单、状态栏、路由修复）。当前后端日志过于简略，缺少关键操作的审计信息，排障困难。本里程碑增强后端日志覆盖，提升可观测性。

版本类型：patch（无新功能，纯改善），版本号 0.34.0 → 0.34.1。

## 产品边界

**本阶段做：**
- API 请求日志中间件（method、path、status、latency）
- 关键操作审计日志增强（SSH 连接、SQL 查询、文件操作、认证事件）
- 错误日志上下文补充（resource_id、env_id、user_agent）

**本阶段不做：**
- 日志持久化到外部系统（ELK、Loki）
- 实时日志流 UI
- 日志级别动态调整

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | API 请求日志中间件 | ⬜ |
| 2 | 关键操作审计日志增强 | ⬜ |

## 子任务详细设计

### 1 API 请求日志中间件

**功能目标**

为所有 HTTP 请求添加结构化日志，包含 method、path、status code、latency。

**文件结构**

修改：
- `crates/rex-hub/src/rex-hub.rs` — 添加 tracing middleware layer
- `crates/rex-hub/src/middleware.rs` — 新增请求日志中间件

**实现方案**

使用 `tower_http::trace::TraceLayer` 或自定义 `axum::middleware`：

```rust
// 方案：自定义 axum middleware
async fn request_logger(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();
    let response = next.run(req).await;
    let latency = start.elapsed();
    tracing::info!(
        method = %method,
        path = %uri,
        status = response.status().as_u16(),
        latency_ms = latency.as_millis() as u64,
        "request"
    );
    response
}
```

添加到 `build_router` 的 `protected_routes` 层：
```rust
.layer(axum::middleware::from_fn(request_logger))
```

**测试标准**

- 启动后访问任意 API，日志输出包含 method、path、status、latency
- 静态文件请求不产生日志（或标记为 static）

**提交信息**: `feat(hub): add request logging middleware`

### 2 关键操作审计日志增强

**功能目标**

确保 SSH 连接、SQL 查询、文件操作、认证事件等关键操作都有结构化审计日志。

**文件结构**

修改：
- `crates/rex-hub/src/terminal_ws.rs` — SSH 连接审计
- `crates/rex-hub/src/sql_api.rs` — SQL 查询审计
- `crates/rex-hub/src/file_api.rs` — 文件操作审计
- `crates/rex-hub/src/auth.rs` — 认证事件审计

**审计日志格式**

```rust
tracing::info!(
    action = "SSH_CONNECT",
    resource_id = %resource_id,
    host = %host,
    port = port,
    user = %username,
    "SSH connection established"
);
```

**审计点**

| 操作 | action | 关键字段 |
|------|--------|----------|
| SSH 连接 | `SSH_CONNECT` | resource_id, host, port |
| SSH 断开 | `SSH_DISCONNECT` | resource_id, reason |
| SQL 连接 | `SQL_CONNECT` | resource_id, db_type, host |
| SQL 查询 | `SQL_QUERY` | resource_id, query_length, duration_ms |
| 文件操作 | `FILE_OP` | resource_id, op (upload/download/delete), path |
| 认证成功 | `AUTH_LOGIN` | ip |
| 认证失败 | `AUTH_LOGIN_FAIL` | ip, reason |

**测试标准**

- SSH 连接/断开产生审计日志
- SQL 查询产生审计日志
- 文件上传/下载产生审计日志
- 认证事件产生审计日志

**提交信息**: `feat(hub): enhance audit logging for key operations`

## 设计核对点

- ✅ 不引入外部日志系统依赖
- ✅ 使用 tracing 宏（已有依赖）
- ✅ 日志级别：info 用于请求和操作，error 用于失败
- ✅ 敏感信息（密码、token）不写入日志

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

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
