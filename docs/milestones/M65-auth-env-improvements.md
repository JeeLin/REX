# M65: Auth & Environment Improvements

## Context
M64 完成 Bug 修复与 UX 优化。本里程碑聚焦认证体验改进和环境变量文档化，提升自托管运维体验。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段做什么：
- 所有 `REX_` 环境变量的完整说明文档
- Token 过期时弹出密码输入 Modal 而非跳转登录页
- 延长 JWT Token 有效期（7 天 → 30 天）

本阶段不做什么：
- 不改变认证架构（仍为单用户 argon2 + JWT）
- 不新增 RBAC、多用户等概念
- 不修改 Agent 端认证逻辑

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 环境变量说明文档 | ✅ |
| 2 | Token 过期弹窗（密码输入 Modal 替代跳转登录页） | ✅ |
| 3 | 延长 JWT Token 有效期（7 天 → 30 天） | ✅ |

## 子任务详细设计

### 1 环境变量说明文档

- **功能目标**：为所有 `REX_` 环境变量创建完整的说明文档，方便自托管用户查阅
- **文件结构**（创建）：
  - `docs/reference/env-variables.md` — 环境变量完整文档
- **文档内容**：每个环境变量包含变量名、默认值、说明、使用场景、示例
- **涉及变量**（当前代码中共 19 个）：
  - `REX_PORT` — Hub 监听端口
  - `REX_DATA_DIR` — 数据存储目录
  - `REX_STATIC_DIR` — 前端静态资源目录
  - `REX_WORKER` — Worker 进程标识
  - `REX_AUTO_UPDATE` — 自动更新开关
  - `REX_HUB_URL` — Hub 地址（Agent 连接用）
  - `REX_AGENT_NAME` — Agent 名称
  - `REX_AGENT_TOKEN` — Agent 认证令牌
  - `REX_HEARTBEAT_INTERVAL` — 心跳间隔
  - `REX_TLS_CERT` — TLS 证书路径
  - `REX_TLS_KEY` — TLS 私钥路径
  - `REX_TLS_SELF_SIGNED` — 自签名证书开关
  - `REX_TLS_INSECURE` — 跳过 TLS 验证
  - `REX_ACME_DOMAIN` — ACME 域名
  - `REX_ACME_EMAIL` — ACME 邮箱
  - `REX_ACME_STAGING` — ACME 测试环境
  - `REX_UPDATE_GITHUB_OWNER` — 更新源 GitHub Owner
  - `REX_UPDATE_GITHUB_REPO` — 更新源 GitHub Repo
  - `REX_UPDATE_PENDING` — 待更新状态
- **提交信息**：`docs: add REX_ environment variables reference`

### 2 Token 过期弹窗

- **功能目标**：Token 过期时不跳转登录页，而是在当前页面弹出密码输入 Modal，输入密码后自动更新 Token 继续工作
- **文件结构**（修改）：
  - `packages/rex-console-web/src/api/client.ts` — 401 拦截改为触发弹窗事件
  - `packages/rex-console-web/src/components/TokenRefreshModal.vue` — 新建密码输入 Modal 组件
  - `packages/rex-console-web/src/stores/auth.ts` — 添加 Token 刷新方法
  - `packages/rex-console-web/src/App.vue` — 挂载 TokenRefreshModal
- **接口设计**：
  - 后端已有 `POST /api/auth/login`，可复用获取新 Token
  - 新增 `POST /api/auth/refresh`（可选）：验证当前密码后签发新 Token
- **交互设计**：
  1. API 请求返回 401 → 显示密码输入 Modal（半透明遮罩 + 居中卡片）
  2. 用户输入密码 → 调用 login API → 获取新 Token → 替换 localStorage 中的 Token
  3. 自动重试失败的请求
  4. 用户取消 → 跳转登录页
  5. 密码错误 → Modal 内显示错误提示，不关闭
- **后端流程**：
  - 可选：新增 `POST /api/auth/refresh` 端点，验证密码后签发新 Token
  - 或直接复用 `POST /api/auth/login`（密码验证 + 签发 Token）
- **测试标准**：
  - Token 过期后弹出 Modal 而非跳转
  - 输入正确密码后 Token 更新且请求自动重试
  - 输入错误密码后显示错误
  - 取消弹窗后跳转登录页
- **提交信息**：`feat: show token refresh modal instead of redirecting to login on 401`

### 3 延长 JWT Token 有效期

- **功能目标**：将 JWT Token 有效期从 7 天延长到 30 天，减少频繁过期
- **文件结构**（修改）：
  - `crates/rex-hub/src/auth.rs` — 修改 `Duration::days(7)` → `Duration::days(30)`
  - `crates/rex-hub/src/auth.rs` — login 和 set_password 返回的 `expiresAt` 同步更新
- **修改点**：
  - `generate_token()` 中 `chrono::Duration::days(7)` → `chrono::Duration::days(30)`
  - `login()` handler 中 `chrono::Duration::days(7)` → `chrono::Duration::days(30)`
  - `set_password()` handler 中 `chrono::Duration::days(7)` → `chrono::Duration::days(30)`
- **测试标准**：
  - 现有 `test_jwt_roundtrip` 仍通过
  - 新增测试验证 Token 有效期为 30 天
- **提交信息**：`feat(auth): extend JWT token expiry from 7 to 30 days`

## 设计核对点

- 单用户模式：Token 刷新不引入多用户概念
- 自托管优先：环境变量文档覆盖所有自托管配置项
- 安全性：Token 刷新 Modal 不降低安全标准（密码验证、审计日志）
- 后端依赖使用 `workspace = true`
- 前端按功能域组织

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
