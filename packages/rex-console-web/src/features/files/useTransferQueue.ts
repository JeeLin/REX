import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listTransfers, cancelTransfer, removeTransfer } from '@/api/transfer'
import type { TransferTask } from '@/api/transfer'
import { getErrorMessage } from '@/utils/error'

const POLL_INTERVAL = 3000

export function useTransferQueue() {
  const { t } = useI18n()
  const tasks = ref<TransferTask[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  let timer: ReturnType<typeof setInterval> | null = null

  async function refresh() {
    loading.value = true
    error.value = null
    try {
      tasks.value = await listTransfers()
    } catch (e: unknown) {
      error.value = getErrorMessage(e, t('files.loadError'))
    } finally {
      loading.value = false
    }
  }

  function startPolling() {
    refresh()
    timer = setInterval(refresh, POLL_INTERVAL)
  }

  function stopPolling() {
    if (timer) {
      clearInterval(timer)
      timer = null
    }
  }

  async function cancel(id: string) {
    await cancelTransfer(id)
    await refresh()
  }

  async function remove(id: string) {
    await removeTransfer(id)
    await refresh()
  }

  onMounted(startPolling)
  onUnmounted(stopPolling)

  return {
    tasks,
    loading,
    error,
    refresh,
    cancel,
    remove,
  }
}
