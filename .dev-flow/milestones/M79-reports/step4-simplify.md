# Step 4: Code Simplify Report

## 范围
M79 期间变更的前端文件（34f1fc93^..HEAD）：
- composables/useTabs.ts（新建）
- composables/usePaneLayout.ts、useSftpDrawer.ts、useWorkspacePersistence.ts
- pages/WorkspacePage.vue、EnvironmentsPage.vue
- components/EnvironmentTile.vue、components/ui/Button.vue、components/ui/Select.vue
- config/shortcuts.ts

## 检查维度（CLAUDE.md simplify 原则）
- 重复代码 / 过度设计 / 提前实现 / 大文件拆分 / Rust·Vue 结构一致性

## 发现与处理

### 1. 冗余委托层（WorkspacePage.vue）
- 原 `localHandleTabCtxAction` 通过中间变量 `ctxActionHandler = handleTabCtxAction` 再委托给 useTabs 版，形成两层包装。
- 简化：删除 `ctxActionHandler` 中间变量，`localHandleTabCtxAction` 直接调用已解构的 `handleTabCtxAction`。

### 2. 未使用死代码（WorkspacePage.vue）
- `showMovePane` ref 与 `moveToPane` 函数无任何模板引用（pane 右键菜单的「移动到面板」子菜单从未渲染），属死代码。
- 删除 `showMovePane`、`moveToPane`，消除未用声明。

### 3. useTabs 解构过度暴露（WorkspacePage.vue）
- 本地只用到 `closeTab`/`handleTabCtxAction` 等，却解构了 `closeOtherTabs`/`closeTabsRight`/`closeTabsLeft`/`closeAllTabs`/`duplicateTab`/`startRename` 等仅供 useTabs 内部使用的项，触发 lint unused warning。
- 精简：从本地解构移除未直接引用的项，保留实际使用的导出。

## 保留项（不视为过度设计）
- `handlePaneCtxAction` / `onPaneContextMenu` / `paneContextMenu` 是子任务4「右键菜单分屏操作」的 pane 侧功能骨架，逻辑完整仅模板渲染缺失，属于「未完成功能」而非死代码，精简阶段保留不删。

## 验证
- type-check：通过（vue-tsc --noEmit 无错误）
- lint（改动文件）：0 error，仅 1 个 warning（handlePaneCtxAction 未渲染使用，预存功能缺口，非本次引入）
- build：通过

## 结论
精简未改变任何功能行为，仅移除冗余委托与死代码，符合步骤4 门禁。
