# 步骤7：设计再确认报告

## 核对维度

### 子任务实现 vs 里程碑文档

| 子任务 | 里程碑描述 | 实现状态 | 一致性 |
|--------|-----------|---------|--------|
| 1. TerminalSftp 补全 rename | 右键菜单新增「重命名」，Enter 确认/Esc 取消，选中文件名部分（不含扩展名），重命名成功后刷新列表 | `startRename` → `nextTick` → `setSelectionRange` 只选文件名部分；`confirmRename` → `renameFile` → `loadDir`；`cancelRename` 清空状态；catch 块 `toastError` | ✅ 一致 |
| 2. TerminalSftp 补全"发送到" | 右键菜单新增「发送到…」（仅文件，有目标时显示），弹出目标列表，确认后调用 `createTransfer`，Toast 提示 | `openSendTo` → modal overlay → `sendToTargets` computed 过滤 tabs（files/terminal/s3，排除当前资源）→ `confirmSendTo` 构造 `TransferEndpoint` 调用 API | ✅ 一致 |
| 3. WorkspaceTerminal 复用 TerminalSftp | 移除内联 SFTP，改为 `import TerminalSftp`，保持分隔条拖拽、拖拽文件到终端、移动端浮动工具栏 | `WorkspaceTerminal.vue` 第 156 行 `import TerminalSftp`，第 41-46 行 `<TerminalSftp>` 组件，分隔条 `startResize`/`onResize` 保留，`handleDrop`/`handleDragOver` 保留 | ✅ 一致 |

### 交互核对

| 检查项 | 结果 |
|--------|------|
| 右键菜单「重命名」位置正确（「复制路径」「删除」同级） | ✅ 第 111 行，在 copyPath 和 delete 之间 |
| 右键菜单「发送到…」仅文件且有目标时显示 | ✅ 第 112 行 `v-if="ctxMenu.entry.file_type !== 'directory' && sendToTargets.length > 0"` |
| rename input 样式使用 accent 边框色 | ✅ `.sftp-rename-input` 使用 `border-color: var(--accent)` |
| sendTo modal 使用 overlay 遮罩 | ✅ `.sftp-sendto-overlay` 全屏遮罩，点击关闭 |
| WorkspaceTerminal 保持 SFTP 按钮（Ctrl+Shift+F） | ✅ 第 13 行 toolbar 按钮 |
| WorkspaceTerminal 保持分隔条拖拽 | ✅ 第 38 行 divider + startResize |
| WorkspaceTerminal 保持拖拽文件到终端 | ✅ 第 2 行 `@drop.prevent`、第 447 行 `handleDrop` |
| WorkspaceTerminal 保持移动端浮动工具栏 | ✅ 第 64-82 行 mobile-bar |

### API 复用核对

| API | 源文件 | 调用位置 | 一致性 |
|-----|--------|---------|--------|
| `renameFile(resourceId, oldPath, newPath)` | `api/files.ts` | TerminalSftp.vue 第 331 行 | ✅ |
| `createTransfer(source, target)` | `api/transfer.ts` | TerminalSftp.vue 第 359 行 | ✅ |
| `useTabs()` | `features/workspace/useTabs.ts` | TerminalSftp.vue 第 173 行 | ✅ |
| `useToast()` | `composables/useToast.ts` | TerminalSftp.vue 第 174 行 | ✅ |

### 设计核对点逐项确认

| 核对点 | 结果 |
|--------|------|
| 单用户设计：无权限检查 | ✅ 所有文件操作无权限校验 |
| 自托管：文件操作通过已有 API | ✅ 复用 `renameFile`、`createTransfer`，未新增 API |
| 数据不经浏览器：文件传输由后端完成 | ✅ `createTransfer` 只创建任务，不传文件数据 |
| 不引入多用户/RBAC | ✅ 未引入任何用户/权限概念 |
| 深色主题一致性 | ✅ 所有新增 CSS 使用 `var(--bg-*)`、`var(--border)`、`var(--accent)` 等主题变量 |
| i18n 覆盖：新增 UI 文本使用 i18n key | ✅ 所有文本使用 `t('files.*')`、`t('common.*')` |

### 产品文档一致性

| 检查项 | 结果 |
|--------|------|
| 产品文档未被修改 | ✅ `docs/PRODUCT.md` 无变更 |
| 实现在产品文档功能边界内 | ✅ SFTP rename 和跨连接传输均为已有功能范畴 |

## 结论

✅ 所有 3 个子任务实现与里程碑文档一致。API 复用正确，交互符合设计，产品语义未变。
