import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'login',
    component: () => import('../pages/LoginPage.vue'),
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

export default router
