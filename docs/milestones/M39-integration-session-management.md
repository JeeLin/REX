# M39: 集成完善 + 会话管理

## Context

M38 完成了测试覆盖和 lint 清理。代码库功能完整但存在三个集成层面的差距：
1. **CommandPalette 使用硬编码 mock 数据**：搜索和命令面板不连接真实资源/环境数据，无法搜索或打开真实资源
2. **无会话超时机制**：auth token 在 localStorage 永不过期，存在安全风险
3. **前端设置不同步后端**：theme/language 仅存 localStorage，不通过 settings API 持久化到数据库

本里程碑填补这三个集成差距，提升产品完整性和安全性。

版本类型：minor（集成改进），版本号 0.35.1 → 0.36.0。

## 产品边界

**本阶段做：**
- CommandPalette 对接真实资源/环境数据 + i18n
- 会话超时（idle 检测 + 自动登出 + 过期警告）
- 前端设置同步后端 settings API

**本阶段不做：**
- Agent 日志查看（需要 Agent 端日志上报协议，复杂度高，留待后续）
- 新 UI 组件或页面
- 性能优化

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | CommandPalette 对接真实数据 + i18n | ⬜ |
| 2 | 会话超时 composable（idle 检测 + 自动登出） | ⬜ |
| 3 | 会话超时集成（设置页配置 + 路由守卫集成） | ⬜ |
| 4 | 前端设置同步后端 settings API | ⬜ |

## 子任务详细设计

### 1 CommandPalette 对接真实数据 + i18n

**功能目标**

将 CommandPalette 从硬编码 mock 数据改为从 Pinia stores 读取真实环境和资源数据，支持实时搜索资源/环境/命令，并使用 i18n 翻译所有标签。

**文件结构**

修改：
- `packages/rex-console-web/src/features/workspace/CommandPalette.vue` — 重写数据源和 i18n
- `packages/rex-console-web/src/i18n/locales/zh.json` — 添加 command palette 翻译 key
- `packages/rex-console-web/src/i18n/locales/en.json` — 添加 command palette 翻译 key

**接口设计**

```typescript
// 从 environments store 读取
const environmentsStore = useEnvironmentsStore()

// 构建 commands 列表（computed）
const commands = computed(() => {
  const cmds: Command[] = []

  // 静态命令
  cmds.push(
    { id: 'new-connection', label: t('commandPalette.newConnection'), icon: '📡', category: 'command', action: ... },
    { id: 'settings', label: t('commandPalette.settings'), icon: '⚙️', category: 'command', action: ... },
    { id: 'dashboard', label: t('commandPalette.dashboard'), icon: '📊', category: 'command', action: ... },
    // ...
  )

  // 环境
  for (const env of environmentsStore.environments) {
    cmds.push({
      id: `env-${env.id}`,
      label: env.name,
      icon: '🌍',
      category: 'environment',
      action: () => { router.push(`/environments/${env.id}`); emit('close') }
    })
  }

  // 资源（从 environments 的 resource_count 间接获取，或直接从 API 获取）
  // 使用 resources API 或 environments store 中的资源数据
  for (const env of environmentsStore.environments) {
    // 如果 store 中有资源数据，遍历添加
  }

  return cmds
})

// 搜索过滤（已有的 filter 逻辑保持，但 category 扩展到 'environment' | 'resource'）
const filtered = computed(() => {
  if (!query.value) return commands.value
  const q = query.value.toLowerCase()
  return commands.value.filter(cmd =>
    cmd.label.toLowerCase().includes(q) ||
    cmd.category.toLowerCase().includes(q)
  )
})
```

**交互设计**

- 打开 CommandPalette 时，从 store 加载环境和资源数据
- 搜索框输入实时过滤：资源名/环境名/命令名
- 类别分组显示：命令 | 环境 | 资源
- 点击资源 → 在工作区打开对应 Tab
- 点击环境 → 导航到环境详情页
- Esc 关闭

**测试标准**

- 打开 CommandPalette 显示真实环境和资源列表
- 搜索框输入可过滤资源/环境/命令
- 点击资源条目打开工作区 Tab
- 所有标签使用 i18n 翻译
- `bun run type-check` + `bun run lint` 通过

**提交信息**: `feat(workspace): integrate CommandPalette with real resource and environment data`

### 2 会话超时 composable（idle 检测 + 自动登出）

**功能目标**

实现 `useSessionTimeout` composable，监听用户活动（鼠标移动、键盘、滚动），在闲置超过配置时间后自动登出。超时前 60 秒弹出警告对话框，用户可选择续期或立即登出。

**文件结构**

