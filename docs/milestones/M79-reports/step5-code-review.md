# 代码审查：M79

## 变更概览

- **变更文件**：10 个前端文件（composables 4、pages 2、components 3、config 1）
- **审查时间**：2026-08-13

## 问题列表

| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 1 | 🔴 | config/shortcuts.ts | 18-36 | 快捷键文档与代码不一致：shortcuts.ts 将 `tab-1~5` 记为 `Alt+1~5`（切换标签），但实际代码（WorkspacePage）中 `Alt+1~5` 是布局预设，`Alt+6~9` 才是标签跳转；同时 `layout-single` 等也用 `Alt+1~5`，造成自相矛盾且与运行时注册冲突。作为子任务11「快捷键文档」的产物，数据应准确反映真实快捷键 |
| 2 | 🟡 | composables/useWorkspacePersistence.ts | 75-83 | 恢复 tab→pane 绑定时，若持久化的 tabs 数量多于布局 leaves 数量，多余 tab 无法绑到任何 pane；且未做 tabs/leaves 数量对齐校验。常见场景可用，属边界健壮性问题 |
| 3 | 🟡 | pages/WorkspacePage.vue | 144-159 | `handlePaneCtxAction`/`onPaneContextMenu`/`paneContextMenu` 逻辑完整但模板未渲染 pane 右键 ContextMenu（子任务4「右键菜单分屏操作」的 pane 侧渲染缺失），导致 pane 右键菜单不显示。属功能半成品，非本次引入的退化 |

## 其他文件结论

- `useTabs.ts`：tab 管理逻辑抽离清晰，类型安全（无 any），close 系列/拖拽/右键委托正确 ✅
- `usePaneLayout.ts`：树操作正确，已修复 deserialize 错误日志与 splitPane children 校验 ✅
- `useSftpDrawer.ts`：拖拽监听正确用命名函数便于卸载移除 ✅
- `EnvironmentTile.vue`：已补 aria-label 与 focus-within 可见性 ✅
- `Button.vue`：ripple setTimeout 已加 onBeforeUnmount 清理 ✅
- `Select.vue`：ARIA combobox 模式完整（aria-expanded/controls/activedescendant）✅
- `EnvironmentsPage.vue`：修复 `</button>`→`</div>` 标签不匹配，消除 build 阻塞 ✅

## 汇总

- 🔴 必须修复：1
- 🟡 应该修复：2
- 🟢 可选改进：0
- **结论**：1 个 🔴 必须修复项（shortcuts.ts 与代码不一致），步骤5 不通过，需修复后重入
