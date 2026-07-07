<template>
  <ErrorBoundary>
    <router-view />
    <ContextMenu />
    <ToastProvider />
  </ErrorBoundary>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import ErrorBoundary from '@/components/ErrorBoundary.vue'
import ContextMenu from '@/components/ContextMenu.vue'
import ToastProvider from '@/components/ToastProvider.vue'
import { useSessionTimeout } from '@/composables/useSessionTimeout'
import { useSettingsStore } from '@/stores/settings'
import { useUserStore } from '@/stores/user'

useSessionTimeout()

const settingsStore = useSettingsStore()

// Load user settings from backend on startup
onMounted(async () => {
  await Promise.all([
    settingsStore.loadSettingsFromBackend(),
    useUserStore().loadFromBackend(),
  ])
})
</script>
