# Step 4: Code Simplification Report — 0.84.1

## 变更范围

| 文件 | 变更 |
|------|------|
| `useSqliteSession.ts` | +12 lines（closedByError 标志 + onclose guard） |
| `agent.rs` | +53 lines（is_last_seen_fresh 函数、list_agents 新鲜度检查、config_json、测试） |
| `env.rs` | ~8 lines（agent_online SQL 查询加 freshness 条件） |
| `AgentCard.vue` | +10 lines（未配置提示） |
| `AgentStatusPanel.vue` | +10 lines（未配置提示） |
| `agent.ts` | +1 line（config_json 字段） |
| `zh.ts` / `en.ts` | 各 +1 line（i18n key） |

## 精简检查

| 维度 | 结论 |
|------|------|
| 无冗余代码 | ✅ 所有新增代码均有明确用途 |
| 无死代码 | ✅ 无未使用的导入或变量 |
| 无过度抽象 | ✅ `is_last_seen_fresh` 作为独立函数合理，可被列表和测试复用 |
| 风格一致 | ✅ 后端新鲜度检查使用 chrono，SQL freshness 使用 datetime() — 两种方式一致 |
| 无重复逻辑 | ✅ env.rs 的 agent_online 查询统一加了 `last_seen_at > datetime('now', '-3 minutes')` |

## 结论

精简不改变功能行为。无回退改动。
