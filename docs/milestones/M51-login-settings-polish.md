# M51: 登录安全增强 + 设置页完善

## Context

M50 完成 UX Bug 修复（v0.43.0）后，登录流程和设置页仍有功能缺口：登录页「记住我」未实际持久化 token、设置页缺少密码修改入口、语言切换和终端配置保存后未即时生效。本里程碑补齐这些功能，提升产品完整性。

版本类型：minor（新功能）
版本号：0.44.0

## 产品边界

**做什么**：
- 登录页「记住我」功能（token 持久化到 localStorage）
- 设置页密码修改功能
- 设置页语言切换联动（切换后立即生效）
- 设置页终端配置联动（字体/光标/保活保存后即时生效）

**不做什么**：
- 不引入 OAuth/SSO 等第三方登录
- 不改变现有 API 契约
- 不做大规模重构

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | 前端：登录页「记住我」功能（token 持久化） | ✅ |
| 2 | 前端：设置页密码修改功能 | ✅ |
| 3 | 前端：设置页语言切换联动 | ✅ |
| 4 | 前端：设置页终端配置联动 | ✅ |
| 5 | 质量验证 | ✅ |

## 子任务详细设计

### 1 前端：登录页「记住我」功能

- **功能目标**：登录时勾选「记住我」后 token 持久化到 localStorage，未勾选则仅存 sessionStorage
- **文件结构**：
  - 修改：`packages/rex-console-web/src/stores/auth.ts`（login 方法支持 remember 参数）
  - 修改：`packages/rex-console-web/src/pages/LoginPage.vue`（添加记住我 checkbox）
- **接口设计**：
  ```typescript
  // auth.ts - login 方法
  async function login(username: string, password: string, remember: boolean): Promise<void>
  // remember=true → localStorage, remember=false → sessionStorage
  ```
- **交互设计**：登录表单底部添加「记住我」checkbox，默认勾选
- **测试标准**：勾选记住我后关闭浏览器重新打开仍保持登录；未勾选则需重新登录
- **提交信息**：`feat(auth): implement remember me with token persistence`

### 2 前端：设置页密码修改功能

- **功能目标**：在设置页添加密码修改区块（当前密码 + 新密码 + 确认密码）
- **文件结构**：
  - 修改：`packages/rex-console-web/src/pages/SettingsPage.vue`
- **接口设计**：
  ```typescript
  // 调用现有 /api/auth/change-password
  POST /api/auth/change-password
  Body: { current_password: string, new_password: string }
  ```
- **交互设计**：设置页「安全」区块添加密码修改表单，保存后 toast 提示成功
- **测试标准**：修改密码后用新密码可登录，旧密码失效
- **提交信息**：`feat(settings): add password change form`

### 3 前端：设置页语言切换联动

- **功能目标**：设置页语言切换后立即生效（无需刷新）
- **文件结构**：
  - 修改：`packages/rex-console-web/src/pages/SettingsPage.vue`
  - 修改：`packages/rex-console-web/src/stores/settings.ts`（如有语言状态管理）
- **修复方案**：语言切换时调用 `locale.value = lang` 并持久化到 localStorage，确保 i18n 实时切换
- **测试标准**：切换语言后页面文本立即变化，刷新后保持
- **提交信息**：`fix(settings): apply language change immediately`

### 4 前端：设置页终端配置联动

- **功能目标**：设置页终端配置（字体/字号/光标闪烁/保活间隔）保存后即时生效到已打开的终端
- **文件结构**：
  - 修改：`packages/rex-console-web/src/pages/SettingsPage.vue`
  - 修改：`packages/rex-console-web/src/features/terminal/useTerminal.ts`（如有配置监听）
- **修复方案**：保存终端配置后通过 event bus 或 store 通知已打开的终端组件更新配置
- **测试标准**：修改字体大小后已打开的终端立即更新
- **提交信息**：`fix(settings): propagate terminal config changes to open terminals`

### 5 质量验证

- **功能目标**：确保所有改动通过质量门禁
- **后端流程**：`cargo fmt --check` + `cargo clippy` + `cargo test`
- **前端流程**：`bun run type-check` + `bun run lint` + `bun run build`
- **测试标准**：所有检查通过
- **提交信息**：`chore: quality gate verification for M51`

## 设计核对点

1. **登录体验**：「记住我」正确持久化 token，未勾选时安全清除
2. **密码安全**：密码修改需验证当前密码，新密码需满足强度要求
3. **设置联动**：语言/终端配置修改后即时生效，无需刷新
4. **一致性**：所有新增 UI 遵循 REX 设计系统

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |

## Bugs

| 状态 | 优先级 | 标题 | 来源 | 描述 |
|------|--------|------|------|------|
