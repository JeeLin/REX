# M25 步骤4：代码精简报告

## 检查范围

M25 三个子任务的全部代码变更（3 个 commit，19 个文件，+1433/-51 行）。

## 精简项

| # | 文件 | 问题 | 处理 |
|---|------|------|------|
| 1 | `SqlSidebar.vue:152` | `select-table` 事件在折叠时也触发，导致编辑器内容被意外覆盖 | ✅ 已修复：移到展开分支 |
| 2 | `SqlConsole.vue:166-171` | `handleQueryDeleted` 为空函数 | ✅ 已修复：实现清空 queryId |
| 3 | `SqlConsole.vue:152` | `handleTabRename` 硬编码 `'输入新名称:'`，未使用 i18n | ✅ 已修复：改用 `t('sql.sidebar.renamePrompt')` |
| 4 | `SqlSidebar.vue:225-235` | `expandAll` 双循环可合并为单循环 | ✅ 已修复 |
| 5 | `queries.rs:184-207` | `rename_query` 与 `update_query` 功能重叠 | 🟡 保留：RESTful 语义清晰，rename 端点有独立的非空校验 |
| 6 | `queries.rs` + 其他模块 | `test_state()` 重复定义（5 份） | ⏭️ 跳过：预存技术债务，不属于 M25 范围 |

## 结论

精简不改变功能行为。3 项修复为 bugfix + i18n 补全 + 代码合并，2 项合理保留/跳过。
