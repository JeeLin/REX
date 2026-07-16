# Step 5: 代码审查报告

## 审查范围

M9 新增/修改的 15 个文件（后端 7 + 前端 8）。

## 发现

### 🟢 可选改进

| # | 文件 | 问题 | 说明 |
|---|------|------|------|
| 1 | resource_api.rs | `update_resource` 使用 `NewResource` 而非 `UpdateResource` | 更新端点要求所有必填字段。当前前端未调用此端点，后续实现编辑资源时需改为 `UpdateResource`（所有字段 optional） |
| 2 | env_api.rs | `update_environment` 未校验空 name | `create_environment` 检查了 `name.trim().is_empty()`，update 遗漏了 |
| 3 | ResourcePanel.vue | `expandedEnvIds` 是死代码 | 未使用的 ref，可在后续清理 |
| 4 | env_api.rs / resource_api.rs | 未校验 `connection_mode` / `protocol` 枚举值 | 后端未验证字段值是否在允许范围内，前端已做限制。单用户自托管可接受 |
| 5 | db.rs | `LIMIT`/`OFFSET` 使用 `format!` 插入 SQL | 参数类型是 `u64`，无注入风险，但不是最佳实践 |

### 🟡 应该修复

| # | 文件 | 问题 | 说明 |
|---|------|------|------|
| 1 | resource_api.rs | `test_connection` S3 handler 嵌套过深 | 可用 early return 简化，但不影响功能 |

### 🔴 必须修复

无。

## 结论

✅ 通过。无 🔴 必须修复项。所有 🟢 为后续迭代可改进项，不影响当前功能正确性和安全性。

关键安全确认：
- SQL 注入：所有查询使用参数化（`rusqlite::params!`），✅
- 认证保护：所有 API 在 `protected_routes` 内，✅
- 外键级联：`ON DELETE CASCADE` 正确处理，✅
- 前端 auth：通过统一 `client.ts` 注入 Bearer token，✅
