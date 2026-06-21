# M18 步骤4：代码精简报告

**日期：** 2026-06-21

## 精简检查

### 1. 重复代码 → 已修复

| 问题 | 修复 |
|------|------|
| `generateUpdate`/`generateDelete`/`copyRow`/`copyJson` 逻辑在 `handleCellContextMenu` 和 `handleRowContextMenu` 中重复 | 提取 `rowToTsv`、`rowToJson`、`formatValStr`、`generateUpdateSql`、`generateDeleteSql` 辅助函数 |
| `handleExecuteSelection`/`handleSort`/`handleGenerateSql`/`handleSave`/`handleShowHistory` 在 `WorkspaceSql.vue` 和 `SqlConsole.vue` 中完全重复 | 提取 `useSqlTabActions` composable，两个父组件复用 |

### 2. 过度设计 → 无

- SFTP 面板逻辑内联在 WorkspaceTerminal.vue，对于单一组件是合理的
- ContextMenu 使用现有 singleton composable，无过度抽象

### 3. 提前实现 → 无

- `handleSave` 和 `handleShowHistory` 保持为 TODO 占位，未提前实现

### 4. 文件结构 → 合理

- 新增 `useSqlTabActions.ts` 在 `features/sql/` 目录，符合功能域组织
- `FileList.vue` 添加 `draggable` 支持是合理的增强

### 5. 功能行为不变

所有精简仅提取重复代码为共享函数/composable，功能行为完全不变。

## 结论

✅ 精简完成，功能行为不变。
