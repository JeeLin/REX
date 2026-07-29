# M54: Bug fixes

## Context
M53（Bug fix + UX polish）完成后，用户测试中发现多个 bug。本里程碑集中修复这些问题。

版本类型：patch（bug 修复）

## 产品边界
本阶段修复已知 bug，不涉及新功能。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 修复首次登录密码设置跳转 | ✅ |
| 2 | 修复新建资源向导验证错位 | ✅ |
| 3 | 修复 Agent Token 显示与复制/重置 | ✅ |

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

- **功能目标**：环境详情页正确显示 Agent Token，无 Agent 时给出提示而非显示无效的复制/重置按钮
- **文件结构**（修改）：
  - `packages/rex-console-web/src/pages/EnvironmentDetailPage.vue`
  - `packages/rex-console-web/src/i18n/locales/en.json`
  - `packages/rex-console-web/src/i18n/locales/zh.json`
- **根因**：环境创建后无 Agent 注册时 `agent_token` 为空，模板显示 `—`，但复制/重置按钮仍然可用且无效果
- **修复方案**：
  1. 当 `agent_token` 为空时隐藏复制/重置按钮，显示提示文字"暂无注册的 Agent。部署 Agent 后可获取注册令牌。"
  2. 有 token 时正常显示 token 值和操作按钮
  3. 添加 i18n 键 `environmentDetail.noAgentToken`（中/英）
- **提交信息**：`fix(agent): improve agent token display when no agent registered`

## 设计核对点

- 修复符合产品文档登录流程（3.1 节）
- 修复符合产品文档资源创建流程（3.4 节）
- 修复符合产品文档 Agent 管理流程（3.10 节）
- 不引入多用户/RBAC 概念
- 前端命令使用 `bun`

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
