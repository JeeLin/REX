import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

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

router.beforeEach(async (to) => {
  const auth = useAuthStore()

  // 首次访问时检查认证状态
  if (!auth.isAuthenticated && to.name !== 'login' && to.name !== 'setup') {
    await auth.checkAuth()
  }

  // 需要设置密码 → setup 页面
  if (auth.requiresSetup && to.name !== 'setup' && to.name !== 'login') {
    return { name: 'setup' }
  }

  // 未登录 → 登录页
  if (!auth.isAuthenticated && to.name !== 'login' && to.name !== 'setup') {
    return { name: 'login', query: { redirect: to.fullPath } }
  }

  // 已登录访问登录页或设置页 → 工作区
  if (auth.isAuthenticated && (to.name === 'login' || to.name === 'setup')) {
    return { name: 'workspace' }
  }
})

export default router
