# M41 代码审查报告

## 审查范围

M41 变更文件 + 本次路由前缀重构：

| 文件 | 变更内容 |
|------|----------|
| `crates/rex-hub/src/resource_api.rs` | 路由前缀改 `/api/resources`，Path 参数名统一 |
| `crates/rex-hub/src/rex-hub.rs` | resource_routes 独立嵌套 |
| `crates/rex-hub/src/audit_api.rs` | 统计端点 |
| `crates/rex-hub/src/db.rs` | query_audit_stats 方法 |
| `crates/rex-hub/src/models.rs` | AuditStats 结构体 |
| `packages/rex-console-web/src/api/resources.ts` | 前端路径同步 |
| `packages/rex-console-web/src/pages/AgentsPage.vue` | 部署指南 + 配置弹窗 |
| `packages/rex-console-web/src/pages/AuditLogPage.vue` | 统计卡片 + 行展开 + CSV 导出 |
| `packages/rex-console-web/src/api/audit.ts` | 统计 API |
| `packages/rex-console-web/src/api/environments.ts` | agent_token 字段 |
| `packages/rex-console-web/src/i11n/locales/en.json` | i18n |
| `packages/rex-console-web/src/i18n/locales/zh.json` | i18n |

## 审查维度

### 1. 正确性

- 🟢 **SQL 参数化**：`query_audit_log` 和 `query_audit_stats` 全部使用 `?{idx}` 占位符，LIMIT/OFFSET 为 `Option<u64>` 数字类型，无注入风险
- 🟢 **路由注册**：所有同路径多方法路由使用链式调用（`.get().post().put().delete()`），无 405 风险
- 🟢 **资源路由前缀**：`/{env_id}` 匹配单段，`/{env_id}/{rid}` 匹配双段，Axum 按段数区分无冲突
- 🟢 **test-connection 路由**：literal 路由 `/api/resources/test-connection` 在 `protected_routes` 顶层注册，不会被 `/{env_id}` 吞掉
- 🟢 **前端路径同步**：`resources.ts` 5 个方法路径全部更新，无其他文件引用旧路径

### 2. 安全性

- 🟢 **SQL 注入防护**：参数化查询
- 🟢 **路由隔离**：资源路由独立于环境路由，不会误匹配

### 3. 架构一致性

- 🟢 **路由组织**：资源路由独立命名空间 `/api/resources`，语义清晰
- 🟢 **前端 API 层**：函数签名不变，仅内部路径变更，调用方无需修改

### 4. 错误处理

- 🟢 所有 handler 使用统一的 `ApiResult<T>` 和 `err()` 函数

### 5. 与里程碑文档一致性

- 🟢 Agent 部署指南 + 配置弹窗 → 实现完整
- 🟢 审计日志增强（统计、行展开、CSV 导出、时间筛选）→ 实现完整
- 🟢 资源路由前缀调整 → M41 过程中的合理变更

## 结论

✅ 通过，无 🔴 必须修复项。

🟡 **建议改进**（非阻塞）：
- `query_audit_log` 和 `query_audit_stats` 的 filter 拼接逻辑重复，可提取公共函数。但这是预有模式，不在本次变更范围。
- 所有受保护的 API 路由目前分散在多个 `.nest()` 调用中，可考虑统一抽取到单一的 `/api` nest 下，然后在公开/受保护层分发。这属于架构重构，不在 M41 范围内。
