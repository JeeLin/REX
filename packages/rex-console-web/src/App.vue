<script setup lang="ts">
import { RouterView } from 'vue-router'
import { onMounted } from 'vue'
import ErrorBoundary from './components/ErrorBoundary.vue'

// 早期应用主题，避免闪烁
const stored = localStorage.getItem('rex-theme') || 'dark'
document.documentElement.dataset.theme = stored === 'dark' ? undefined : stored

onMounted(async () => {
  // 从后端同步最新设置（覆盖 localStorage）
  try {
    const { settingsApi } = await import('@/api/settings')
    const settings = await settingsApi.get()
    document.documentElement.dataset.theme = settings.theme === 'dark' ? undefined : settings.theme
    localStorage.setItem('rex-theme', settings.theme)
  } catch {
    // ignore，使用 localStorage 缓存
  }
})
</script>

<template>
  <ErrorBoundary>
    <RouterView />
  </ErrorBoundary>
</template>
