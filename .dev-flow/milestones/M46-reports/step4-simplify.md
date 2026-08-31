# 代码精简：M46 右键上下文菜单补全

## 精简检查

| # | 检查项 | 结果 | 说明 |
|---|--------|------|------|
| 1 | 代码重复 | ✅ 无问题 | 各模块 context menu 实现独立，无跨文件重复逻辑 |
| 2 | 不必要复杂度 | ✅ 无问题 | 菜单逻辑直接明了，无过度抽象 |
| 3 | 命名一致性 | ✅ 无问题 | 统一使用 `ctxMenu` / `folderCtx` / `tabContextMenu` 命名 |
| 4 | 样式一致性 | ✅ 无问题 | 所有菜单使用 CSS 变量（`--bg-elevated`, `--border`, `--radius` 等） |
| 5 | 未使用代码 | ✅ 无问题 | 无残留 dead code |

## 变更文件

- `WorkspacePage.vue`：Tab 右键菜单 +71 行
- `TerminalContextMenu.vue`：终端右键菜单 +43 行
- `TerminalView.vue`：新增 emit 事件 +18 行
- `SqlNavTree.vue`：SQL 导航树右键菜单 +57 行
- `RedisPage.vue`：Redis 文件夹右键菜单 +326 行（含 flat tree 视图）
- `AuditLogPage.vue`：审计日志右键菜单 +94 行
- `EnvironmentsPage.vue`：环境卡片右键菜单 +47 行
- `EnvironmentDetailPage.vue`：资源行右键菜单 +162 行
- `ResourcePanel.vue`：侧栏资源右键菜单 +83 行

## 结论

精简不改变功能行为，无需修改。代码质量良好，遵循项目现有模式。
