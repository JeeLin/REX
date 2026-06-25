# Step 4: 代码精简报告

## 检查范围

`global_query` 功能新增/修改的文件：
- `crates/rex-hub/src/sql.rs` — 后端核心逻辑
- `crates/rex-hub/src/bin/rex-hub.rs` — dotenvy 加载
- `crates/rex-hub/Cargo.toml` — 新增依赖
- `crates/rex-common/Cargo.toml` — 依赖版本
- `Cargo.toml` / `Cargo.lock` — workspace 依赖
- `packages/rex-console-web/src/components/GlobalQueryModal.vue` — 前端弹窗
- `packages/rex-console-web/src/composables/useGlobalQuery.ts` — 前端逻辑
- `packages/rex-console-web/src/pages/SqlConsole.vue` — 入口集成

## 精简项

### 1. MySQL/PostgreSQL 分支重复代码提取 ✅ 已修复

**问题**：`global_query` 中 MySQL 和 PostgreSQL 的连接、执行、超时、关闭逻辑几乎完全相同（约 200 行重复）。

**修复**：
- 提取 `create_connector()` — 根据 db_type 创建对应的连接器
- 提取 `execute_single_db()` — 统一处理连接→执行→关闭→超时逻辑
- 提取 `send_error()` / `send_event()` — 统一 SSE 事件发送
- 提取 `send_query_result()` — 统一结果格式化和发送

**效果**：`sql.rs` 从约 800 行减少到 654 行，消除约 150 行重复代码。

### 2. 死代码变量清理 ✅ 已修复

**问题**：MySQL/PostgreSQL 分支内计算了 `row_count`、`limited_rows` 但未使用（分支内直接返回整个 result）。

**修复**：删除分支内的死代码，将结果处理集中到 `send_query_result()` 中。

### 3. 未使用导入清理 ✅ 已修复

- 移除 `futures_util::stream::StreamExt`（未使用）
- 移除 `tracing::error`（未使用，Infallible 错误分支已删除）

### 4. `GlobalQueryEvent::Result` 的 columns 类型修正 ✅ 已修复

**问题**：原代码中 `GlobalQueryEvent::Result` 的 `columns` 字段是 `Vec<String>`，但 `SqlResult.columns` 是 `Vec<SqlColumn>`。

**修复**：在 `send_query_result()` 中通过 `.iter().map(|c| c.name.clone())` 正确转换。

### 5. MutexGuard 跨 await 问题 ✅ 已修复

**问题**：`results.lock().unwrap()` 返回的 `MutexGuard` 在 `tokio::spawn` 产生的 future 中被跨 await 使用，导致 `Send` trait 不满足。

**修复**：先 clone 出 `Vec<QueryResult>`，释放锁后再遍历发送。

### 6. `dotenvy` 依赖修复 ✅ 已修复

**问题**：`dotenvy` 被添加到 `Cargo.toml` 但 workspace 定义中 `features = ["alloc"]` 不存在，且 `crates/rex-hub/Cargo.toml` 缺少依赖声明。

**修复**：
- `Cargo.toml`：移除无效 feature → `dotenvy = "0.15"`
- `crates/rex-hub/Cargo.toml`：添加 `dotenvy = { workspace = true }`

### 7. 简化 dialect 验证逻辑 ✅ 已修复

**问题**：原代码用 `match db_type.as_str() { "mysql" => "mysql", "postgresql" => "postgresql", _ => ... }` 重新映射字符串，实际 `db_type` 本身就是目标值。

**修复**：简化为 `match db_type.as_str() { "mysql" | "postgresql" => db_type.clone(), _ => ... }`。

## 编译验证

```
cargo check -p rex-hub → ✅ 通过（仅 warnings，无 error）
```

## 结论

精简不改变功能行为，仅优化代码组织方式。所有精简均符合项目现有风格。
