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

  // Speed/ETA state
  const speeds = ref<Map<string, number>>(new Map())
  const etas = ref<Map<string, number>>(new Map())
  const prevBytes = new Map<string, number>()

  // Previous tasks snapshot (for toast notifications)
  const prevTasks = ref<TransferTask[]>([])

  function computeSpeedAndEta(currentTasks: TransferTask[]) {
    const newSpeeds = new Map<string, number>()
    const newEtas = new Map<string, number>()

    for (const task of currentTasks) {
      if (task.status === 'running' || task.status === 'pending') {
        const prev = prevBytes.get(task.id)
        if (prev !== undefined) {
          const delta = task.progress.transferred_bytes - prev
          const speed = Math.max(0, delta / (POLL_INTERVAL / 1000)) // bytes/s
          newSpeeds.set(task.id, speed)

          if (speed > 0) {
            const remaining = task.progress.total_bytes - task.progress.transferred_bytes
            newEtas.set(task.id, Math.ceil(remaining / speed))
          }
        }
        prevBytes.set(task.id, task.progress.transferred_bytes)
      } else {
        prevBytes.delete(task.id)
      }
    }

    speeds.value = newSpeeds
    etas.value = newEtas
  }

  async function refresh() {
    loading.value = true
    error.value = null
    try {
      prevTasks.value = tasks.value
      tasks.value = await listTransfers()
      computeSpeedAndEta(tasks.value)
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
    speeds,
    etas,
    prevTasks,
  }
}
