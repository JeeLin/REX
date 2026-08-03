import { defineStore } from 'pinia'
import { ref } from 'vue'
import { updateApi, type UpdateInfo, type UpdateStatus } from '@/api/settings'

export const useUpdateStore = defineStore('update', () => {
  const currentVersion = ref(__APP_VERSION__)
  const latestVersion = ref('')
  const hasUpdate = ref(false)
  const updateLoading = ref(false)
  const updating = ref(false)
  const updateError = ref('')
  const updateStatusText = ref('')
  const updateProgress = ref(0)

  let pollInterval: ReturnType<typeof setInterval> | null = null

  function stopPolling() {
    if (pollInterval) {
      clearInterval(pollInterval)
      pollInterval = null
    }
  }

  async function checkForUpdate() {
    updateLoading.value = true
    updateError.value = ''
    try {
      const info: UpdateInfo = await updateApi.check()
      latestVersion.value = info.latest_version
      hasUpdate.value = info.has_update
    } catch (e: unknown) {
      updateError.value = e instanceof Error ? e.message : 'Update check failed'
    } finally {
      updateLoading.value = false
    }
  }

  function startPolling(onComplete?: () => void) {
    stopPolling()
    pollInterval = setInterval(async () => {
      try {
        const status: UpdateStatus = await updateApi.status()
        updateStatusText.value = status.phase
        updateProgress.value = status.attempt * 25
        if (status.phase === 'committed' || status.phase === 'rolled_back' || status.phase === 'failed') {
          stopPolling()
          updating.value = false
          await checkForUpdate()
          onComplete?.()
        }
      } catch (e: unknown) {
        stopPolling()
        updating.value = false
        updateError.value = e instanceof Error ? e.message : 'Failed to get update status'
      }
    }, 2000)
  }

  async function triggerUpdate() {
    updating.value = true
    updateError.value = ''
    try {
      await updateApi.trigger()
      startPolling()
    } catch (e: unknown) {
      updating.value = false
      updateError.value = e instanceof Error ? e.message : 'Failed to trigger update'
    }
  }

  async function rollbackUpdate() {
    updating.value = true
    updateError.value = ''
    try {
      await updateApi.rollback()
      startPolling()
    } catch (e: unknown) {
      updating.value = false
      updateError.value = e instanceof Error ? e.message : 'Failed to rollback update'
    }
  }

  return {
    currentVersion,
    latestVersion,
    hasUpdate,
    updateLoading,
    updating,
    updateError,
    updateStatusText,
    updateProgress,
    checkForUpdate,
    triggerUpdate,
    rollbackUpdate,
    stopPolling,
  }
})
