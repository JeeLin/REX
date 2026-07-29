# M54: Bug fixes

## Context
M53（Bug fix + UX polish）完成后，用户测试中发现多个 bug：首次登录密码设置跳转失效、新建资源向导验证错位、Agent Token 显示异常。本里程碑集中修复这些问题。

版本类型：patch（bug 修复）

## 产品边界
本阶段修复已知 bug，不涉及新功能。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 修复首次登录密码设置跳转 | ✅ |
| 2 | 修复新建资源向导验证错位 | ✅ |
| 3 | 修复 Agent Token 显示与复制/重置 | ⬜ |

## 子任务详细设计

### 1 修复首次登录密码设置跳转

- **功能目标**：首次运行时无论访问哪个页面，都能正确跳转到密码设置页
- **文件结构**（修改）：`packages/rex-console-web/src/router/index.ts`
- **根因**：路由守卫中 `checkAuth()` 和 `requiresSetup` 重定向都排除了 `to.name === 'login'`，导致直接访问 `/login` 时不检查是否需要设置密码
- **修复方案**：
  1. 移除 `checkAuth()` 的 `/login` 排除
  2. 移除 `requiresSetup` 重定向的 `/login` 排除
  3. 新增反向守卫：密码已设但访问 `/setup` 时跳转 `/login`
- **提交信息**：`fix(auth): redirect to setup page on first login`

### 2 修复新建资源向导验证错位

- **功能目标**：向导各步骤只验证当前步骤的字段，不提前验证后续步骤
- **文件结构**（修改）：`packages/rex-console-web/src/features/resource/WizardModal.vue`
- **根因**：`validateStep()` 中主机地址验证条件为 `step.value === 2`，但主机字段在模板中是 `v-if="step === 3"` 才渲染。验证条件与渲染条件不匹配，导致步骤 2（基本信息）就提示"主机地址为必填项"
- **修复方案**：将主机验证条件改为 `step.value === 3`，名称验证条件改为 `step.value === 2`
- **提交信息**：`fix(wizard): validate host field on correct step`

### 3 修复 Agent Token 显示与复制/重置

- **功能目标**：环境详情页正确显示 Agent Token，复制和重置功能正常工作
- **文件结构**（修改）：`packages/rex-console-web/src/pages/EnvironmentDetailPage.vue`、`crates/rex-hub/src/db.rs`
- **根因**：
  1. 环境详情 API 查询 `(SELECT a.token_hash FROM agents ...) AS agent_token` 将 `token_hash` 别名为 `agent_token`，但 `token_hash` 列存储的是原始 token（命名误导），需确认实际数据流
  2. 重置 token 后前端丢弃了 API 返回的新 token，重新从环境 API 获取，但环境 API 可能未正确返回 token
  3. 无 Agent 注册时 token 显示为 `—`，复制按钮点击无效果（`env.agent_token` 为空）
- **修复方案**：
  1. 确认后端 `GET /api/environments/{id}` 返回的 `agent_token` 字段正确
  2. 重置 token 后使用 API 返回的新 token 直接显示，而非重新 fetch 环境
  3. token 为空时禁用复制按钮，显示提示文字
- **提交信息**：`fix(agent): correct agent token display and copy/reset`

## 设计核对点

- 修复符合产品文档登录流程（3.1 节）
- 修复符合产品文档资源创建流程（3.4 节）
- 修复符合产品文档 Agent 管理流程（3.10 节）
- 不引入多用户/RBAC 概念
- 前端命令使用 `bun`

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
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
