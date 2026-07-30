# M73: Test Coverage & Integration Tests

## Context
M72 完成质量与文档。本里程碑聚焦提升测试覆盖率和添加集成测试，为生产发布建立质量保障。

版本类型：patch（测试/文档，无新功能）

## 产品边界
本阶段做什么：
- Rust 后端测试覆盖率提升（目标 90%）
- 前端单元测试补充
- 集成测试（API 端点测试）
- 测试 CI 覆盖率报告

本阶段不做什么：
- 不新增功能
- 不改变产品架构
- 不修改数据库 schema

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Rust 后端测试覆盖率提升 | ⬜ |
| 2 | 前端单元测试补充 | ⬜ |
| 3 | API 集成测试 | ⬜ |
| 4 | CI 覆盖率报告 | ⬜ |

## 子任务详细设计

### 1 Rust 后端测试覆盖率提升

- **功能目标**：提升后端测试覆盖率
- **文件结构**（修改）：
  - `crates/rex-hub/src/auth.rs` — 补充 token 刷新、过期、无效 token 测试
  - `crates/rex-hub/src/middleware.rs` — CSRF + Cache-Control 中间件测试
  - `crates/rex-hub/src/audit_api.rs` — 安全报告 API 测试
  - `crates/rex-hub/src/env_api.rs` — 环境配置导出测试
  - `crates/rex-hub/src/backup_api.rs` — 备份 API 测试
- **测试标准**：cargo test 全部通过
- **提交信息**：`test: improve Rust backend test coverage`

### 2 前端单元测试补充

- **功能目标**：补充前端关键组件测试
- **文件结构**（创建/修改）：
  - `packages/rex-console-web/src/components/__tests__/TokenRefreshModal.test.ts`
  - `packages/rex-console-web/src/composables/__tests__/useSwipeGesture.test.ts`
  - `packages/rex-console-web/src/composables/__tests__/useVirtualKeyboard.test.ts`
  - `packages/rex-console-web/src/api/__tests__/client.test.ts`
- **测试标准**：bun test 全部通过
- **提交信息**：`test: add frontend unit tests for new components`

### 3 API 集成测试

- **功能目标**：端到端测试 API 端点
- **文件结构**（创建）：
  - `tests/api/auth_test.rs` — 认证端点测试（login/check/refresh）
  - `tests/api/environments_test.rs` — 环境 CRUD 测试
  - `tests/api/audit_test.rs` — 审计日志测试
- **测试标准**：启动测试服务器，发送 HTTP 请求验证响应
- **提交信息**：`test: add API integration tests`

### 4 CI 覆盖率报告

- **功能目标**：在 CI 中生成覆盖率报告
- **文件结构**（修改）：
  - `.github/workflows/ci.yml` — 添加覆盖率步骤
- **实现**：在 lint job 中添加 `cargo llvm-cov` 覆盖率检查
- **提交信息**：`ci: add test coverage reporting`

## 设计核对点

- 测试不改变功能行为
- 测试覆盖率可量化验证
- 集成测试覆盖关键 API 路径

## Flow Status

- [x] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
- [ ] 步骤3：开发
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
