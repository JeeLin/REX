# M37: i18n 完整翻译 + 性能优化 + 生产加固

## Context

M36 完成了后端日志增强（请求日志中间件、审计日志增强）。当前前端 i18n 覆盖不完整，LoginPage、EnvironmentsPage、AgentsPage、SettingsPage 及 ShortcutPanel、WizardModal 等功能组件仍有硬编码中文/英文。此外，部分路由（/login、/setup）在生产环境也可访问，WebSocket 缺少心跳机制，路由无懒加载。

本里程碑补全 i18n 翻译、优化性能、加固生产环境。

版本类型：minor（新增 i18n 翻译、路由懒加载、生产加固），版本号 0.34.1 → 0.35.0。

## 产品边界

**本阶段做：**
- 所有页面文本使用 `$t()` 调用，中英文完整覆盖
- 功能组件（ShortcutPanel、WizardModal、ResourceProperties 等）i18n 翻译
- 路由级懒加载（所有页面组件使用 `defineAsyncComponent`）
- WebSocket 心跳（客户端定时 ping，断开后显示重连）
- 路由守卫（/login、/setup 页面在生产环境不可访问，认证后重定向到 /workspace）
- 安全 HTTP 响应头（CSP、X-Frame-Options、X-Content-Type-Options）

**本阶段不做：**
- 响应式移动端适配（M14 范围）
- 凭据加密存储（M14 范围）
- 全面单元测试覆盖（M14 范围）
- 新功能开发

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | i18n：页面组件翻译（LoginPage、SetupPage、EnvironmentsPage、EnvironmentDetailPage） | ✅ |
| 2 | i18n：页面组件翻译（AgentsPage、AuditLogPage、SettingsPage） | ✅ |
| 3 | i18n：功能组件翻译（ShortcutPanel、ConnectionTree、ResourcePanel、WorkspacePage） | ✅ |
| 4 | i18n：补全 zh.json 和 en.json 语言包 | ✅ |
| 5 | 路由级懒加载 | ✅ |
| 6 | WebSocket 心跳 + 断线重连 UI | ✅ |
| 7 | 生产加固（安全头 + 路由守卫） | ✅ |

## 子任务详细设计

### 1 i18n：页面组件翻译（LoginPage、SetupPage、EnvironmentsPage、EnvironmentDetailPage）

**功能目标**

将以下页面中的硬编码中文/英文字符串替换为 `$t()` 调用：
- `LoginPage.vue` — 登录表单、按钮、提示信息
- `SetupPage.vue` — 设置密码表单、按钮、提示信息
- `EnvironmentsPage.vue` — 页面标题、按钮、筛选、表格列标题、空状态
- `EnvironmentDetailPage.vue` — 页面标题、按钮、Agent 面板、资源表格

**文件结构**

修改：
- `packages/rex-console-web/src/pages/LoginPage.vue`
- `packages/rex-console-web/src/pages/SetupPage.vue`
- `packages/rex-console-web/src/pages/EnvironmentsPage.vue`
- `packages/rex-console-web/src/pages/EnvironmentDetailPage.vue`

**实现方案**

在每个 Vue 文件中找到所有硬编码的中文/英文字符串，替换为 `$t('key')` 调用。键命名遵循现有约定（如 `login.title`、`env.create`、`env.description`）。

示例：
```vue
<!-- Before -->
<h1>登录</h1>
<button>登录</button>

<!-- After -->
<h1>{{ $t('login.title') }}</h1>
<button>{{ $t('login.submit') }}</button>
```

**测试标准**

- 所有硬编码字符串已替换为 `$t()`
- 切换语言后页面文本正确变化
- 无遗漏的硬编码中文/英文

**提交信息**: `feat(i18n): translate LoginPage, SetupPage, EnvironmentsPage, EnvironmentDetailPage`

### 2 i18n：页面组件翻译（AgentsPage、AuditLogPage、SettingsPage）

**功能目标**

将以下页面中的硬编码字符串替换为 `$t()` 调用：
- `AgentsPage.vue` — Agent 列表、状态、部署指南、操作按钮
- `AuditLogPage.vue` — 日志筛选、表格、统计卡片、空状态
- `SettingsPage.vue` — 分组标题、表单标签、按钮、提示信息

