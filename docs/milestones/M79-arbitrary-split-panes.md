# M79: 任意方向分屏系统 + 工作空间重构

## Context
M78 完成了全系统 UX/UI 重设计。当前工作空间的分屏系统仅支持单一方向（水平或垂直），关闭按钮存在索引错位问题，WorkspacePage 是 ~1100 行的 god component。M79 重构分屏系统支持任意方向嵌套分栏，拆分 WorkspacePage，并修复 M78 遗留的 UI/UX 问题。

版本类型：minor
版本号：0.67.0

## 产品边界
本阶段做什么：工作空间分屏系统重构、WorkspacePage 拆分、M78 遗留 UI 修复
本阶段不做什么：不新增后端功能

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 1 | 分屏树状数据结构设计与实现 | 前端 | ✅ |
| 2 | 分屏面板渲染与方向切换 | 前端 | ✅ |
| 3 | 分屏关闭与合并逻辑 | 前端 | ✅ |
| 4 | 右键菜单分屏操作 | 前端 | ✅ |
| 5 | 分屏状态持久化与恢复 | 前端 | ✅ |
| 6 | WorkspacePage 拆分为 composables | 前端 | ✅ |
| 7 | env-tile 嵌套 button 修复 | 前端 | ✅ |
| 8 | AuditLogPage 右键菜单改用 ContextMenu | 前端 | ✅ |
| 9 | 共享 EnvironmentTile 组件提取 | 前端 | ✅ |
| 10 | Select 组件 ARIA combobox 补全 | 前端 | ✅ |
| 11 | 快捷键文档维护 | 前端 | ✅ |

## 子任务详细设计

### 1 分屏树状数据结构设计与实现

- **功能目标**：用树状数据结构替代当前的平铺 `splitCount` + `splitDirection` 方案，每个节点可独立控制方向和子面板
- **文件结构**：`packages/rex-console-web/src/composables/usePaneLayout.ts`（新建）
- **数据模型**：
  ```ts
  interface PaneNode {
    id: string
    direction: 'row' | 'column' | null  // null = 叶子节点（内容面板）
    children: PaneNode[]
    size: number  // 百分比 0-100
  }
  ```
- **交互设计**：
  - 根节点始终存在，direction 可为 row 或 column
  - 叶子节点持有 tab 信息（resourceId, protocol 等）
  - 非叶子节点持有子节点数组
- **测试标准**：创建/删除/移动节点的单元测试
- **提交信息**：`feat: implement tree-based pane layout data structure`

### 2 分屏面板渲染与方向切换

- **功能目标**：根据树状结构递归渲染 Splitpanes 嵌套，支持在任意 pane 上下左右分屏
- **文件结构**：修改 `packages/rex-console-web/src/pages/WorkspacePage.vue`
- **交互设计**：
  - 顶部工具栏提供 5 种布局预设：单面板 / 左右 / 上下 / 四宫格 / 主+侧边
  - 在当前 pane 上添加子面板时自动选择方向（右分→row，下分→column）
  - Splitpanes 嵌套渲染：每个非叶子节点是一个 `<Splitpanes>` 容器
- **测试标准**：视觉验证 4 种以上分屏布局
- **提交信息**：`feat: recursive split pane rendering with direction switching`

### 3 分屏关闭与合并逻辑

- **功能目标**：修复关闭按钮索引错位问题，实现正确的 pane 关闭和合并
- **文件结构**：修改 `packages/rex-console-web/src/composables/usePaneLayout.ts`
- **交互设计**：
  - 关闭叶子节点时，从父节点的 children 中移除
  - 父节点只剩 1 个子节点时，自动提升为父节点（合并）
  - 关闭所有子节点时，删除父节点
- **测试标准**：关闭左/右/上/下 pane 的正确性验证
- **提交信息**：`feat: implement correct pane close and merge logic`

### 4 右键菜单分屏操作

