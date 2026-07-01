<template>
  <router-view />
  <ContextMenu />
  <ToastProvider />
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import ContextMenu from '@/components/ContextMenu.vue'
import ToastProvider from '@/components/ToastProvider.vue'
import { useSessionTimeout } from '@/composables/useSessionTimeout'
import { loadSettingsFromBackend } from '@/stores/settings'
import { useUserStore } from '@/stores/user'

useSessionTimeout()

// Load user settings from backend on startup
onMounted(async () => {
  await Promise.all([
    loadSettingsFromBackend(),
    useUserStore().loadFromBackend(),
  ])
})
</script>
