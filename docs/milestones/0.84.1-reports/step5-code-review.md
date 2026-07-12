# Step 5: Code Review Report — 0.84.1

## 变更概要

| 文件 | 变更内容 |
|------|---------|
| `useSqliteSession.ts` | closedByError 标志防止 onclose 覆盖错误消息 |
| `agent.rs` | is_last_seen_fresh 函数、list_agents 新鲜度检查、config_json 字段、测试 |
| `env.rs` | agent_online SQL 查询加 last_seen_at 新鲜度条件 |
| `AgentCard.vue` / `AgentStatusPanel.vue` | 在线但未配置时显示「⚠ 未配置」提示 |
| `agent.ts` | Agent 接口添加 config_json 字段 |
| `zh.ts` / `en.ts` | 添加 unconfigured i18n key |

## 审查结论

**无 🔴 必须修复项。**

## 发现

### 🟢 可选改进

1. **重复 CSS**：`.unconfigured-hint` 样式在 `AgentCard.vue` 和 `AgentStatusPanel.vue` 中重复定义。可提取到共享 CSS 文件，但项目约定使用 scoped styles，保持现状合理。

2. **重复 row.get**：`agent.rs` 的 `list_agents` 中 `last_seen_at` 通过 `row.get(10)` 读取了两次（一次用于新鲜度检查，一次用于 struct 构造）。可优化为读取一次复用，但影响微乎其微。

## 安全审查

- 无新用户输入面
- 无新 API 端点
- SQL 查询均使用参数化查询
- config_json 已在现有 API 中暴露，无新增信息泄漏
- 无敏感数据暴露

## 结论

✅ 代码审查通过，无 🔴 必须修复项。
