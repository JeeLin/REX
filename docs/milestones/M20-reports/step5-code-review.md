# M20 步骤5：代码审查报告（第二次）

## 审查范围

M20 共 3 个子任务，涉及 8 个文件变更。审查聚焦前端源码。

---

## 🔴 必须修复

无。

CR-1（路由名不存在）已在本轮修复：`router.push({ name: 'resource' })` → `router.push({ name: 'environments' })`。资源详情页待后续里程碑实现。

---

## 🟡 应该修复

### CR-2: 未使用的 i18n 键

**文件**: `i18n/zh.ts:88-89`, `i18n/en.ts:88-89`
**代码**: `ws.panel.dropHere` 和 `ws.panel.swap`
**问题**: 这两个键在模板中未被引用。面板拖拽的 drop zone 仅使用 CSS 类提供视觉反馈。
**建议**: 保留（后续可在 drop zone 添加文字提示）。

### CR-3: navigator.clipboard.writeText 未处理异常

**文件**: `WorkspaceTerminal.vue:348`
**代码**: `navigator.clipboard.writeText(...)` 在无 HTTPS 或权限被拒绝时会抛出异常。
**建议**: 添加 try-catch。当前影响较小（用户主动触发）。

---

## 🟢 可选改进

### CR-4: onPanelDragOver 中 dragOverPanel 可能闪烁

**文件**: `Workspace.vue:219-223`
**问题**: `@dragleave` 在子元素间移动时也会触发，造成高亮闪烁。当前视觉影响较小。

---

## 审查维度总结

| 维度 | 结果 |
|------|------|
| 正确性 | ✅ CR-1 已修复 |
| 安全性 | ✅ 无敏感信息泄露 |
| 架构一致性 | ✅ 使用现有 composable，符合单用户模型 |
| 测试覆盖 | ✅ 待步骤6验证 |
| 错误处理 | 🟡 CR-3 clipboard 未处理异常 |
| 文档一致性 | ✅ 与 M20 里程碑文档一致 |

---

## 结论

✅ 无 🔴 必须修复项。2 项 🟡 建议修复但不阻塞。
