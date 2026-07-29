import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { startSession, stopSession } from '@/composables/useSessionTimeout'

const DEFAULT_SESSION_TIMEOUT = 30 // minutes

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'login',
    component: () => import('../pages/LoginPage.vue'),
    meta: { fullscreen: true },
  },
  {
    path: '/setup',
    name: 'setup',
    component: () => import('../pages/SetupPage.vue'),
    meta: { fullscreen: true },
  },
  {
    path: '/',
    component: () => import('../layouts/AppLayout.vue'),
    children: [
      { path: '', redirect: '/workspace' },
      { path: 'workspace', name: 'workspace', component: () => import('../pages/WorkspacePage.vue') },
      { path: 'dashboard', name: 'dashboard', component: () => import('../pages/DashboardPage.vue') },
      { path: 'environments', name: 'environments', component: () => import('../pages/EnvironmentsPage.vue') },
      { path: 'environments/:id', name: 'environment-detail', component: () => import('../pages/EnvironmentDetailPage.vue') },
      { path: 'agents', name: 'agents', component: () => import('../pages/AgentsPage.vue') },
      { path: 'audit-log', name: 'audit-log', component: () => import('../pages/AuditLogPage.vue') },
      { path: 'settings', name: 'settings', component: () => import('../pages/SettingsPage.vue') },
    ],
  },
  // 设计系统预览（开发期可视化，2.0 打磨阶段保留）
  {
    path: '/design-preview',
    name: 'design-preview',
    component: () => import('../features/design-preview/DesignPreview.vue'),
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

let sessionStarted = false

router.beforeEach(async (to) => {
  const auth = useAuthStore()

  // 未登录时检查认证状态，判断是否需要密码设置
  if (!auth.isAuthenticated) {
    await auth.checkAuth()
  }

  // 需要设置密码 → 强制跳转 setup 页面
  if (auth.requiresSetup && to.name !== 'setup') {
    return { name: 'setup' }
  }

  // 不需要设置密码但访问了 setup → 跳转登录页
  if (!auth.isAuthenticated && !auth.requiresSetup && to.name === 'setup') {
    return { name: 'login' }
  }

  // 未登录 → 登录页，停止 session timeout
  if (!auth.isAuthenticated && to.name !== 'login' && to.name !== 'setup') {
    if (sessionStarted) {
      stopSession()
      sessionStarted = false
    }
    return { name: 'login', query: { redirect: to.fullPath } }
  }

  // 已登录访问登录页或设置页 → 工作区
  if (auth.isAuthenticated && (to.name === 'login' || to.name === 'setup')) {
    return { name: 'workspace' }
  }

  // 已登录 → 启动 session timeout（如果尚未启动）
  if (auth.isAuthenticated && !sessionStarted && to.name !== 'login' && to.name !== 'setup') {
    const timeout = parseInt(localStorage.getItem('rex-session-timeout') || String(DEFAULT_SESSION_TIMEOUT), 10)
    startSession(timeout, () => {
      auth.logout()
      sessionStarted = false
      router.push('/login')
    })
    sessionStarted = true
  }
})

export default router
