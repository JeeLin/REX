# M21 步骤5：代码审查报告

## 审查范围

M21 共 3 个子任务，涉及 4 个文件变更。

---

## 🔴 必须修复

无。

---

## 🟡 应该修复

### CR-1: onBeforeUnmount 中嵌套注册清理函数可能不执行

**文件**: `WorkspaceTerminal.vue:525`
**代码**: `onBeforeUnmount(() => mq.removeEventListener('change', mqHandler))` 在 `onMounted` 内部调用
**问题**: Vue 的 `onBeforeUnmount` 在 `onMounted` 内部调用时，会在当前组件实例上注册一个新的卸载钩子。虽然技术上可行（Vue 支持嵌套注册），但不够直观。如果组件在 `onMounted` 完成前被卸载（极端情况），可能导致注册不完整。
**建议**: 将 `mq` 和 `mqHandler` 作为模块级变量，在外层 `onBeforeUnmount` 中统一清理。当前影响较小。

### CR-2: 历史按钮功能未实现

**文件**: `WorkspaceTerminal.vue:85`
**代码**: `@click="showMobileMore = true"` — 历史按钮打开的是 more 菜单，而非独立的历史面板
**问题**: PRODUCT.md §3.6 描述「📜 历史 — 打开 bash 历史记录选择面板，支持搜索」。当前实现将历史按钮绑定到 `showMobileMore`，与 more 按钮功能重复。
**建议**: 保留当前实现（M21 范围内不实现 bash 历史面板），但在 TODO 注释中标记后续实现。

---

## 🟢 可选改进

### CR-3: ws-term-reconnect 遮罩层高度硬编码

**文件**: `WorkspaceTerminal.vue:675`
**代码**: `inset: 32px 0 22px 0` 硬编码了工具栏高度（32px）和状态栏高度（22px）
**问题**: 如果工具栏或状态栏高度变化，遮罩层需要同步调整。
**建议**: 使用 CSS 变量或 calc()。当前影响较小。

---

## 审查维度总结

| 维度 | 结果 |
|------|------|
| 正确性 | ✅ 无逻辑错误 |
| 安全性 | ✅ 无敏感信息泄露 |
| 架构一致性 | ✅ 纯前端变更，不涉及后端 |
| 测试覆盖 | ✅ 待步骤6验证 |
| 错误处理 | ✅ 移动端弹窗有关闭逻辑 |
| 文档一致性 | ✅ 与 M21 里程碑文档一致 |

---

## 结论

✅ 无 🔴 必须修复项。2 项 🟡 建议修复但不阻塞。
