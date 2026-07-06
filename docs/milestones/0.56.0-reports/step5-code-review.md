# 步骤5：代码审查报告

## 里程碑：0.56.0 SQL 控制台体验优化

## 审查范围

本次里程碑修改的 8 个文件（不含测试）：
- `SqlTabs.vue` — 未保存标记、副标题
- `SqlResults.vue` — 斑马纹、行选中
- `SqlHistoryPanel.vue` — 时间分组、右键菜单
- `useSqlTabActions.ts` — subtitle computed
- `SqlConsole.vue` — 事件绑定

## 审查结果

### 🔴 必须修复（已修复）

| # | 文件 | 问题 | 修复 |
|---|------|------|------|
| 1 | `SqlResults.vue` | `selectedRow` 在结果变化时未重置，导致新结果高亮错误行 | ✅ 在 result watcher 中添加 `selectedRow.value = null` |

### 🟡 应该修复（已修复）

| # | 文件 | 问题 | 修复 |
|---|------|------|------|
| 2 | `SqlResults.vue` | `props.result` 有两个独立 watcher，执行顺序隐式且脆弱 | ✅ 合并为单个 watcher |
| 3 | `SqlResults.vue` | 头部排序与右键排序冲突，result 变化后 sort 状态残留 | ✅ 在 result watcher 中重置 `sortColumn`/`sortDirection` |

### 🟡 应该修复（不在本次范围）

| # | 文件 | 问题 | 说明 |
|---|------|------|------|
| 4 | `SqlHistoryPanel.vue` | 历史删除仅客户端生效 | 设计限制：后端无单条删除 API，里程碑文档明确"不修改后端 API" |
| 5 | `SqlResults.vue` | `pageSize` select 会导致类型从 number 变 string | 预存问题 |
| 6 | `SqlConsole.vue` | `handleTabSave`/`handleTabSaveAs` 缺少 try/catch | 预存问题 |
| 7 | `SqlConsole.vue` | `splitRatio` 可能为 NaN | 预存问题 |

### 🟢 可选改进（不在本次范围）

| # | 文件 | 问题 |
|---|------|------|
| 8 | `SqlResults.vue` | `generateUpdateSql` WHERE 子句未使用 `formatValStr` |
| 9 | `SqlHistoryPanel.vue` | `parseInt(executed_at)` 未处理 NaN |

## 结论

✅ **通过**。本次里程碑引入的 3 个问题已全部修复。其余发现均为预存问题或设计限制。
