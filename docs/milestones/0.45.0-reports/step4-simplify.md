# 0.45.0 Step 4: 代码精简

## 检查日期: 2026-07-03

## 检查维度

### 1. 重复代码
- **connector.rs**: `SqlConnector` impl 的 `list_tables` 委托给 `SqliteConnector::list_tables`，但 `list_databases`、`list_views`、`explain` 直接访问 `self.state`。不一致但无重复逻辑，可接受。
- **sql.rs**: `get_sql_connector()` 的 3 个 match arm（mysql/postgresql/sqlite）结构一致，无重复可消除。

### 2. 过度设计
- 无过度设计。所有改动仅覆盖里程碑文档定义的功能。

### 3. 提前实现
- 无提前实现下一阶段能力。

### 4. 功能域结构
- Rust 改动在正确的 crate 中。
- 前端改动在 `features/sql/` 功能域内。

### 5. 可精简项
- **SqlSidebar.vue**: `loadViews()` 和 `loadProcedures()` 在 SQLite 协议下仍会调用 API（虽然后端返回空列表）。可以在函数开头加 `if (isSqlite.value) return` 跳过无意义的 API 调用。
- **connector.rs**: 旧 `SqliteConnector` trait 仅在 `connector.rs` 内部使用（作为委托层）。可以考虑内联消除这层间接，但属于较大重构，不在本里程碑范围内。

## 结论

精简不改变功能行为。代码组织合理，遵循项目现有风格。