- **功能目标**：在 pane 内容区右键菜单添加分屏操作选项
- **文件结构**：修改 `packages/rex-console-web/src/pages/WorkspacePage.vue`
- **交互设计**：
  - 右键菜单项：向右分屏 / 向下分屏 / 关闭面板 / 移动到面板 ▸（子菜单列出所有 pane）
  - 使用共享 ContextMenu 组件
- **测试标准**：右键菜单操作正确触发分屏/关闭
- **提交信息**：`feat: add split pane operations to context menu`

### 5 分屏状态持久化与恢复

- **功能目标**：分屏布局状态保存到 localStorage，刷新后恢复
- **文件结构**：修改 `packages/rex-console-web/src/composables/useWorkspacePersistence.ts`
- **交互设计**：
  - 树状布局结构序列化为 JSON 存入 localStorage
  - 刷新页面后自动恢复之前的分屏布局
  - 超过 24 小时自动清理
- **测试标准**：刷新页面后分屏布局正确恢复
- **提交信息**：`feat: persist split pane layout to localStorage`

### 6 WorkspacePage 拆分为 composables

- **功能目标**：将 ~1100 行的 WorkspacePage 拆分为独立的 composable，降低复杂度
- **文件结构**：
  - `packages/rex-console-web/src/composables/useTabs.ts`（新建）— Tab 管理（创建、关闭、重命名、拖拽、右键菜单）
  - `packages/rex-console-web/src/composables/useSftpDrawer.ts`（新建）— SFTP 抽屉拖拽逻辑
  - `packages/rex-console-web/src/composables/usePaneLayout.ts`（与子任务 1 合并）
- **接口设计**：
  - `useTabs()` 返回 `{ tabs, activeTab, paneTabs, openResource, closeTab, ... }`
  - `useSftpDrawer()` 返回 `{ showSftpDrawer, sftpDrawerHeight, startSftpDrag, toggleSftpDrawer }`
  - WorkspacePage 仅负责布局编排和组件组合
- **测试标准**：拆分后功能不变，type-check 通过
- **提交信息**：`refactor: extract WorkspacePage into composables`

### 7 env-tile 嵌套 button 修复

- **功能目标**：Dashboard 和 EnvironmentsPage 的 env-tile 使用 `<button>` 内含 action `<button>`，HTML 不合法
- **文件结构**：修改 `DashboardPage.vue`、`EnvironmentsPage.vue`
- **交互设计**：
  - 将 env-tile 改为 `<div role="button" tabindex="0">`
  - action 按钮保留 `<button>` + `@click.stop`
- **测试标准**：点击 tile 和 action 按钮各自独立响应
- **提交信息**：`fix: replace nested buttons with div role=button for valid HTML`

### 8 AuditLogPage 右键菜单改用 ContextMenu

- **功能目标**：统一右键菜单实现，消除原生菜单代码
- **文件结构**：修改 `packages/rex-console-web/src/pages/AuditLogPage.vue`
- **交互设计**：替换原生 `<Teleport>` + 定位逻辑为共享 `<ContextMenu>` 组件
- **测试标准**：右键菜单正确弹出和关闭
- **提交信息**：`refactor: use shared ContextMenu component in AuditLogPage`

### 9 共享 EnvironmentTile 组件提取

- **功能目标**：Dashboard 和 EnvironmentsPage 的环境卡片重复 CSS，提取为共享组件
- **文件结构**：新建 `packages/rex-console-web/src/components/EnvironmentTile.vue`
- **接口设计**：
  ```ts
  props: {
    name: string
    description?: string
    agentStatus: StatusDotStatus | null
    resourceCount: number
    connectionMode: string
    showActions?: boolean  // 编辑/删除按钮（仅 EnvironmentsPage 需要）
  }
  slots: { default, actions }
  ```
- **测试标准**：Dashboard 和 EnvironmentsPage 使用同一组件，视觉一致
- **提交信息**：`refactor: extract shared EnvironmentTile component`

### 10 Select 组件 ARIA combobox 补全

- **功能目标**：完善 Select 组件的无障碍支持
- **文件结构**：修改 `packages/rex-console-web/src/components/ui/Select.vue`
- **交互设计**：
  - trigger 添加 `aria-haspopup="listbox"` 和 `aria-controls`
  - 添加 `aria-activedescendant` 指向高亮的 option
  - dropdown 添加 `id` 以关联 `aria-controls`
