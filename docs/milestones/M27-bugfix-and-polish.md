# M27: Bugfix & Polish（SQL 执行模式修复 + UX 完善）

## Context

M26 里程碑的代码审查（step5）发现了若干遗留问题：SQL Run Current/Run Selected 模式未传递光标位置和选中文本导致实际不生效；剪贴板弹窗缺少点击外部关闭；Redis 切换 DB 未检查响应状态；SFTP 删除文件无确认对话框。本里程碑修复这些用户可感知的 bug 和 UX 缺陷。

版本类型：patch（bug 修复），版本号 0.26.0 → 0.26.1。

## 产品边界

**本阶段做：**
- SQL 编辑器：Run Current/Run Selected 修复（传递 cursorPos/selectedText）
- SQL 编辑器：剪贴板弹窗点击外部关闭
- Redis：切换 DB 响应检查 + 错误提示
- SFTP：删除文件/文件夹前确认对话框
- SFTP：拖拽跳过目录时显示提示
- SQL 编辑器：onSave 实现（下载 .sql 文件）

**本阶段不做：**
- Redis FormatViewer 高级格式解码（Msgpack/Pickle 等，后续里程碑）
- SQL 格式化引擎重写（后续里程碑）
- SSH 终端背景图/透明度（后续里程碑）

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | SQL Run Current/Run Selected 修复 | ✅ |
| 2 | SQL 剪贴板弹窗 onClickOutside + onSave 下载 | ✅ |
| 3 | Redis 切换 DB 错误处理 | ✅ |
| 4 | SFTP 删除确认对话框 + 拖拽目录提示 | ✅ |

## 子任务详细设计

### 1 SQL Run Current/Run Selected 修复

**功能目标**

修复 Run Current 和 Run Selected 模式不生效的问题。当前 SqlEditor emit `execute` 时只传了完整 SQL 文本，未传 cursorPos 和 selectedText，导致 useSqlQuery 中的 current/selected 分支永远走不到。

**文件结构**

修改：
- `packages/rex-console-web/src/features/sql/SqlEditor.vue` — emit execute 时携带 cursorPos 和 selectedText
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — onExecute 接收并转发 cursorPos/selectedText 给 useSqlQuery.run()

**实现流程**

1. SqlEditor.vue：
   - 修改 emit 声明：`execute: [sql: string, cursorPos?: number, selectedText?: string]`
   - Run All（Ctrl+Enter）：emit `('execute', doc.toString())`
   - Run Current：获取 `view.state.selection.main.head` 作为 cursorPos，emit `('execute', doc.toString(), cursorPos)`
   - Run Selected：获取选中文本 `doc.sliceString(from, to)`，emit `('execute', doc.toString(), undefined, selectedText)`
   - 注：Run Current/Run Selected 按钮不在 SqlEditor 内部（在 SqlPage 工具栏的 select 下拉框），需要通过 expose 方法或事件传递当前模式
2. SqlPage.vue：
   - onExecute 改为接收三个参数，转发给 runQuery
   - 从 SqlEditor expose 获取 `getCursorPos()` 和 `getSelectedText()` 方法

**测试标准**

- Run All 执行全部 SQL（原有行为不变）
- Run Current 执行光标所在语句
- Run Selected 执行选中文本
- 无选中文本时 Run Selected 降级为 Run All
- type-check + build 通过

**提交信息**: `fix(sql): fix Run Current/Run Selected by passing cursorPos and selectedText`

### 2 SQL 剪贴板弹窗 onClickOutside + onSave 下载

**功能目标**

修复剪贴板弹窗无法通过点击外部关闭的问题；实现 onSave 功能（下载 .sql 文件）。

**文件结构**

修改：
- `packages/rex-console-web/src/features/sql/SqlPage.vue` — 添加 onClickOutside + onSave 实现

**实现流程**

1. 剪贴板弹窗：
   - 引入 `onClickOutside` from `@vueuse/core`（RedisPage 已有此用法）
   - 创建 template ref 指向弹窗容器
   - `onClickOutside(clipboardRef, () => { showClipboard.value = false })`
2. onSave：
   - 创建 Blob（`new Blob([sql], { type: 'text/sql' })`）
   - 创建临时 `<a>` 元素触发下载
   - 文件名：`query-{timestamp}.sql`

**测试标准**

- 点击剪贴板弹窗外部可关闭
- Ctrl+S 下载 .sql 文件
- type-check + build 通过

**提交信息**: `fix(sql): add onClickOutside to clipboard popup and implement onSave as file download`

### 3 Redis 切换 DB 错误处理

**功能目标**

Redis 切换 DB 时检查响应状态，失败时显示错误提示而非静默忽略。

**文件结构**

修改：
- `packages/rex-console-web/src/features/redis/RedisPage.vue` — switchDb 添加 try/catch + toast
- `packages/rex-console-web/src/api/redis.ts` — selectDb 检查 res.ok

**实现流程**

1. `redis.ts` 的 selectDb：
   - 检查 `res.ok`，不 ok 时抛出错误（与项目其他 API 一致）
2. `RedisPage.vue` 的 switchDb：
   - try/catch 包裹，catch 时显示错误 toast
   - 失败时不更新 currentDb（保持在原 DB）

**测试标准**

- 正常切换 DB 成功
- 切换到无效 DB 时显示错误提示
- 错误后 DB 选择器仍显示原 DB
- type-check + build 通过

**提交信息**: `fix(redis): add error handling for DB switch with user feedback`

### 4 SFTP 删除确认对话框 + 拖拽目录提示

**功能目标**

删除文件前弹出确认对话框；拖拽文件夹时显示不支持提示。

**文件结构**

修改：
- `packages/rex-console-web/src/features/files/FilesPage.vue` — 添加删除确认状态 + 目录拖拽提示

**实现流程**

1. 删除确认：
   - 添加 `showDeleteConfirm` state（ref<boolean>）
   - 添加 `pendingDelete` state（存储 { side, names }）
   - deleteSelected 改为先设置 pendingDelete，打开确认弹窗
   - 确认后执行实际删除
   - 弹窗内容：「Delete N file(s)? This action cannot be undone.」+ [Cancel] [Delete]
2. 拖拽目录提示：
   - onDragStart 中检查：如果拖拽项包含目录，设置 `dragHasFolder` flag
   - onDrop 中：如果 dragHasFolder 为 true，显示 toast 提示「Folder drag transfer is not yet supported」
   - 或者更简洁：在 onDragStart 中过滤掉目录，只拖拽文件

**测试标准**

- 删除文件弹出确认对话框
- 确认后执行删除
- 取消不执行删除
- 拖拽文件夹显示提示或被过滤
- type-check + build 通过

**提交信息**: `fix(files): add delete confirmation dialog and folder drag filter`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ 不引入新概念（纯 bug 修复 + UX 完善）
- ✅ 不跳阶段实现
- ✅ 实现细节不污染产品文档

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

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
