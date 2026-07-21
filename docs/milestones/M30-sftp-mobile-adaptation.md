# M30: SFTP 移动端适配（浮动工具栏 + 上下分栏）

## Context

M29 完成了 SSH 终端主题增强。当前 FilesPage.vue 是纯桌面布局（水平双面板、4px 拖拽手柄、小图标工具栏），在 360-414px 移动端视口完全不可用。PRODUCT.md 3.8 要求 SFTP 移动端上下分栏 + 浮动工具栏。本里程碑将 FilesPage 适配移动端。

版本类型：minor（新功能），版本号 0.28.0 → 0.29.0。

## 产品边界

**本阶段做：**
- FilesPage 移动端单面板布局（双面板→单面板 + 面板切换）
- 移动端浮动工具栏（上传/下载/新建文件夹/刷新/更多）
- 响应式对话框（chmod/删除确认/文件夹同步）
- 隐藏移动端不适用的元素（调整手柄、Modified 列）

**本阶段不做：**
- 触屏拖拽传输（替代方案：复制/移动到操作）
- SFTP 传输队列移动端优化（后续里程碑）
- FilesDrawer 移动端适配（已是单面板，基本可用）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | FilesPage 移动端单面板布局 + 面板切换 | ⬜ |
| 2 | 移动端浮动工具栏 MobileFilesBar | ⬜ |
| 3 | 响应式对话框 + 列隐藏 | ⬜ |

## 子任务详细设计

### 1 FilesPage 移动端单面板布局 + 面板切换

**功能目标**

在 `@media (max-width: 768px)` 下将水平双面板改为单面板显示，隐藏调整手柄，添加面板切换控件。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 添加响应式 CSS + 面板切换状态

**交互设计**

- 移动端：只显示活动面板（active side），隐藏另一个面板
- 顶部添加面板切换 Segmented Control：「Left」/「Right」
- 隐藏拖拽调整手柄（`.fh2`）
- 隐藏 Modified 列（只显示 Name + Size）
- 面板切换时保留各自的 currentPath 和 selected 状态

**实现流程**

1. 添加 `mobileActiveSide` ref（'left' | 'right'），默认 'left'
2. 模板中：移动端只渲染 `mobileActiveSide` 对应的面板
3. 面板切换按钮：两个按钮组成的 segmented control
4. CSS：`.fp` 在移动端改为 `flex-direction: column`，`.fh2` 设 `display: none`

**测试标准**

- 桌面端：双面板布局不变
- 移动端（≤768px）：单面板显示，可切换 Left/Right
- 切换面板后路径和选中状态保持
- type-check + build 通过

**提交信息**: `feat(files): add mobile single-panel layout with panel switcher`

### 2 移动端浮动工具栏 MobileFilesBar

**功能目标**

创建 MobileFilesBar 组件，在移动端底部显示文件操作按钮，遵循 MobileTerminalBar 的设计模式。

**文件结构**

新建：
- `packages/rex-console-web/src/features/files/MobileFilesBar.vue` — 移动端浮动工具栏

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 集成 MobileFilesBar

**交互设计**

底部固定工具栏（参照 MobileTerminalBar 模式）：
- 📤 Upload（上传）
- 📥 Download（下载选中文件）
- 📁 New Folder（新建文件夹）
- 🔄 Refresh（刷新）
- ⋯ More（更多菜单：Rename/Delete/Permissions/Copy Path）

按钮最小触控区域 40×40px，`touch-action: manipulation`。

**实现流程**

1. MobileFilesBar.vue：
   - `display: none` 默认，`@media (max-width: 768px)` 时 `display: flex`
   - `position: fixed; bottom: 0; left: 0; right: 0; z-index: 50`
   - 半透明深色背景 + backdrop-filter blur
   - Props：`selectedCount`、`emit` 事件（upload/download/newFolder/refresh/more）
2. FilesPage.vue：
   - 在 template 中添加 `<MobileFilesBar />`
   - 监听事件，调用已有的 upload/download/mkdir/refresh 函数
   - More 菜单：弹出 action sheet 显示 Rename/Delete/Permissions/Copy Path

**测试标准**

- 桌面端：不显示 MobileFilesBar
- 移动端：底部显示工具栏，按钮可点击
- Upload 触发文件选择器
- Refresh 刷新当前面板
- More → Delete 弹出确认对话框
- type-check + build 通过

**提交信息**: `feat(files): add MobileFilesBar for touch-friendly file operations`

### 3 响应式对话框 + 列隐藏

**功能目标**

确保所有对话框在移动端不溢出，文件列表列在移动端自适应。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 响应式 CSS
- `packages/rex-console-web/src/features/files/FolderSyncDialog.vue` — 响应式宽度

**实现流程**

1. FilesPage.vue CSS：
   - `@media (max-width: 768px)` 下 `.cm`（Modified 列）`display: none`
   - Chmod 对话框：`min-width: auto; width: 90vw; max-width: 340px`
   - 删除确认对话框：同上
   - 面板工具栏：按钮缩小，路径显示截断
2. FolderSyncDialog.vue：
   - `min-width: auto; width: 95vw; max-width: 520px`
   - 预览表格在移动端隐藏部分列

**测试标准**

- 移动端：对话框不溢出屏幕
- 文件列表：只显示 Name + Size
- 工具栏按钮不重叠
- type-check + build 通过

**提交信息**: `feat(files): add responsive dialog widths and column hiding for mobile`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ 不引入新概念（纯 UI 适配）
- ✅ 不跳阶段实现
- ✅ 实现细节不污染产品文档
- ✅ 与 Xftp 对标（移动端上下分栏 + 浮动工具栏）

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
