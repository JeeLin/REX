<template>
  <ErrorBoundary>
    <router-view />
    <ContextMenu />
    <ToastProvider />
  </ErrorBoundary>
</template>

<script setup lang="ts">
import { onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import ErrorBoundary from '@/components/ErrorBoundary.vue'
import ContextMenu from '@/components/ContextMenu.vue'
import ToastProvider from '@/components/ToastProvider.vue'
import { useSessionTimeout } from '@/composables/useSessionTimeout'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { useToast } from '@/composables/useToast'
import { useSettingsStore } from '@/stores/settings'
import { useUserStore } from '@/stores/user'

useSessionTimeout()

const { t } = useI18n()
const { isOnline } = useNetworkStatus()
const toast = useToast()
const settingsStore = useSettingsStore()

// Watch network status changes
watch(isOnline, (online) => {
  if (online) {
    toast.success(t('network.restored'))
  } else {
    toast.error(t('network.offline'))
  }
})

// Load user settings from backend on startup
onMounted(async () => {
  await Promise.all([
    settingsStore.loadSettingsFromBackend(),
    useUserStore().loadFromBackend(),
  ])
})
</script>
