# 步骤4：代码精简报告

## 检查范围

本次精简覆盖以下文件：
- `crates/rex-hub/src/sql.rs` — DDL API 后端
- `packages/rex-console-web/src/features/sql/SqlSidebar.vue` — 侧边栏组件
- `packages/rex-console-web/src/i18n/zh.ts` / `en.ts` — 国际化

## 精简操作

### 1. 消除重复数据库查询（sql.rs）

`get_ddl` 函数中 `resource_protocol()` 被调用了两次（view 分支和 table 分支各一次），实际是相同查询。提取到 match 外部，只查一次。

### 2. 提取公共函数消除重复逻辑（SqlSidebar.vue）

`toggleTable` 和 `expandAll` 中都有相同的列加载逻辑：

```ts
if (!columns.value.has(name)) {
  const cols = await listColumns(props.resourceId, props.database, name)
  columns.value.set(name, cols)
}
```

提取为 `loadColumnsForTable(tableName)` 函数，两处复用。

### 3. 清理重复 i18n 键

`zh.ts` 和 `en.ts` 中 `sql.tab.ctx` 出现了两次（第653行和第711行），删除了第二次重复定义，修复了 TypeScript 编译错误。

## 结论

- ✅ 所有精简均为组织方式优化，未改变功能行为
- ✅ type-check 通过
- ✅ lint 通过（无新增 error）
