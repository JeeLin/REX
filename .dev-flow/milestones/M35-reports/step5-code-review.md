# M35 Step 5: 代码审查报告

## 审查范围

M35 四个子任务 + 路由修复的全部代码变更。

## 发现

### 🔴 必须修复

| # | 文件 | 问题 |
|---|------|------|
| 1 | ShortcutPanel.vue | Escape 键不关闭面板 — 里程碑文档明确要求「Esc 关闭」 |

### 🟡 应该修复

| # | 文件 | 问题 |
|---|------|------|
| 2 | WorkspacePage.vue | propsResource.encoding 硬编码为 'UTF-8'，应读取 tab.encoding |
| 3 | 里程碑文档 | 文件名 ShortcutsPanel vs 实际 ShortcutPanel 不一致 |

### 🟢 可选改进

| # | 文件 | 问题 |
|---|------|------|
| 4 | ShortcutPanel.vue | 面板尺寸 420px 偏离设计稿 70vw×70vh |
| 5 | TerminalContextMenu.vue | 右键菜单靠近视口边缘时可能溢出 |

## 路由修复确认

- resource_routes 合并到 env_routes 的 nest 块 — ✅ 正确
- env_agent_routes 位于 env_routes 前 — ✅ 正确
- `cargo fmt --check` + `cargo clippy` — ✅ 通过

## 结论

✅ 所有🔴必须修复项已修复（Escape 键、encoding 硬编码）。审查通过。
