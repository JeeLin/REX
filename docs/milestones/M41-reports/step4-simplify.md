# M41 代码精简报告

## 精简内容

1. **AgentsPage.vue**: 合并重复的 `envName` 和 `envNameById` 函数为一个 `envName`
2. **路由注册**: 修复所有同路径多方法的 `.route()` 注册，改为链式调用 `.get().post().put().delete()`，防止 405 错误
   - `resource_api.rs`: `/{id}/resources` GET+POST, `/{id}/resources/{rid}` GET+PUT+DELETE
   - `env_api.rs`: `/` GET+POST, `/{id}` GET+PUT+DELETE
   - `settings_api.rs`: `/` GET+PUT
   - `file_api.rs`: `/acl` GET+PUT
3. **audit_api.rs**: 修复 `environment_id: None` → `q.environment_id`（M40 遗留 bug）

## 精简结果

所有改动不改变功能行为，仅改善代码组织方式。

✅ 通过
