# M8 Step 5 代码审查报告

## 审查范围

M8 所有提交的代码变更：
- `crates/rex-hub/src/db.rs` — SQLite 数据库层
- `crates/rex-hub/src/models.rs` — 数据模型
- `crates/rex-hub/src/auth.rs` — JWT 认证
- `crates/rex-hub/src/middleware.rs` — AuthUser extractor
- `crates/rex-hub/src/error.rs` — 统一错误格式
- `crates/rex-hub/src/app.rs` — AppState
- `crates/rex-hub/src/rex-hub.rs` — 路由重构
- `crates/rex-hub/src/sql_api.rs` / `redis_api.rs` / `file_api.rs` — handler 适配
- `packages/rex-console-web/src/api/client.ts` — API 客户端
- `packages/rex-console-web/src/stores/auth.ts` — auth store
- `packages/rex-console-web/src/router/index.ts` — 路由守卫
- `packages/rex-console-web/src/pages/LoginPage.vue` / `SetupPage.vue` — 登录/设置页

## 审查发现

### 🔴 必须修复

无。

### 🟡 应该修复

1. **JWT secret 存储在数据库中** — 如果数据库文件被复制，攻击者可伪造 token。适合自托管场景，记录为已知 tradeoff。
2. **密码最低长度 6 位偏弱** — 对于管理远程基础设施的工具，建议 8 位。记录为已知 tradeoff。
3. **error_response 在各 API 模块中重复定义** — 应统一到 error.rs。不影响功能，后续清理。
4. **现有 API 模块（sql/redis/files）的 authHeaders 是复制粘贴** — 应提取到共享模块。不影响功能，后续清理。

### 🟢 可选改进

1. **clippy new_without_default** — SqlConnectionPool 等缺少 Default impl
2. **argon2 crate API** — 使用了 0.5 版本的新 API，代码正确但注释可以更清晰

## 结论

**✅ 通过** — 无 🔴 必须修复项。所有 🟡 项为已知 tradeoff 或后续清理项，不阻塞发布。
