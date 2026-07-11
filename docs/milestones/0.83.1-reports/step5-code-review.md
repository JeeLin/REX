# Step 5: Code Review — 0.83.1

## 变更概要

修复 3 个工作空间交互 bug：分屏拖拽、全局快捷键、SSH 终端复制粘贴。纯前端变更，无后端/API 变更。

## 审查结果

### 🟢 useTabs.ts — reorderTab 修复

- `adjustedDstIdx` 计算逻辑正确：splice 后目标索引偏移 +1
- `position` 参数可选，向后兼容
- 唯一风险：`insertIdx` 在 `position === 'before'` 时不做 +1，符合"插入到目标前面"语义

### 🟢 TabBar.vue — drop position 跟踪

- `onDragOver` 按元素宽度中点判断左右半区，简洁正确
- `onDrop` 传递 position 给 reorderTab，`onDragEnd` 清理
- 无内存泄漏风险（ref 清理）

### 🟢 Workspace.vue — overlay guard

- Escape 优先级链：CommandPalette → ConnMenu → ShortcutsPanel，逻辑清晰
- 其余快捷键在 overlay 打开时全部跳过，正确
- contentEditable 检查补充了 contenteditable 元素场景

### 🟢 AppLayout.vue — contentEditable 检查

- 与 Workspace.vue 一致的防御逻辑，一处遗漏已补全

### 🟢 Terminal.vue — Ctrl+C/V 智能处理

- Ctrl+C：有选区→复制，无选区→SIGINT（透传 xterm）
- Ctrl+V：return false → 浏览器原生 paste → textarea paste listener → WebSocket
- Ctrl+Shift+C：强制复制
- handleCopy 改用 copyWithFallback（带 execCommand fallback）
- 与 WorkspaceTerminal.vue 行为一致

### 🟢 i18n key 重命名

- `newTab` → `newConnection`，语义准确
- en.ts / zh.ts + Workspace.vue 引用同步更新

### 🟢 useTabs.test.ts — 测试适配

- 断言更新匹配修正后行为，覆盖正确

## 总结

- 🔴 必须修复：0
- 🟡 应该修复：0
- 🟢 可选改进：0

**结论：✅ 通过**
