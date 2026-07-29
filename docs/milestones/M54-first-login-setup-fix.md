# M54: 首次登录密码设置修复

## Context
M53（Bug fix + UX polish）完成后，用户发现首次进入系统时无法设置密码——不会跳转到密码设置界面，而是直接显示登录表单。本里程碑修复此问题。

版本类型：patch（bug 修复）

## 产品边界
本阶段修复首次登录密码设置跳转逻辑，不涉及新功能或其他模块变更。

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 修复路由守卫：首次运行时正确跳转密码设置页 | ✅ |
| 2 | 补充路由守卫测试 | ⬜ |

## 子任务详细设计

### 1 修复路由守卫：首次运行时正确跳转密码设置页

- **功能目标**：首次运行时无论用户访问哪个页面，都能正确检测到需要设置密码并跳转到 `/setup` 页面
- **文件结构**（修改）：`packages/rex-console-web/src/router/index.ts`
- **接口设计**：无 API 变更，仅前端路由守卫逻辑修复
- **交互设计**：
  - 首次运行访问 `/` → 自动跳转 `/setup`
  - 首次运行访问 `/login` → 自动跳转 `/setup`（修复点）
  - 首次运行访问 `/setup` → 放行，显示密码设置表单
  - 密码已设置访问 `/login` → 显示登录表单
  - 密码已设置访问 `/setup` → 跳转 `/login`
- **后端流程**：无变更
- **测试标准**：路由守卫覆盖所有首次运行和已设置密码的场景
- **提交信息**：`fix(auth): redirect to setup page on first login`

**根因分析**：

原路由守卫中 `checkAuth()` 调用和 `requiresSetup` 重定向都排除了 `to.name === 'login'`：
```typescript
// 原代码（有 bug）
if (!auth.isAuthenticated && to.name !== 'login' && to.name !== 'setup') {
    await auth.checkAuth()  // 访问 /login 时跳过
}
if (auth.requiresSetup && to.name !== 'setup' && to.name !== 'login') {
    return { name: 'setup' }  // 从 /login 跳转时也跳过
}
```

导致直接访问 `/login` 时 `checkAuth()` 不被调用，`requiresSetup` 保持 `false`。

**修复方案**：
1. 移除 `checkAuth()` 的 `/login` 排除 — 所有未认证访问都检查是否需要密码设置
2. 移除 `requiresSetup` 重定向的 `/login` 排除 — 首次运行时从 login 也能跳转 setup
3. 新增反向守卫：密码已设但误访问 `/setup` 时跳转 `/login`

### 2 补充路由守卫测试

- **功能目标**：为路由守卫的首次登录场景编写测试，防止回归
- **文件结构**（创建）：`packages/rex-console-web/src/router/__tests__/auth-guard.test.ts`
- **接口设计**：无
- **交互设计**：无
- **后端流程**：无
- **测试标准**：覆盖首次运行跳转 setup、密码已设显示登录、已登录跳转 workspace 等场景
- **提交信息**：`test(auth): add router guard tests for first-login setup redirect`

## 设计核对点

- 修复符合产品文档 3.1 节登录流程：首次进入应引导用户设置密码
- 不引入多用户/RBAC 概念
- 前端命令使用 `bun`（不用 `npm`）

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
