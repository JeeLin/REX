# 步骤2：设计核对报告 — 0.83.1 工作空间交互可靠性

**核对对象**：里程碑文档 `docs/milestones/0.83.1-workspace-interaction-reliability.md` vs 产品文档 `docs/PRODUCT.md` §3.5 / §3.6、开发文档 `docs/DEVELOPMENT.md`、约束文件 `AGENTS.md`。
**结论**：✅ 通过（补充一处模板字段，已在步骤1保持勾选下直接修正）。

---

## 核对维度

### 1. 产品边界一致性（里程碑文档 vs 产品文档）

| 子任务 | 产品文档依据 | 设计是否吻合 |
|---|---|---|
| 1 分屏拖拽 | §3.5「面板拖拽」：拖入空面板填充 / 已占用面板交换 / 单面板禁用 / `opacity:0.5` + 橙色虚线 `layout-drop-zone` | ✅ 详细设计的「交互设计」逐条对应产品文档规则 |
| 2 全局快捷键 | §3.5「快捷键面板 / 分屏布局 / 标签栏」：`Ctrl+K` 全局搜索、`Ctrl+N` 新建连接、`Alt+1~5` 布局、`F11` 全屏、`F1` 帮助 | ✅ 主体吻合；见下方「缺陷对齐」中 1 处展示文案需修正 |
| 3 SSH 终端复制粘贴 | §3.6 状态栏「`Ctrl+Shift+C` 复制」+ 右键菜单「复制 / 粘贴」 | ✅ 详细设计的交互规则与产品文档一致 |

### 2. 已知问题映射（AGENTS.md 已知问题清单）

| AGENTS.md 已知问题 | 里程碑子任务 | 映射 |
|---|---|---|
| #4 分屏和拖拽基本不可用 | 子任务1 | ✅ |
| #3 快捷键展示异常（landing page `Ctrl+K`/`Ctrl+N`/`Alt+1~5`/`F1`） | 子任务2 | ✅ |
| #5 SSH 终端复制粘贴与浏览器 `Ctrl+C/V` 冲突 | 子任务3 | ✅ |

三项均为 `AGENTS.md` 已知问题清单中的待修复项，范围闭环，无遗漏也无越界。

### 3. 版本类型判定

纯交互缺陷修复、不改变后端 API、不修改数据模型、不引入新功能 → **patch (0.83.1)**，与 `DEVELOPMENT.md` 规划一致。✅

### 4. 接口面 / 架构约束

- 复用现有 `useTabs`（`moveTabToPanel`/`swapPanels`/`reorderTab`）与 `rex:shortcut` 事件机制，不扩大公开接口面。✅
- 单用户、自托管、深色优先约束未被触及。✅
- 不修改后端代码、不修改数据库 schema。✅（全部改动限定在 `packages/rex-console-web/src`）

### 5. 符号与文件真实性核对（实际代码验证）

全部引用经 grep / 读取确认存在：

- `pages/Workspace.vue`（DnD `:422-448`、快捷键面板 `:246-272`、`onShortcut` `:722-731`）
- `features/workspace/TabBar.vue`、`features/workspace/useTabs.ts`（`moveTabToPanel` `:127`、`swapPanels` `:134`、`reorderTab` `:163`）
- `layouts/AppLayout.vue`（`onGlobalKeyDown` `:482-500`）
- `components/CommandPalette.vue`、`features/workspace/panels/WorkspaceTerminal.vue`（`attachCustomKeyEventHandler` `:284-313`）

**缺陷对齐（实施阶段须落实，非打回理由）**：

1. **`Ctrl+N` 展示与行为不一致（AGENTS.md #3 的根因之一）**：快捷键面板 `:251` 用 `ws.shortcuts.newTab` 文案，但实际 `onShortcut` 收到 `new-connection` 时弹出的是连接菜单（`showConnMenu`），而非新建标签。子任务2 须对齐文案或行为，使面板内容与绑定一致。
2. **`AppLayout.vue` 是唯一 `window` 级 keydown 监听**：`Workspace.vue` 的 `onKeyDown`（`:735-772`）仅作为模板 `@keydown` 绑定，作用域在 workspace 根元素，并非第二个全局监听。因此里程碑「收敛为单一全局入口」「Workspace.vue 不再 `window.addEventListener`」的建议在**当前代码下不适用**——文档已用「设计核对点」正确设为 guardrail（若实施发现已是模板作用域绑定，则不强行合并，避免回归）。核对结论是：以「消除触发/展示异常」为实际目标，而非机械合并监听器。
3. **子任务3 残留缺陷定位**：`WorkspaceTerminal.vue :284-313` 当前**已实现**预期逻辑（Ctrl+V→paste、Ctrl+C 有选中→复制/无选中→SIGINT、Ctrl+Shift+C→强制复制）。独立 `pages/Terminal.vue`（`terminal.html`）**未被 router / vite 任何入口引用**（死代码），已正确排除在范围之外。子任务3 实施时须先实测复现 AGENTS.md #5 的真实残留（paste 转发边界、移动端路径、或 xterm 默认 `Ctrl+C` 仍吞信号等），再据实测修复，不在已正确代码上重写。

### 6. 测试标准合理性

- 子任务1/2/3 均给出可执行的单测 / 组件 / 集成测试标准（vitest + @testing-library），覆盖关键分支（no-op 边界、焦点在 input 不触发、选中态返回值）。✅
- 子任务4 明确 `bun run type-check` / `lint` / `build` / `bun test` 门禁。✅

### 7. 模板完整性

原里程碑文档缺少 `## 设计核对点` 章节（模板必需字段）。已在保持步骤1勾选状态下直接补入该章节（含已发现的设计细节 guardrail），不触发打回。

---

## 最终结论

✅ 设计通过。子任务拆分粒度合理（1–2 commit/子任务），产品边界与已知问题对齐，patch 约束明确，引用符号真实。实施阶段须落实上述「缺陷对齐」3 项（尤其 #1 的 `Ctrl+N` 文案/行为一致性、#3 的先复现再修复）。可进入步骤3 开发。
