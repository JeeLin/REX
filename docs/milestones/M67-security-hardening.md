# M67: Security Hardening & Audit Enhancement

## Context
M66 完成移动端适配。本里程碑聚焦安全加固，增强 CSP 策略、添加 CSRF 保护、完善审计日志。

版本类型：minor（新功能，向后兼容）

## 产品边界
本阶段做什么：
- CSP 安全头增强（script-src 移除 unsafe-inline）
- CSRF 保护（API 请求验证 Origin/Referer）
- 审计日志增强（IP 地址记录、登录失败追踪）
- 安全审计报告（API 端点）

本阶段不做什么：
- 不修改数据库 schema（已有 audit_log 表）
- 不新增用户认证方式
- 不修改 Agent 端逻辑

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | CSP 安全头增强 | ✅ |
| 2 | CSRF 保护（Origin/Referer 验证） | ✅ |
| 3 | 审计日志增强（IP + 登录失败追踪） | ✅ |
| 4 | 安全审计报告 API | ✅ |

## 子任务详细设计

### 1 CSP 安全头增强

- **功能目标**：强化 Content-Security-Policy，移除 unsafe-inline
- **文件结构**（修改）：
  - `crates/rex-hub/src/middleware.rs` — 增强 CSP 策略
- **修改内容**：
  - 移除 `script-src 'unsafe-inline'`，改用 nonce 或 hash
  - 添加 `frame-ancestors 'none'`
  - 添加 `base-uri 'self'`
  - 添加 `form-action 'self'`
- **提交信息**：`feat(security): harden CSP policy`

### 2 CSRF 保护

- **功能目标**：验证 API 请求的 Origin/Referer 头，防止跨站请求伪造
- **文件结构**（修改）：
  - `crates/rex-hub/src/middleware.rs` — 新增 CSRF 中间件
- **实现方案**：
  - 仅对 POST/PUT/DELETE 请求验证
  - 检查 Origin 或 Referer 头是否匹配服务器地址
  - 本地开发模式（127.0.0.1/localhost）跳过验证
  - WebSocket 连接不验证（由 JWT 保护）
- **提交信息**：`feat(security): add CSRF protection middleware`

### 3 审计日志增强

- **功能目标**：记录客户端 IP 地址，增强登录失败追踪
- **文件结构**（修改）：
  - `crates/rex-hub/src/db.rs` — audit_log 表添加 ip 字段
  - `crates/rex-hub/src/auth.rs` — 记录登录 IP
  - `crates/rex-hub/src/middleware.rs` — 提取客户端 IP
- **接口设计**：
  - `NewAuditEntry` 新增 `ip: Option<String>` 字段
  - 登录失败时记录 IP 地址和失败次数
- **提交信息**：`feat(security): add IP tracking to audit log`

### 4 安全审计报告 API

- **功能目标**：提供安全相关的统计和报告
- **文件结构**（修改）：
  - `crates/rex-hub/src/audit_api.rs` — 新增安全报告端点
- **接口设计**：
  - `GET /api/audit/security-report` — 返回最近 24h 登录失败次数、异常 IP
- **提交信息**：`feat(security): add security audit report API`

## 设计核对点

- 单用户模式：CSRF 验证不引入多用户概念
- 自托管优先：安全加固不增加部署复杂度
- 向后兼容：现有 API 行为不变
- 后端依赖使用 `workspace = true`

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
