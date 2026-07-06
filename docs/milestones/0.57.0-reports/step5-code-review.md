# 步骤5：代码审查报告

## 里程碑：0.57.0 终端移动端浮动工具栏

## 审查范围

- `packages/rex-console-web/src/features/terminal/TerminalMobileToolbar.vue`（新增）
- `packages/rex-console-web/src/pages/Terminal.vue`（修改）
- `packages/rex-console-web/src/features/terminal/__tests__/TerminalMobileToolbar.test.ts`（新增）

## 审查发现

### 🔴 必须修复

| # | 文件 | 问题 | 严重程度 |
|---|------|------|----------|
| 1 | TerminalMobileToolbar.vue | **i18n 键名错误**：组件使用 `terminal.mobile.history`/`paste`/`more`，但实际 i18n 键是 `ws.terminal.mobile.history`/`paste`/`more`（在 zh.ts/en.ts 第 230-232 行），按钮文字会显示为原始 key 而非翻译文本 | 🔴 |
| 2 | Terminal.vue | **`toolbar-action` 事件监听缺失**：`handleMoreAction` 在 `window` 上 dispatch `toolbar-action` CustomEvent，但 Terminal.vue 无监听器。更多菜单中的清屏、SFTP、全屏、断开连接功能完全无响应 | 🔴 |

### 🟡 应该修复

| # | 文件 | 问题 |
|---|------|------|
| 1 | Terminal.vue | `checkMobile` 中 `'ontouchstart' in window` 在某些桌面触屏笔记本上可能误判为移动端，建议增加额外条件（如 `window.innerWidth < 768` 优先） |
| 2 | Terminal.vue | `onBeforeUnmount` 未移除 `checkMobile` 的 `resize` 监听器（`window.removeEventListener('resize', checkMobile)`），存在轻微内存泄漏 |

### 🟢 可选改进

| # | 文件 | 说明 |
|---|------|------|
| 1 | TerminalMobileToolbar.vue | 更多菜单无点击外部关闭逻辑，用户点击菜单外区域不会自动关闭 |
| 2 | TerminalMobileToolbar.vue | 无 `onBeforeUnmount` 生命周期钩子清理 `showMoreMenu` 状态 |

## 结论

🔴 发现 2 个必须修复项，不通过。需要修复 i18n 键名和添加 `toolbar-action` 事件监听后再重新审查。