**文件结构**

修改：
- `packages/rex-console-web/src/pages/AgentsPage.vue`
- `packages/rex-console-web/src/pages/AuditLogPage.vue`
- `packages/rex-console-web/src/pages/SettingsPage.vue`

**实现方案**

同子任务1，找到硬编码字符串并替换为 `$t()`。

键命名约定：
- `agents.xxx` — AgentsPage
- `audit.xxx` — AuditLogPage
- `settings.xxx` — SettingsPage

**测试标准**

- 同子任务1

**提交信息**: `feat(i18n): translate AgentsPage, AuditLogPage, SettingsPage`

### 3 i18n：功能组件翻译

**功能目标**

将功能组件中的硬编码字符串替换为 `$t()` 调用：
- `ShortcutPanel.vue` — 快捷键分组标题和按键描述
- `ShortcutSettings.vue` — 快捷键设置分组和按键名称
- `WizardModal.vue` — 向导标题、步骤、按钮、表单标签
- `ResourceProperties.vue` — 属性标签、按钮、占位符
- `ResourcePanel.vue` — 侧栏标题、搜索、空状态
- `WorkspacePage.vue` — 工作区 Tab 标签、右键菜单项、快捷键提示

**文件结构**

修改：
- `packages/rex-console-web/src/features/workspace/ShortcutPanel.vue`
- `packages/rex-console-web/src/features/workspace/ShortcutSettings.vue`
- `packages/rex-console-web/src/features/resource/WizardModal.vue`
- `packages/rex-console-web/src/features/resource/ResourceProperties.vue`
- `packages/rex-console-web/src/features/resource-panel/ResourcePanel.vue`
- `packages/rex-console-web/src/pages/WorkspacePage.vue`

**实现方案**

同子任务1，找到硬编码字符串并替换为 `$t()`。

键命名约定：
- `shortcuts.xxx` — 快捷键相关
- `wizard.xxx` — 向导相关
- `resource.xxx` — 资源属性相关
- `resourcePanel.xxx` — 侧栏相关
- `workspace.xxx` — 工作区相关

**测试标准**

- 同子任务1

**提交信息**: `feat(i18n): translate ShortcutPanel, WizardModal, ResourceProperties, ResourcePanel, WorkspacePage`

### 4 i18n：补全语言包

**功能目标**

在 zh.json 和 en.json 中添加所有新增的翻译 key，确保两个语言包完全对齐。

**文件结构**

修改：
- `packages/rex-console-web/src/i18n/locales/zh.json`
- `packages/rex-console-web/src/i18n/locales/en.json`

**实现方案**

1. 在子任务1-3完成后，运行 `vue-tsc --noEmit` 检查是否有未定义的 i18n key
2. 在 zh.json 中添加所有新增 key 的中文翻译
3. 在 en.json 中添加所有新增 key 的英文翻译
4. 确保两个文件的 key 集合完全一致

**测试标准**

- zh.json 和 en.json 的 key 数量一致
- 所有 `$t()` 调用在两个语言包中都有对应翻译
- `vue-tsc --noEmit` 无 i18n 相关错误

**提交信息**: `feat(i18n): complete zh.json and en.json locale files`

### 5 路由级懒加载

**功能目标**

所有页面组件（LoginPage、WorkspacePage、EnvironmentsPage、AgentsPage 等）使用 `defineAsyncComponent` 实现路由级懒加载，减少首屏包体积。

**文件结构**

修改：
- `packages/rex-console-web/src/router/index.ts`

**实现方案**

```typescript
import { defineAsyncComponent } from 'vue'

const LoginPage = defineAsyncComponent(() => import('@/pages/LoginPage.vue'))
const WorkspacePage = defineAsyncComponent(() => import('@/pages/WorkspacePage.vue'))
const EnvironmentsPage = defineAsyncComponent(() => import('@/pages/EnvironmentsPage.vue'))
const AgentsPage = defineAsyncComponent(() => import('@/pages/AgentsPage.vue'))
const AuditLogPage = defineAsyncComponent(() => import('@/pages/AuditLogPage.vue'))
const SettingsPage = defineAsyncComponent(() => import('@/pages/SettingsPage.vue'))
const DashboardPage = defineAsyncComponent(() => import('@/pages/DashboardPage.vue'))
```

