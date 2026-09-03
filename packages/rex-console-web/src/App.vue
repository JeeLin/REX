<script setup lang="ts">
import { RouterView } from 'vue-router'
import { onMounted, onUnmounted, ref } from 'vue'
import { KeepAlive } from 'vue'
import ErrorBoundary from './components/ErrorBoundary.vue'
import NotificationToast from './components/NotificationToast.vue'
import TokenRefreshModal from './components/TokenRefreshModal.vue'
import { tokenRefreshEvent } from '@/api/client'

// 早期应用主题，避免闪烁
const stored = localStorage.getItem('rex-theme') || 'dark'
document.documentElement.dataset.theme = stored === 'dark' ? undefined : stored

onMounted(async () => {
  // 从后端同步最新设置（仅作为 localStorage 缺失时的回退，不覆盖用户前端偏好）
  // 仅在已登录时同步，避免 401 触发弹窗
  const hasToken = localStorage.getItem('rex-token') || sessionStorage.getItem('rex-token')
  if (!hasToken) return
  // 如果 localStorage 已有主题设置，信任前端偏好，不再覆盖
  if (localStorage.getItem('rex-theme')) return
  try {
    const { settingsApi } = await import('@/api/settings')
    const settings = await settingsApi.get()
    document.documentElement.dataset.theme = settings.theme === 'dark' ? undefined : settings.theme
    localStorage.setItem('rex-theme', settings.theme)
  } catch {
    // ignore，使用 localStorage 缓存
  }
})

// Token 过期弹窗状态
const showTokenRefresh = ref(false)

function onUnauthorized() {
  showTokenRefresh.value = true
}

function onRefreshCancel() {
  showTokenRefresh.value = false
}

onMounted(() => {
  tokenRefreshEvent.addEventListener('unauthorized', onUnauthorized)
})

onUnmounted(() => {
  tokenRefreshEvent.removeEventListener('unauthorized', onUnauthorized)
})
</script>

<template>
  <ErrorBoundary>
    <KeepAlive :include="['WorkspacePage']">
      <RouterView />
    </KeepAlive>
    <NotificationToast />
    <TokenRefreshModal v-if="showTokenRefresh" @cancel="onRefreshCancel" />
  </ErrorBoundary>
</template>
