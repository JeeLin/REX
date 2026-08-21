# 代码精简：M55 v0.47.0

## 变更文件

- `crates/rex-hub/src/migrations.sql` — 新增 registration_token 列，移除 agent_token 列
- `crates/rex-hub/src/models.rs` — Environment 新增 registration_token，EnvironmentDetail 移除 agent_token
- `crates/rex-hub/src/db.rs` — 新增 find_environment_by_registration_token、find_agent_by_env_id
- `crates/rex-hub/src/agent_ws.rs` — 认证流程改用注册令牌，自动创建 Agent
- `crates/rex-agent/src/agent_ws.rs` — auth 消息新增 name 字段
- `packages/rex-console-web/src/pages/EnvironmentDetailPage.vue` — 使用 registration_token
- `packages/rex-console-web/src/pages/AgentsPage.vue` — 使用 registration_token
- `packages/rex-console-web/src/api/environments.ts` — 类型定义更新
- `packages/rex-console-web/src/stores/__tests__/environments.test.ts` — 测试数据更新

## 结论

✅ 变更简洁，无重复代码，无过度设计。