**测试标准**

- `bun run build` 输出的 chunk 数量增加（每个页面独立 chunk）
- 页面切换正常，无白屏或闪烁

**提交信息**: `perf(web): add route-level lazy loading for all pages`

### 6 WebSocket 心跳 + 断线重连 UI

**功能目标**

TerminalView 的 WebSocket 连接添加心跳机制（客户端每 30 秒发送 ping），断线后显示重连界面（含重连按钮和倒计时）。

**文件结构**

修改：
- `packages/rex-console-web/src/features/terminal/TerminalView.vue`
- `crates/rex-hub/src/terminal_ws.rs`（服务端忽略 ping 帧）

**实现方案**

客户端（TerminalView.vue）：
```typescript
let heartbeatTimer: ReturnType<typeof setInterval>

function startHeartbeat() {
  heartbeatTimer = setInterval(() => {
    if (ws.value && ws.value.readyState === WebSocket.OPEN) {
      ws.value.send(JSON.stringify({ type: 'ping' }))
    }
  }, 30000)
}

function stopHeartbeat() {
  clearInterval(heartbeatTimer)
}

// 在 onclose 时停止心跳，显示重连界面
// 在 onopen 时启动心跳
```

服务端（terminal_ws.rs）：
```rust
// 在 handle_socket 中忽略 ping 消息
// WebSocket 的 ping/pong 由 tungstenite/tokio-tungstenite 自动处理
// 客户端发送的自定义 ping 消息需要服务端忽略
```

**测试标准**

- WebSocket 连接正常工作
- 30 秒无活动后客户端发送 ping
- 服务端正确处理 ping 消息（不报错）
- 断线后显示重连界面

**提交信息**: `feat(web): add WebSocket heartbeat and reconnect UI for terminal`

### 7 生产加固（安全头 + 路由守卫）

**功能目标**

1. 路由守卫：认证后直接访问 `/login` 或 `/setup` 时重定向到 `/workspace`
2. 安全 HTTP 响应头：CSP、X-Frame-Options、X-Content-Type-Options

**文件结构**

修改：
- `packages/rex-console-web/src/router/index.ts`（路由守卫）
- `crates/rex-hub/src/rex-hub.rs`（安全头 middleware）

**实现方案**

路由守卫：
```typescript
router.beforeEach(async (to) => {
  const auth = useAuthStore()
  if (!auth.isAuthenticated) await auth.checkAuth()

  // 已登录 → 访问 /login 或 /setup → 重定向到 /workspace
  if (auth.isAuthenticated && (to.name === 'login' || to.name === 'setup')) {
    return { name: 'workspace' }
  }

  // 未登录 → 访问受保护页面 → 重定向到 /login
  if (!auth.isAuthenticated && to.name !== 'login' && to.name !== 'setup') {
    return { name: 'login' }
  }
})
```

安全头 middleware（Rust）：
```rust
async fn security_headers(req: Request<Body>, next: Next) -> Response {
  let mut response = next.run(req).await;
  let headers = response.headers_mut();
  headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
  headers.insert("X-Frame-Options", "DENY".parse().unwrap());
  headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
  response
}
```

**测试标准**

- 已登录访问 /login 自动跳转 /workspace
- 未登录访问 /workspace 自动跳转 /login
- HTTP 响应包含安全头

**提交信息**: `feat(web): add security headers and route guard for authenticated redirect`

## 设计核对点

- ✅ 不引入新功能，只补全翻译和加固
- ✅ 使用现有 `$t()` 机制，不引入新的 i18n 库
- ✅ 路由懒加载不影响现有功能
- ✅ WebSocket 心跳不影响 SSH 终端交互
- ✅ 安全头不破坏现有 API 调用
- ✅ 敏感信息（密码、token）不写入日志

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

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
