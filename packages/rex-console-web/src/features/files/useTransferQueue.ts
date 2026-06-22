import { ref, onMounted, onUnmounted } from 'vue'
import { listTransfers, cancelTransfer, removeTransfer } from '@/api/transfer'
import type { TransferTask } from '@/api/transfer'
import { getErrorMessage } from '@/utils/error'

const POLL_INTERVAL = 3000

export function useTransferQueue() {
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
      error.value = getErrorMessage(e, '加载失败')
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
