# M48：侧栏增强 + 工作空间 Tab 交互

## Context

PRODUCT.md §4 导航结构要求侧栏包含「连接树（环境→资源，可折叠、收藏、最近使用）」和「全局搜索」；§3.5 工作空间要求双击 Tab 进入分屏、拖 Tab 到目标 Pane。这些功能在 M0-M47 期间未实现，属于对标 Xshell 体验的遗留缺口。

版本类型：minor（新增交互功能，向后兼容）

## 产品边界

### 做什么
1. 侧栏收藏/最近使用 tab：用户可标记常用资源为收藏，自动记录最近打开的资源
2. 侧栏全局搜索：侧栏顶部搜索框，跨环境/资源名实时搜索
3. 双击 Tab 进入左右分屏：双击 Tab 标签自动创建左右分屏布局
4. 拖 Tab 到目标 Pane：拖拽 Tab 到另一个 Pane，填充或交换位置

### 不做什么
- TLS 实现（复杂后端安全任务，留作 M49）
- Tab 拖出分离到新窗口（PRODUCT.md 标记为可选）
- 拖文件到 SQL 表触发导入（minor UX，优先级低）
- Redis CMD 批量命令导入（minor UX，优先级低）
- 资源属性隧道 Tab（后端依赖，留作后续）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 侧栏收藏/最近使用 tab | ✅ |
| 2 | 侧栏全局搜索 | ✅ |
| 3 | 双击 Tab 进入左右分屏 | ✅ |
| 4 | 拖 Tab 到目标 Pane | ✅ |
| 5 | i18n + 验证 | ✅ |

## 子任务详细设计

### 1 侧栏收藏/最近使用 tab

- **功能目标**：ResourcePanel 增加「收藏」和「最近使用」tab，与现有 connections tab 并列
- **文件结构**：修改 `features/resource-panel/ResourcePanel.vue`、新增 `stores/favorites.ts`
- **接口设计**：
  - 新增 Pinia store `useFavoritesStore`：`favorites: Map<resourceId, timestamp>`、`recent: Map<resourceId, {time, tab}>`
  - 侧栏 tab 切换：connections / favorites / recent
  - 收藏操作：资源右键菜单增加「收藏/取消收藏」，资源名旁显示 ⭐ 图标
  - 最近使用：自动记录打开的资源（工作区打开时写入），按时间倒序展示
- **交互设计**：
  - 收藏 tab：星标图标 tab，显示所有收藏资源（扁平列表，按添加时间排序）
  - 最近使用 tab：时钟图标 tab，显示最近 20 个打开的资源（按时间倒序）
  - 点击资源项 → 在工作区打开对应控制台 Tab（复用现有 openResource 逻辑）
- **持久化**：收藏和最近使用存储在 localStorage（单用户场景无需后端）
- **提交信息**：`feat(workspace): add sidebar favorites and recent tabs`

### 2 侧栏全局搜索

- **功能目标**：侧栏顶部增加搜索框，跨环境/资源名实时搜索
- **文件结构**：修改 `features/resource-panel/ResourcePanel.vue`
- **接口设计**：
  - 搜索框位于侧栏 Logo 下方、连接树上方
  - 实时过滤：输入即搜，debounce 200ms
  - 搜索范围：所有环境下的资源名、描述（不搜密码/主机名等敏感字段）
  - 结果分组：按环境分组显示，高亮匹配文本
  - 回车/点击 → 打开第一个匹配结果
- **交互设计**：
  - 搜索框 placeholder：`t('sidebar.search')`
  - 无结果：显示「无匹配资源」
  - ESC 清空搜索并关闭结果
- **提交信息**：`feat(workspace): add sidebar global search`

### 3 双击 Tab 进入左右分屏

- **功能目标**：双击 Tab 标签自动创建左右分屏布局，将当前 Tab 移到左面板
- **文件结构**：修改 `pages/WorkspacePage.vue`
- **接口设计**：
  - Tab 添加 `@dblclick` 事件
  - 双击时：如果当前是单面板 → 切换为左右分屏，当前 Tab 移到左面板，右面板为空
  - 如果已经是分屏 → 无操作（或切换为四宫格，可选）
- **交互设计**：
  - 双击 Tab → 平滑过渡到左右分屏
  - 当前 Tab 内容保留在左面板
  - 右面板显示空状态 + 新建连接提示
- **提交信息**：`feat(workspace): double-click tab to split pane`

### 4 拖 Tab 到目标 Pane

- **功能目标**：拖拽 Tab 到另一个 Pane 的标题区域，将 Tab 移动到该 Pane
- **文件结构**：修改 `pages/WorkspacePage.vue`
- **接口设计**：
  - Tab 添加 `draggable="true"` + `@dragstart` / `@dragend` 事件
  - Pane 标题区域添加 `@dragover` / `@drop` 事件
  - Drag 数据：携带 `{ tabId, sourcePaneId }`
  - Drop 处理：从源 Pane 移除 Tab，添加到目标 Pane
- **交互设计**：
  - 拖拽时 Tab 半透明 + 目标 Pane 高亮边框
  - 放下后 Tab 立即出现在目标 Pane
  - 源 Pane 无 Tab 时自动关闭（可选，或保留空 Pane）
- **提交信息**：`feat(workspace): drag tab between panes`

### 5 i18n + 验证

- **功能目标**：新增 i18n key（sidebar.search、sidebar.favorites、sidebar.recent 等），确保 type-check + lint 通过
- **文件结构**：修改 `src/i18n/locales/zh.json`、`src/i18n/locales/en.json`
- **提交信息**：`chore(M48): add i18n keys and verify`

## 设计核对点

- 侧栏 tab 切换不影响现有连接树功能
- 收藏/最近使用数据持久化到 localStorage
- 全局搜索不泄露敏感信息（密码、主机名等）
- Tab 拖拽交互与现有右键菜单「移动到面板」功能一致
- 双击 Tab 分屏时保留当前终端/查询/文件状态
- 所有新增字符串使用 i18n

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
