# 代码审查：M80

## 变更概览

- **变更文件**：9（6 代码 + 1 文档 + 1 里程碑文档 + 1 报告）
- **审查时间**：2026-08-14

## 问题列表

### #11 分栏方向递归渲染（PaneNode.vue / usePaneLayout.ts / WorkspacePage.vue）
| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 1 | 🟢 | PaneNode.vue | 15-26 | 每个容器节点用自身 `node.direction` 决定 `Splitpanes :horizontal`，递归渲染子节点。逻辑正确、无重复。仅样式/结构建议（已足够清晰）。 |

### #12 SSH 日志补充可读名称（terminal_ws.rs:131）
| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 2 | 🟢 | terminal_ws.rs | 131 | `name = %conn_info.name` 字段新增。需用 `conn_info` 在该作用域已可用（已确认 `conn_info` 在函数内持有），无空值风险。 |

### #13 右下角快捷键指南 FAB（WorkspacePage.vue）
| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 3 | 🟢 | WorkspacePage.vue | 462-471 | 独立 FAB 按钮，复用已有的 `showShortcuts` 状态与 `ShortcutPanel`。使用设计 token（--bg-elevated/--border/--text-secondary/--accent），符合设计系统。`z-index: 70` 层级合理。无安全问题。 |

### #14 审计 stats 支持 result 过滤（audit_api.rs / db.rs / audit.ts / AuditLogPage.vue）
| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 4 | 🟢 | audit_api.rs | 64/77 | `AuditStatsQuery.result` 透传给 `AuditFilter.result`，与 action/environment_id 处理一致，参数化绑定无 SQL 注入风险。 |
| 5 | 🟢 | db.rs | 240-243 | `result` 过滤沿用既有 `Box<dyn ToSql>` 参数化拼接（`?{idx}`），与其它过滤字段完全同构，安全。 |
| 6 | 🟢 | audit.ts / AuditLogPage.vue | 46 / 160 | 前端仅做 `resultFilter.value || undefined` 透传，未做额外处理，正确。 |

### #15 agent 部署文档精简（docs/agent-deploy.md）
| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 7 | 🟢 | agent-deploy.md | 11-21 | **修正此前错误**：源码 `agent_ws.rs` 仅读取 `REX_HUB_URL`/`REX_AGENT_TOKEN`（186/188 行），`REX_AGENT_ID` 从未被读取，Hub 在认证时依据 token 自动分配 agent_id（agent_ws.rs:219-293）。文档现正确声明 Agent ID 无需客户端配置；故障排查表 `auth failed: invalid registration token`（agent_ws.rs:207）亦已校准。 |

### #16 手机端全局查询回车换行（GlobalQueryModal.vue）
| # | 严重程度 | 文件 | 行号 | 描述 |
|---|----------|------|------|------|
| 8 | 🟢 | GlobalQueryModal.vue | 130 | `@keydown.enter.exact.prevent="execute"` 阻止移动端软键盘回车插入换行，改为执行。`.exact` 保证组合键（如 Ctrl+Enter 换行）不受影响。与 MobileTerminalBar 的 `@click="terminal?.paste('\r')"` 互不冲突。 |

## 汇总

- 🔴 必须修复：0
- 🟡 应该修复：0
- 🟢 可选改进：0（8 项均为已确认正确的最小改动）
- **结论：✅ 通过（无 🔴 必须修复项）**
