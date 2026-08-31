# M8 Step 2 设计核对报告

## 审查对象

- 里程碑文档：`docs/milestones/M8-infrastructure.md`
- 产品文档：`docs/PRODUCT.md`
- 参考文档：`docs/reference/data-models.md`、`docs/reference/api-design.md`
- 现有代码：`crates/rex-hub/src/*`、`packages/rex-console-web/src/*`

## 审查维度

| # | 维度 | 结论 | 说明 |
|---|------|------|------|
| 1 | 产品定位一致性 | ✅ PASS | 单用户、自托管，无多用户/RBAC |
| 2 | 架构一致性 | ✅ PASS | 兼容现有 axum + Vue3 架构 |
| 3 | 数据模型合理性 | ✅ PASS | 对齐 reference/data-models.md，新增 settings 表 |
| 4 | 认证设计合理性 | ✅ PASS | argon2 + JWT 单用户认证，首次设置流程完整 |
| 5 | API 设计一致性 | ✅ PASS | 对齐 reference/api-design.md 错误格式和认证方式 |
| 6 | 前端设计一致性 | ✅ PASS | 兼容现有 Pinia + Vue Router 结构 |
| 7 | 子任务拆分合理性 | ✅ PASS | 7 个子任务，紧耦合的已合并 |
| 8 | 安全考虑 | ✅ PASS | JWT secret 管理、WebSocket token auth、set_password 保护 |
| 9 | 向后兼容 | ✅ PASS | 现有 API 模块注入 auth header，不中断现有功能 |
| 10 | 是否跳阶段 | ✅ PASS | 仅建表和基础设施，不实现业务逻辑 |

## 发现并修复的问题

### 🔴 阻塞问题（已修复）

1. **前端 ApiClient 双重前缀**：auth store 中 API 路径带 `/api/` 前缀，但 client.ts 已有 `baseUrl = '/api'`，导致 `/api/api/auth/...` 404。**修复**：auth store 路径改为 `/auth/check`、`/auth/login`、`/auth/password`。

2. **现有前端 API 模块不发 auth header**：sql.ts、redis.ts、files.ts 使用原始 fetch 无 Authorization header，M8 后全部 401。**修复**：新增 Task 3，要求在 M8 中为现有 API 模块注入 auth header。

### 🟡 重要问题（已修复）

3. **WebSocket 无法传自定义 header**：浏览器 WebSocket API 不支持自定义 header，终端连接会断。**修复**：采用 query param 方案 `?token=xxx`。

4. **r2d2_sqlite 未在 workspace 声明**：需要添加到 workspace dependencies。**修复**：补充依赖声明。

5. **set_password 无二次调用保护**：密码已设置后可被覆盖。**修复**：handler 检查 `requires_setup()` 后返回 409。

6. **Tasks 2/3 紧耦合**：auth 中间件需要 AppState，AppState 路由需要 auth 中间件。**修复**：合并为一个 Task 2。

7. **error_response 固定返回 400**：auth 错误应返回 401。**修复**：新增 `auth_error_response` 函数。

### 🟢 可选改进（已记录）

8. **密码最低长度 6 位偏弱**：记录为已知 tradeoff，后续可调整。

9. **JWT secret 存储在数据库中**：记录为已知 tradeoff，适合自托管场景。

## 结论

**✅ 通过**

所有阻塞性问题已修复，重要问题已补充方案。里程碑文档可以进入开发阶段。