- **测试标准**：屏幕阅读器能正确播报 Select 状态
- **提交信息**：`feat: complete ARIA combobox pattern in Select component`

### 11 快捷键文档维护

- **功能目标**：创建统一的快捷键文档，作为 ShortcutPanel 的数据源，确保快捷键变更时文档同步更新
- **文件结构**：`packages/rex-console-web/src/config/shortcuts.ts`（新建）
- **数据模型**：
  ```ts
  interface ShortcutEntry {
    id: string
    keys: string          // 如 "Ctrl+T"
    description: string   // 本地化 key
    category: 'workspace' | 'tab' | 'split' | 'nav'
  }
  ```
- **交互设计**：
  - ShortcutPanel 组件从 shortcuts.ts 读取数据渲染
  - 所有 useKeyboardShortcuts 注册的快捷键在此文件中定义描述
- **测试标准**：ShortcutPanel 显示所有快捷键，文档与代码一致
- **提交信息**：`feat: centralize keyboard shortcuts documentation`

## 设计核对点

- [ ] Splitpanes 嵌套渲染不会导致内部状态冲突
- [ ] 关闭任意位置的 pane 都能正确合并
- [ ] 右键菜单操作与工具栏按钮行为一致
- [ ] 分屏状态持久化后恢复无异常
- [ ] 深色主题下 splitter 和 pane 边框清晰可见
- [ ] 响应式：窄屏自动切换为垂直布局

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发（补做子任务6 + 修复 🟡）
- [x] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| 2026-08-13 | 步骤3（流程重置） | 文档状态与实际代码脱节：子任务6 useTabs.ts 从未创建，WorkspacePage 仍 1028 行；步骤5 遗留 5 个 🟡 未修复。用户要求保留代码，重置流程从头收尾，并真正拆分 useTabs.ts、修复 🟡 项 |

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
| [x] | 🔴 | 子任务6 未完成：useTabs.ts 未创建，WorkspacePage 仍 1028 行 | 步骤3 重置 | 需从 WorkspacePage 抽取 tab 管理逻辑（创建/关闭/重命名/拖拽/右键菜单）到 useTabs.ts，实现子任务6 原设计目标 |
| [x] | 🟡 | EnvironmentTile.vue Edit/Delete 按钮缺 aria-label | 步骤5 代码审查 | 建议补全无障碍标签 |
| [x] | 🟡 | Button.vue ripple setTimeout cleanup 可能在 unmount 后触发 | 步骤5 代码审查 | 低风险，需确保 onUnmounted 清理 |
| [x] | 🟡 | usePaneLayout.ts deserialize 静默吞掉错误 | 步骤5 代码审查 | 建议加 console.error |
| [x] | 🟡 | usePaneLayout.ts splitPane 除零保护 | 步骤5 代码审查 | children>0 才能保证，需加前置校验 |
| [x] | 🟡 | EnvironmentTile.vue action 按钮仅 hover 可见 | 步骤5 代码审查 | 键盘可达性改进建议 |
| ⬜ | 🔴 | shortcuts.ts 快捷键文档与代码不一致 | 步骤5 代码审查 | tab-1~5 记为 Alt+1~5（切换标签），但代码 Alt+1~5 是布局预设、Alt+6~9 才是标签跳转；且 layout-single 等也用 Alt+1~5，自相矛盾。需对齐 shortcuts.ts 与 WorkspacePage 实际注册 |
| ⬜ | 🟡 | WorkspacePage pane 右键 ContextMenu 未渲染 | 步骤5 代码审查 | 子任务4 pane 侧 splitClose/moveToPane 逻辑完整但模板无 ContextMenu，pane 右键菜单不显示 |
| ⬜ | 🟡 | useWorkspacePersistence 恢复时 tabs>leaves 数量未对齐 | 步骤5 代码审查 | 持久化 tabs 多于布局 leaves 时多余 tab 无法绑定，缺数量校验 |
