# Step 5: Code Review — M4 SQL 控制台

## 审查范围

| 文件 | 变更类型 |
|------|----------|
| `crates/rex-common/src/sql.rs` | 新增：SqlConnector trait + 共享类型 |
| `crates/rex-mysql/src/lib.rs` | 修改：MySqlConnector 实现 |
| `crates/rex-postgresql/src/lib.rs` | 修改：PostgresConnector 实现 |
| `crates/rex-sqlite/src/lib.rs` | 修改：SqliteConnector 实现 |
| `crates/rex-hub/src/sql_api.rs` | 新增：REST API 路由 |
| `crates/rex-hub/src/bin/rex-hub.rs` | 修改：注册 SQL 路由 |
| `packages/rex-console-web/src/api/sql.ts` | 新增：前端 API 封装 |
| `packages/rex-console-web/src/features/sql/SqlPage.vue` | 新增：SQL 页面布局 |
| `packages/rex-console-web/src/features/sql/SqlNavTree.vue` | 新增：导航树 |
| `packages/rex-console-web/src/features/sql/SqlEditor.vue` | 新增：CodeMirror 编辑器 |
| `packages/rex-console-web/src/features/sql/SqlResultGrid.vue` | 新增：结果网格 |
| `packages/rex-console-web/src/features/sql/useSqlNav.ts` | 新增：导航数据 |
| `packages/rex-console-web/src/features/sql/useSqlQuery.ts` | 新增：查询执行 |

---

## 发现

### 🟡 应该修复

**1. SQL 注入风险：元数据查询使用字符串拼接**

文件：`crates/rex-mysql/src/lib.rs`、`crates/rex-postgresql/src/lib.rs`

`tables()` 和 `columns()` 方法中使用 `format!` 拼接 SQL 查询 `information_schema`，`db` 和 `table` 参数直接嵌入字符串。虽然是元数据查询且 db/table 来自后端自身数据，但最佳实践应使用参数化查询。

**建议**：使用 sqlx 的参数绑定替代字符串拼接。当前风险低（参数来自自身 metadata 查询），可在后续迭代修复。

---

**2. `useSqlQuery` 的 `run` 接受 `state: QueryState` 而非直接操作 tab**

文件：`packages/rex-console-web/src/features/sql/useSqlQuery.ts`

`run` 函数接受外部 `QueryState` 对象，设计灵活但增加了调用方复杂度。`SqlPage.vue` 中 `onExecute` 已正确使用。

**建议**：可接受的设计，无需立即修改。

---

### 🟢 可选改进

**3. `SqlConnectorFactory` 未被实际使用**

文件：`crates/rex-common/src/sql.rs`

`SqlConnectorFactory` 的 `connect()` 方法默认返回 `Err`，实际连接逻辑在 `sql_api.rs` 中通过 `DatabaseType` match 实现。工厂类可简化为仅保留 `DatabaseType` 枚举。

**建议**：保持现状，后续可能需要工厂模式。

---

**4. 前端 `onVDragMove` 使用 `document.querySelector` 定位容器**

文件：`packages/rex-console-web/src/features/sql/SqlPage.vue`

垂直分割拖拽使用全局选择器 `.sql-right-split`，若页面有多个实例可能定位错误。可改用 ref 引用。

**建议**：可接受，单用户场景下不太可能出现多实例。

---

## 总结

| 级别 | 数量 |
|------|------|
| 🔴 必须修复 | 0 |
| 🟡 应该修复 | 2 |
| 🟢 可选改进 | 2 |

**结论**：✅ 无 🔴 必须修复项，可以进入步骤 6。
