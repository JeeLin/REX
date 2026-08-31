# M42: Axum 0.8 升级 + 路由参数修复

## Context

M41 完成了 Agent 部署指南和审计日志增强。当前项目使用 axum 0.7，存在路由参数冲突问题：在 `/api/environments` 嵌套下，`resource_api`（`/{env_id}/resources`）、`agent_api`（`/{env_id}/agents`）和 `env_api`（`/{id}`）的路径参数在 axum 0.7 的 matchit 路由器中产生歧义，导致 404/405 错误。升级到 axum 0.8 并同步升级周边依赖（tower 0.5、tower-http 0.6），解决路由参数冲突问题。

版本类型：patch（依赖升级 + bug 修复），版本号 0.38.0 → 0.38.1。

## 产品边界

**本阶段做：**
- 升级 axum 从 0.7 到 0.8
- 升级 tower 从 0.4 到 0.5
- 升级 tower-http 从 0.5 到 0.6
- 修复路由参数冲突导致的 404/405 问题
- 适配 axum 0.8 breaking changes（extractor、middleware、WebSocket）

**本阶段不做：**
- 新功能开发
- API 端点变更
- 前端代码修改
- 数据库 schema 变更

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 升级 workspace 依赖（axum/tower/tower-http） | ✅ |
| 2 | 修复 axum 0.8 breaking changes | ✅ |
| 3 | 验证编译和测试通过 | ✅ |

## 子任务详细设计

### 1 升级 workspace 依赖

**功能目标**

更新 `Cargo.toml` 中的 axum 相关依赖版本，确保版本兼容。

**文件结构**

修改：
- `Cargo.toml` — 更新 workspace.dependencies 中的 axum、tower、tower-http 版本

**依赖版本对照**

| 依赖 | 当前版本 | 目标版本 |
|------|----------|----------|
| axum | 0.7 | 0.8 |
| tower | 0.4 | 0.5 |
| tower-http | 0.5 | 0.6 |

**注意**：axum 0.8 默认使用 matchit 0.8，路由参数语法从 `{param}` 改为 `{param}`（语法不变，但匹配行为可能变化）。需验证 hyper/hyper-util 版本兼容性。

**测试标准**

- `cargo check` 通过
- `cargo clippy` 无新增 error

**提交信息**: `chore(deps): upgrade axum to 0.8, tower to 0.5, tower-http to 0.6`

### 2 修复 axum 0.8 breaking changes

**功能目标**

适配 axum 0.8 的 API 变更，修复路由参数冲突。

**文件结构**

修改：
- `crates/rex-hub/src/rex-hub.rs` — 适配 Router API 变更
- `crates/rex-hub/src/middleware.rs` — 适配 middleware API 变更
- `crates/rex-hub/src/agent_ws.rs` — 适配 WebSocket API 变更
- `crates/rex-hub/src/terminal_ws.rs` — 适配 WebSocket API 变更
- `crates/rex-hub/src/tunnel_ws.rs` — 适配 WebSocket API 变更
- `crates/rex-hub/src/resource_api.rs` — 修复路由参数冲突
- `crates/rex-hub/src/agent_api.rs` — 修复路由参数冲突
- `crates/rex-hub/src/env_api.rs` — 修复路由参数冲突

**axum 0.8 Breaking Changes**

1. **Router API**：`.with_state()` 移至 Router 构建最后，State 提取器不再需要泛型
2. **Middleware**：`from_extractor_with_state` 改为 `from_extractor`，State 通过 `.with_state()` 注入
3. **WebSocket**：`WebSocketUpgrade` 提取器签名变化
4. **Path extractor**：路由参数匹配行为变化，需验证 `{env_id}` 与 `{id}` 冲突是否解决

**路由参数冲突修复策略**

当前冲突：`/api/environments` 下合并了三个 Router：
- `resource_routes`: `/{env_id}/resources`
- `env_agent_routes`: `/{env_id}/agents`  
- `env_routes`: `/`, `/export`, `/import`, `/{id}`

冲突原因：`/{env_id}` 与 `/{id}` 在同一层级产生歧义。

修复方案：将 `resource_routes` 和 `env_agent_routes` 的路由前缀改为更明确的路径，或在 `build_router` 中调整嵌套结构，避免路径参数歧义。

**测试标准**

- `cargo check` 通过
- `cargo clippy` 无 error
- `cargo test --workspace` 全部通过
- 手动验证：GET/POST/PUT/DELETE `/api/environments/{id}/resources` 返回正确响应

**提交信息**: `fix(hub): adapt axum 0.8 breaking changes and fix route parameter conflicts`

### 3 验证编译和测试通过

**功能目标**

确保升级后所有质量门禁通过。

**检查项**

- `cargo fmt --check` — 格式检查
- `cargo clippy --workspace --all-targets` — Lint 检查
- `cargo test --workspace` — 单元测试
- `bun run type-check` — 前端类型检查
- `bun run lint` — 前端 Lint
- `bun run build` — 前端构建

**测试标准**

- 所有检查通过，无 error
- 前端构建成功

**提交信息**: 无（合并到子任务2提交）

## 设计核对点

- ✅ 不引入多用户、RBAC、企业协作概念
- ✅ 依赖升级不改变 API 端点和数据模型
- ✅ 前端代码无需修改
- ✅ 所有测试通过

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交

## 打回记录

（打回时追加一条，创建里程碑文档时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
