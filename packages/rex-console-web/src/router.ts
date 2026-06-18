import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/login',
      name: 'login',
      component: () => import('./pages/Login.vue'),
      meta: { layout: 'fullscreen', public: true },
    },
    {
      path: '/',
      component: () => import('./layouts/AppLayout.vue'),
      children: [
        {
          path: '',
          name: 'dashboard',
          component: () => import('./pages/Dashboard.vue'),
        },
        {
          path: 'environments',
          name: 'environments',
          component: () => import('./pages/Environments.vue'),
        },
        {
          path: 'environments/new',
          name: 'environment-new',
          component: () => import('./pages/EnvironmentNew.vue'),
        },
        {
          path: 'environments/:id',
          name: 'environment-detail',
          component: () => import('./pages/EnvironmentDetail.vue'),
        },
        {
          path: 'environments/:envId/resources/new',
          name: 'resource-new',
          component: () => import('./pages/ResourceNew.vue'),
        },
        {
          path: 'agents',
          name: 'agents',
          component: () => import('./pages/Agents.vue'),
        },
      ],
    },
    {
      path: '/terminal/:resourceId',
      name: 'terminal',
      component: () => import('./pages/Terminal.vue'),
    },
    {
      path: '/files/:resourceId',
      name: 'files',
      component: () => import('./pages/Files.vue'),
    },
  ],
})

// 路由守卫：未登录跳转 /login
router.beforeEach((to) => {
  const token = localStorage.getItem('rex-token')
  if (!to.meta.public && !token) {
    return { name: 'login' }
  }
  if (to.name === 'login' && token) {
    return { name: 'dashboard' }
  }
})

export default router
