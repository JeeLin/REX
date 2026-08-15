# 代码审查：M81（重跑）

## 变更概览

- **变更文件**：18 个非测试源文件（Rust 8 + 前端 10）+ 步骤5 打回修复 `usePaneLayout.ts`
- **审查维度**：内置默认（正确性 / 安全性 / 健壮性 / 可维护性 / 性能 / 规范）
- **基准**：`git diff cfefa782 HEAD`

## 问题列表

| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 1 | 🟡 | `crates/rex-hub/src/sql_api.rs` | 466 | 注释仍写 `DELETE /api/sql/saved-queries/:id`（冒号语法），实际代码为 `{id}`。注释与实现不一致，仅文档性，不影响行为。 |
| 2 | 🟢 | `crates/rex-hub/src/update_checker.rs` | 73 | 若 GitHub tag 含预发布后缀（如 `v0.69.0-rc1`），`compare_version` 的段 `parse::<u32>()` 失败回落 0，比较可能偏旧。正式 Release 通常无后缀，属边缘场景。 |

> 注：原 🔴（`lastFocusedPaneId` 悬空致分栏 no-op）已在打回后修复（提交 `2d317c8e`），`closePane`/`applyLayoutPreset` 末尾同步 `lastFocusedPaneId` 到有效 pane，并补充回归测试（usePaneLayout.test.ts 20 passed）。本轮不复计入。

## 汇总

- 🔴 必须修复：0
- 🟡 应该修复：1（注释与路由语法不一致）
- 🟢 可选改进：1（预发布后缀版本比较）
- **结论**：0 个必须修复项 → 通过

## 各维度结论

- 正确性：`update_checker.rs:318 compare_version` 逐段数值比较正确（`1.10.0 > 1.2.0`）；`check_for_update` 仅 `latest > current` 严格返回 `Some`，无降级（M81 #5 ✅）。`sql_api.rs:64` brace 语法 `{id}` 正确（M81 #9 ✅）。`usePaneLayout` `focusPane` 与 `closePane`/`applyLayoutPreset` 现一致维护 `lastFocusedPaneId`。
- 安全性：无 SQL 注入 / XSS；敏感字段仅内存使用。
- 健壮性：错误处理覆盖全；`deserialize` try/catch；`splitPane` 对 null target 早退。
- 可维护性：initScript 两段解析维持可接受。
- 性能：无新增 N+1 / 阻塞热路径。
- 规范：符合 Rust workspace 与 Vue 功能域约定。

🟡 注释不一致不阻塞；可在收尾时顺手修正（见步骤8 提交）。
