import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/login',
      name: 'login',
      component: () => import('./pages/Login.vue'),
    },
    {
      path: '/',
      name: 'dashboard',
      component: () => import('./pages/Dashboard.vue'),
    },
  ],
})

export default router
