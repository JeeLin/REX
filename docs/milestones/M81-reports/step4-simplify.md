# 步骤4：代码精简 — M81（重跑）

> 因步骤5 打回修复 🔴（`lastFocusedPaneId` 悬空）后重跑步骤4→5→6→7。

## 检查范围

基准：`cfefa782`（M81 首个提交父提交）。当前 `HEAD` 相对基准的全部非测试源文件（含步骤5 打回修复后的 `usePaneLayout.ts`）。

检查维度：重复代码、过度设计、提前实现、workspace 结构、大文件拆分、行为不变。

## 审查结论

| # | 维度 | 结论 | 说明 |
|---|------|------|------|
| 1 | 重复代码 | ✅ | `nextTabId()` 已作为唯一 tab id 生成入口（useTabs 导出 + WorkspacePage Ctrl+T 复用）；initScript 两段解析仅 2 处各 4 行，判定可接受不抽取。 |
| 2 | 过度设计 | ✅ | `lastFocusedPaneId` 同步逻辑为最小赋值（2 处各 1 行），非新状态机。 |
| 3 | 提前实现 | ✅ | 未涉及 M82 能力。 |
| 4 | workspace 结构 | ✅ | Rust `workspace = true`、前端功能域组织均保持。 |
| 5 | 大文件拆分 | ✅ | 无新增超大文件。 |
| 6 | 行为不变 | ✅ | 本步骤仅做组织/复用整理；🟡/🔴 行为修复已在步骤3 修复提交中完成。 |

## 本轮精简（行为不变，已提交）

- 导出 `nextTabId()`，`WorkspacePage.vue` Ctrl+T 复用，消除 `tab-${Date.now()}` 手写分支。
- `AuditLogPage.vue` 直接绑定已为 `{label,value}` 的 `pageSizeOptions`，移除冗余 `.map()`。

## 汇总

- 需修改项：无（基准精简已落地并验证通过）。
- 精简不改变功能行为，符合门禁。
