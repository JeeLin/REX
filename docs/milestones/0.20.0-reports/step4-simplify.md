# M19 步骤4：代码精简报告

## 精简项

### 已修复

1. **移除未使用的 `SplitPane` 接口** — WorkspacePage.vue 中定义了 `SplitPane` 但从未使用，已删除。

2. **移除空操作 `@resized` 处理器** — `Splitpanes` 组件上的 `@resized="() => {}"` 无实际作用，已删除。

3. **添加 SFTP 拖拽监听器清理** — `startSftpDrag` 在 document 上添加了 mousemove/mouseup 监听器，但组件卸载时未清理。已在 `onBeforeUnmount` 中添加清理逻辑。

### 已知但未处理（后续迭代）

4. **FilesDrawer 与 FilesPage 重复代码** — `fmtSize`、文件选择逻辑、删除逻辑、导航逻辑在两个组件中重复。可提取为共享 composable（`useFileManager`），但属于较大重构，不在本里程碑范围内。

5. **SSH Tab 抽屉状态未按 Tab 隔离** — `showSftpDrawer` 和 `sftpDrawerHeight` 是全局状态，多个 SSH Tab 共享。当前行为：切换到非 SSH Tab 时隐藏抽屉，切回时恢复。可接受。

6. **TerminalView `@update:status` 监听器未触发** — WorkspacePage 监听了 `@update:status` 但 TerminalView 从未 emit 该事件。这是既有问题，不在本里程碑修复范围。

## 结论

精简后功能行为不变，移除了死代码和潜在的内存泄漏。