创建：
- `packages/rex-console-web/src/composables/useSessionTimeout.ts` — 会话超时 composable

**接口设计**

```typescript
// composables/useSessionTimeout.ts
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

export function useSessionTimeout(timeoutMinutes: number = 30) {
  const router = useRouter()
  const authStore = useAuthStore()

  const showWarning = ref(false)
  const remainingSeconds = ref(0)
  let idleTimer: ReturnType<typeof setInterval> | null = null
  let warningTimer: ReturnType<typeof setInterval> | null = null
  let lastActivity = Date.now()

  // 检测用户活动的事件列表
  const EVENTS = ['mousedown', 'keydown', 'scroll', 'touchstart']

  function resetIdle() {
    lastActivity = Date.now()
    showWarning.value = false
    remainingSeconds.value = 0
    if (warningTimer) { clearInterval(warningTimer); warningTimer = null }
  }

  function startWarningCountdown() {
    remainingSeconds.value = 60
    warningTimer = setInterval(() => {
      remainingSeconds.value--
      if (remainingSeconds.value <= 0) {
        logout()
      }
    }, 1000)
  }

  function logout() {
    stop()
    authStore.logout()
    router.push('/login')
  }

  function extendSession() {
    resetIdle()
    startMonitoring()
  }

  function startMonitoring() {
    stopMonitoring()
    idleTimer = setInterval(() => {
      const idleMs = Date.now() - lastActivity
      const timeoutMs = timeoutMinutes * 60 * 1000
      const warningMs = timeoutMs - 60 * 1000  // 提前 60 秒警告

      if (idleMs >= timeoutMs) {
        logout()
      } else if (idleMs >= warningMs && !showWarning.value) {
        showWarning.value = true
        startWarningCountdown()
      }
    }, 5000)  // 每 5 秒检查一次
  }

  function stopMonitoring() {
    if (idleTimer) { clearInterval(idleTimer); idleTimer = null }
  }

  function stop() {
    stopMonitoring()
    if (warningTimer) { clearInterval(warningTimer); warningTimer = null }
    EVENTS.forEach(e => document.removeEventListener(e, resetIdle))
  }

  onMounted(() => {
    EVENTS.forEach(e => document.addEventListener(e, resetIdle, { passive: true }))
    startMonitoring()
  })

  onBeforeUnmount(stop)

  return { showWarning, remainingSeconds, extendSession, logout }
}
```

**交互设计**

- 用户活跃（鼠标/键盘/滚动）→ 重置计时器
- 闲置接近超时（最后 60 秒）→ 弹出警告对话框「会话即将过期，是否续期？」
- 警告对话框显示倒计时（60 秒），两个按钮：「续期」/「立即登出」
- 点击「续期」→ 重置计时器，继续监控
- 倒计时归零 → 自动登出到登录页
- 超时时间可通过设置页配置（子任务 3）

**测试标准**

- 闲置指定时间后自动登出
- 用户活动重置计时器
- 警告对话框在超时前 60 秒显示
- 续期按钮重置计时器
- `bun run type-check` + `bun run lint` 通过

**提交信息**: `feat(auth): add session timeout with idle detection and warning dialog`

### 3 会话超时集成（设置页配置 + 路由守卫集成）

**功能目标**

将 useSessionTimeout 集成到应用层：
1. 路由守卫中启动会话超时监控（登录后开始，登出后停止）
2. 设置页新增「安全」区块，支持配置会话超时时间（15/30/60/120 分钟）
3. 超时时间持久化到 localStorage

**文件结构**

修改：
- `packages/rex-console-web/src/router/index.ts` — 路由守卫中启动/停止会话超时
- `packages/rex-console-web/src/pages/SettingsPage.vue` — 新增安全区块
- `packages/rex-console-web/src/i18n/locales/zh.json` — 添加 session timeout 翻译 key
- `packages/rex-console-web/src/i18n/locales/en.json` — 添加 session timeout 翻译 key

**路由守卫集成**

```typescript
// router/index.ts
let sessionTimeout: ReturnType<typeof useSessionTimeout> | null = null

router.beforeEach(async (to) => {
  const auth = useAuthStore()

  if (!auth.isAuthenticated && to.name !== 'login') {
    await auth.checkAuth()
  }

  if (auth.requiresSetup && to.name !== 'setup') {
    return { name: 'setup' }
  }

  if (!auth.isAuthenticated && to.name !== 'login') {
    return { name: 'login', query: { redirect: to.fullPath } }
  }

  // 登录后启动会话超时（如果尚未启动）
  if (auth.isAuthenticated && !sessionTimeout) {
    const timeout = parseInt(localStorage.getItem('rex-session-timeout') || '30')
    sessionTimeout = useSessionTimeout(timeout)
  }
})

// 登出时清理
export function cleanupSession() {
  if (sessionTimeout) {
    sessionTimeout.stop()
    sessionTimeout = null
  }
}
```

