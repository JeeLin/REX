# M72: Quality & Documentation

## Context
M71 完成 i18n 全面补全。本里程碑聚焦产品质量提升和文档完善，为生产发布做准备。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段做什么：
- 自动化测试覆盖率提升
- API 文档生成
- 部署指南完善
- Bug 修复

本阶段不做什么：
- 不新增功能
- 不改变产品架构
- 不修改数据库 schema

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Rust 后端单元测试补充 | ✅ |
| 2 | API 文档（OpenAPI/Swagger） | ✅ |
| 3 | 部署指南更新 | ✅ |
| 4 | 已知 Bug 修复 | ✅ |

## 子任务详细设计

### 1 Rust 后端单元测试补充

- **功能目标**：提升后端测试覆盖率
- **文件结构**（修改）：
  - `crates/rex-hub/src/auth.rs` — 补充 token 刷新测试
  - `crates/rex-hub/src/audit_api.rs` — 补充安全报告测试
  - `crates/rex-hub/src/middleware.rs` — 补充 CSRF 中间件测试
- **测试标准**：cargo test 全部通过
- **提交信息**：`test: add unit tests for auth, audit, and middleware`

### 2 API 文档

- **功能目标**：生成所有 API 端点的文档
- **文件结构**（创建）：
  - `docs/reference/api-endpoints.md` — API 端点参考文档
- **内容**：
  - 所有 REST 端点列表（路径、方法、参数、响应）
  - WebSocket 端点说明
  - 错误码参考
- **提交信息**：`docs: add API endpoints reference documentation`

### 3 部署指南更新

- **功能目标**：完善自托管部署文档
- **文件结构**（修改）：
  - `docs/deployment/README.md` — 更新部署指南
- **内容更新**：
  - Docker Compose 配置示例（使用 REX_DATA_DIR）
  - 环境变量完整列表
  - 备份与恢复步骤
  - 常见问题解答
- **提交信息**：`docs: update deployment guide with env vars and backup`

### 4 已知 Bug 修复

- **功能目标**：修复 M65-M71 期间发现的 bug
- **修复范围**：
  - Token 刷新弹窗在登录页的误触发（已修复 checkAuth）
  - CSRF 中间件对 OPTIONS 请求的处理
  - 环境变量命名不一致（DATA_DIR → REX_DATA_DIR，已修复）
- **提交信息**：`fix: resolve known bugs from M65-M71`

## 设计核对点

- 测试不改变功能行为
- 文档准确反映当前实现
- Bug 修复不影响现有功能

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

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
