# 步骤2 设计核对报告 — 0.84.1 Bug 修复与稳定性提升

## 核对范围
- 里程碑文档：`docs/milestones/0.84.1-bug-fixes.md`
- 产品文档：`docs/PRODUCT.md` §2.2 Agent 状态定义
- 约定文档：`AGENTS.md` 硬性约束与质量门禁
- 代码现状：`crates/rex-hub/src/{ws_sqlite.rs, agent.rs, ws.rs}`、`packages/rex-console-web/src/features/sqlite/useSqliteSession.ts`、`features/agents/{AgentCard.vue, AgentStatusPanel.vue}`

## 审查维度逐项

### 1. 产品边界一致性
| 检查项 | 结论 | 说明 |
|--------|------|------|
| 修复范围符合 AGENTS.md 已知问题 #1、#7 | ✅ | 子任务1 对应「SQLite 连接失败时页面不显示错误」，子任务2 对应「未配置的 agent 直接显示在线」 |
| 无新功能引入（patch 定义） | ✅ | 仅修正错误展示与状态判定逻辑，无新 API/组件 |
| Agent 状态模型符合 PRODUCT.md 两态 | ⚠️→✅ | 初稿曾设计「未配置」第三状态（黄色徽章），已修正为：仍保持 🟢在线/⚫离线 两态，仅在 online 下追加「未配置」灰色小字提示 |

### 2. 方案技术可行性
| 检查项 | 结论 | 说明 |
|--------|------|------|
| 子任务1 根因正确 | ✅ | `useSqliteSession.ts` 的 `onclose` 确实用 "Connection lost..." 覆盖后端 `error` 消息；引入 `closedByError` 标志可保留真实错误并禁止自动重连 |
| 子任务2 根因正确 | ✅ | DB `status` 字段被硬编码为 online，`last_seen_at` 由 `update_heartbeat` 在 WS 连接/心跳时刷新；基于新鲜度（3 分钟窗口）判定真实状态可消除 Hub 重启残留的虚假 online |
| 修复不影响现有自动重连 | ✅ | 子任务1 仅在 `closedByError` 时禁用重连，正常断开仍按原逻辑重连 |
| 前端向后兼容 | ✅ | 子任务2 仅依赖现有 `last_seen_at` / `config_json` 字段，无需新增字段或迁移 |

### 3. 测试覆盖设计
| 检查项 | 结论 | 说明 |
|--------|------|------|
| 后端有单元测试 | ✅ | 子任务2 `list_agents` 对 `last_seen_at` 过期返回 offline 的用例 |
| 前端有测试或验证 | ✅ | 子任务1 有 `useSqliteSession` 错误保留的单测；子任务2 前端变更极小，手动验证 |
| 质量门禁对齐 AGENTS.md | ✅ | 子任务3 明确 `cargo test` / `cargo clippy` / `bun run {type-check,lint,build}` |

### 4. 子任务拆分粒度
| 检查项 | 结论 | 说明 |
|--------|------|------|
| 1-2 个 commit 粒度 | ✅ | 子任务1（前端）、子任务2（后端+前端提示）、子任务3（测试）均独立可提交 |

## 修订记录
初稿子任务2 将「未配置」作为第三种状态（黄色徽章），与 PRODUCT.md §2.2 严格的 🟢在线/⚫离线 两态定义冲突。已修正为：
- 后端：基于 `last_seen_at` 新鲜度动态判定 online/offline，消除 DB 残留的虚假 online
- 前端：仅当 online 且 `config_json` 为空时显示「⚠ 未配置」灰色小字，不引入第三状态

## 结论
✅ **通过** — 里程碑文档设计合理，修复方案根因准确、符合产品边界与质量门禁，子任务拆分粒度恰当。