**设置页安全区块**

```vue
<!-- SettingsPage.vue 新增安全区块 -->
<div class="settings-section">
  <h3>{{ t('settings.security') }}</h3>
  <div class="form-row">
    <label class="form-label">{{ t('settings.sessionTimeout') }}</label>
    <select v-model="settings.session_timeout" class="form-input" @change="onSessionTimeoutChange">
      <option value="15">15 {{ t('settings.minutes') }}</option>
      <option value="30">30 {{ t('settings.minutes') }}</option>
      <option value="60">60 {{ t('settings.minutes') }}</option>
      <option value="120">120 {{ t('settings.minutes') }}</option>
    </select>
  </div>
</div>
```

**测试标准**

- 登录后会话超时自动启动
- 设置页可修改超时时间（15/30/60/120 分钟）
- 超时时间持久化到 localStorage
- 修改超时时间后立即生效
- `bun run type-check` + `bun run lint` 通过

**提交信息**: `feat(web): integrate session timeout with settings page configuration`

### 4 前端设置同步后端 settings API

**功能目标**

将前端设置（theme、language、terminal_font、terminal_font_size）从仅 localStorage 改为双写（localStorage + 后端 settings API），确保设置跨会话持久化。

后端 settings API 已存在（`GET/PUT /api/settings`），支持 theme、language、terminal_font、terminal_font_size。前端需要在启动时从后端加载设置，在保存时同步到后端。

**文件结构**

修改：
- `packages/rex-console-web/src/stores/settings.ts` — 新建或修改 settings store，加载/保存时调用后端 API
- `packages/rex-console-web/src/pages/SettingsPage.vue` — 使用 settings store 而非直接操作 localStorage
- `packages/rex-console-web/src/api/settings.ts` — 新建 settings API 模块

**接口设计**

```typescript
// api/settings.ts
import { api } from './client'

export interface Settings {
  theme: string
  language: string
  terminal_font: string
  terminal_font_size: string
}

export const settingsApi = {
  get: () => api.get<Settings>('/settings'),
  update: (settings: Partial<Settings>) => api.put<{ ok: boolean }>('/settings', settings),
}

// stores/settings.ts
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { settingsApi, type Settings } from '@/api/settings'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<Settings>({
    theme: localStorage.getItem('rex-theme') || 'dark',
    language: localStorage.getItem('rex-lang') || 'zh',
    terminal_font: 'JetBrains Mono',
    terminal_font_size: '14',
  })

  async function loadSettings() {
    try {
      const remote = await settingsApi.get()
      settings.value = remote
      // 同步到 localStorage 作为 fallback
      localStorage.setItem('rex-theme', remote.theme)
      localStorage.setItem('rex-lang', remote.language)
    } catch {
      // 后端不可用时使用 localStorage
    }
  }

  async function saveSettings(partial: Partial<Settings>) {
    Object.assign(settings.value, partial)
    // 双写：localStorage + 后端
    if (partial.theme) localStorage.setItem('rex-theme', partial.theme)
    if (partial.language) localStorage.setItem('rex-lang', partial.language)
    try {
      await settingsApi.update(partial)
    } catch {
      // 后端保存失败，localStorage 已更新
    }
  }

  return { settings, loadSettings, saveSettings }
})
```

**交互设计**

- 应用启动时：从后端加载设置 → 覆盖 localStorage → 应用到 UI
- 修改设置时：立即更新 localStorage（即时生效）+ 异步同步到后端
- 后端不可用时：降级到 localStorage（离线兼容）

**测试标准**

- 启动时从后端加载设置并应用
- 修改 theme/language 后双写到 localStorage + 后端
- 刷新页面后设置保留
- 后端不可用时降级到 localStorage
- `bun run type-check` + `bun run lint` 通过

**提交信息**: `feat(settings): sync frontend settings with backend API for persistence`

## 设计核对点

- ✅ 不引入多用户、RBAC、企业协作概念
- ✅ 单用户模型，session timeout 为单用户闲置检测
- ✅ 文件传输不经过浏览器（本里程碑不涉及文件传输）
- ✅ 前端命令使用 bun
- ✅ 依赖声明符合 workspace 规则
- ✅ 不新增后端 API（settings API 已存在）
- ✅ CommandPalette 数据来自真实 store，不引入新数据源

## Flow Status

- [ ] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

（打回时追加一条，创建里程碑文档时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
