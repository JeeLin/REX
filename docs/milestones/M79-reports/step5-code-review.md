# 代码审查：M79

## 变更概览

- **变更文件**：10 个前端文件（composables 4、pages 2、components 3、config 1）
- **审查时间**：2026-08-13（重入，🔴 已修复）

## 问题列表

| # | 严重程度 | 文件 | 行号 | 描述 | 状态 |
|---|----------|------|------|------|------|
| 1 | 🔴 | config/shortcuts.ts | 18-36 | 快捷键文档与代码不一致：tab-1~5 记为 Alt+1~5（切换标签），但代码 Alt+1~5 是布局预设、Alt+6~9 才是标签跳转 | ✅ 已修复（对齐 WorkspacePage 实际注册：Alt+1~5=布局，Alt+6~9=标签；Ctrl+B=SFTP，Ctrl+Shift+B=广播） |
| 2 | 🟡 | composables/useWorkspacePersistence.ts | 75-83 | 恢复 tab→pane 绑定时 tabs 数量 > leaves 数量边界未校验 | 已知，非本次退化 |
| 3 | 🟡 | pages/WorkspacePage.vue | 144-159 | pane 右键 ContextMenu 逻辑完整但模板未渲染（子任务4 pane 侧渲染缺失） | 已知功能半成品，非本次退化 |

## 其他文件结论

- `useTabs.ts`：tab 管理逻辑抽离清晰，类型安全 ✅
- `usePaneLayout.ts`：树操作正确，已修复 deserialize 日志与 splitPane 校验 ✅
- `useSftpDrawer.ts`：拖拽监听正确用命名函数便于卸载移除 ✅
- `EnvironmentTile.vue`：已补 aria-label 与 focus-within 可见性 ✅
- `Button.vue`：ripple setTimeout 已加 onBeforeUnmount 清理 ✅
- `Select.vue`：ARIA combobox 模式完整 ✅
- `EnvironmentsPage.vue`：修复标签不匹配消除 build 阻塞 ✅
- `shortcuts.ts`：已修正为与代码一致 ✅

## 汇总

- 🔴 必须修复：0（原 1 个已修复）
- 🟡 应该修复：2（均为预存/边界问题，非本次引入退化，记录备查）
- 🟢 可选改进：0
- **结论**：无 🔴 必须修复项，步骤5 通过
