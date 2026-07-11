# 步骤7：设计再确认报告 — 0.83.0 前端 UI/UX 重新设计

日期：2026-07-10

## 设计核对点逐项确认

| # | 核对点 | 结论 | 说明 |
|---|---|---|---|
| 1 | 设计令牌一致性：消除 phantom tokens、重复语义令牌 | ✅ | Step 3 Subtask 1 已新增语义别名（`--bg-panel`/`--bg-deep`/`--border`/`--text-*`）、组件令牌层、间距/字阶、浅色主题兼容；`variables.css` 无未定义引用 |
| 2 | 响应式断点完整性：移动/平板/桌面三档 | ✅ | `AppLayout.vue` 增加 `@media (min-width: 768px) and (max-width: 1024px)` 平板档；移动 `<768px`、桌面 `>1024px` 已覆盖 |
| 3 | 移动端 100vh 溢出 | ✅ | `Terminal.vue`/`SqlConsole.vue`/`Files.vue` 独立路由页 `100vh`→`100%`，避免与底部导航重叠 |
| 4 | 交互一致性：模态动画统一、错误反馈不静默、Toast 统一 | ✅ | 共享 `<Transition name="modal">`；`components.css` 统一 `modalIn`/`spin` keyframes；`ResourceEditModal`/`Environments`/Agent 模态框已加 toast 错误反馈（Step 3 Subtask 3） |
| 5 | 组件库规范：按钮/表单/表格/卡片/模态框样式统一 | ✅ | 组件库样式集中于 `base.css` + `components.css`，令牌驱动 |
| 6 | 无障碍：焦点陷阱、Esc 关闭、reduced-motion 守卫 | ⚠️→✅ | 见下方「补齐项」 |

## 补齐项（本步新增，满足核对点 6）

设计再确认时发现：**核对点 6 三项中，reduced-motion 已全局具备，但焦点陷阱与 Esc 关闭未在所有模态框落地**。按交付契约，命名验收项不得被静默缩水，故本步补齐，且严格沿用既有仓库约定（`EnvironmentEditModal`/`ConfirmDialog` 已有的内联 `trapFocus` + `aria-modal` + 初始/恢复焦点模式），不引入第二套约定。

### 修改清单
- `src/components/ResourceEditModal.vue`
  - `.modal-content` 加 `ref="dialogEl"` `role="dialog"` `aria-modal="true"` `:aria-labelledby="titleId"` `@keydown.tab="trapFocus"` `@keydown.esc="close"`
  - 标题 `span` 加 `:id="titleId"`；name 输入加 `ref="firstFieldEl"`
  - 脚本加 `useId` 引入、`titleId`/`dialogEl`/`firstFieldEl`/`previousActive`、`watch` 焦点保存/恢复、`trapFocus`、`finally` 中加载完成后聚焦首字段
- `src/features/agents/AgentConfigModal.vue`：`.modal-panel` 加同组 a11y 属性 + 关闭按钮 `ref="closeBtnEl"` + 脚本焦点管理
- `src/features/agents/AgentLogModal.vue`：`.log-panel` 同组 + 关闭按钮 `ref` + 脚本焦点管理
- `src/features/agents/AgentResetTokenModal.vue`：`.confirm-panel` 同组 + 取消按钮 `ref` + 脚本焦点管理
- `src/components/ConfirmDialog.vue`：已有 `trapFocus`/`aria-modal`/焦点恢复，补 `@keydown.esc="$emit('cancel')"`
- `src/components/EnvironmentEditModal.vue`：已有焦点陷阱 + aria，补 `@keydown.esc="close"`

### reduced-motion
`src/styles/base.css:397-405` 已有全局 `@media (prefers-reduced-motion: reduce)` 守卫（将所有 `animation`/`transition` 时长压至 `0.01ms`），覆盖组件 scoped keyframes（含 `components.css` 的 `modalIn`/`spin`）。✅ 不需改动。

## 质量门禁复测
| 检查项 | 命令 | 结果 |
|---|---|---|
| 类型检查 | `bun run type-check` | ✅ 0 错误 |
| Lint | `bun run lint` | ✅ 0 parse/syntax error（仅存量 warning） |
| 构建 | `bun run build` | ✅ 成功产出 dist |
| 单元测试 | `bun run test` | ⚠️ 1 失败 / 292 通过（唯一失败为存量 `SqlResults.test.ts` 断言错位，非本里程碑引入，详见 step6-test.md） |

## 结论
全部 6 项设计核对点均已满足。核对点 6 的焦点陷阱与 Esc 关闭 gaps 已在本步补齐并复测通过（type-check/lint/build 全绿，未引入新测试失败）。步骤7通过。
