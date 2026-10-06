//! Transfer store wrapper (v0.91.0 T6).
//! Re-creates the composable that FilesPage consumes, backed by the shared
//! Pinia transfer store. Keeps the unified queue (server + browser-blob tasks)
//! and its derived helpers in one place.

import { ref, computed, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import * as filesApi from '@/api/files'
import { useTransferStore } from '@/stores/transfer'
import type { TransferItem } from '@/stores/transfer'

export function useTransfer() {
  const store = useTransferStore()
  const { t } = useI18n()
  const showTransferQueue = ref(false)

  /** Unified queue items read directly from the shared transfer store. */
  const queueItems = computed(() => Array.from(store.tasks.values()))

  const completedCount = computed(() =>
    queueItems.value.filter((i) => i.status === 'done').length,
  )
  const activeCount = computed(() =>
    queueItems.value.filter(
      (i) => i.status === 'running' || i.status === 'pending',
    ).length,
  )

  function taskPath(item: TransferItem): string {
    if (item.kind === 'server') {
      return `${item.source_path || ''} → ${item.target_path || ''}`
    }
    return item.direction === 'up'
      ? item.target_path || ''
      : item.source_path || ''
  }

  /** Queue-row glyph: sync gets its own mark, transfers keep the existing ones. */
  function taskIcon(item: TransferItem): string {
    if (item.task_kind === 'sync') return '🔁'
    if (item.kind === 'browser') return item.direction === 'up' ? '⬆' : '⬇'
    return item.op === 'move' ? '🔄' : '📄'
  }

  /**
   * Server phase word for a queue row. A sync walks
   * pending → scanning → planning → running → verifying → completed/failed,
   * but the store collapses every in-flight phase to `running` so the progress
   * bar shows; the raw server status is kept on `item.phase` and rendered here.
   */
  function phaseLabel(item: TransferItem): string {
    switch (item.phase ?? 'running') {
      case 'scanning':
        return t('files.syncPhaseScanning')
      case 'planning':
        return t('files.syncPhasePlanning')
      case 'verifying':
        return t('files.syncPhaseVerifying')
      case 'running':
        return item.task_kind === 'sync' ? t('files.syncPhaseSyncing') : t('files.transferring')
      case 'pending':
        return t('files.waiting')
      case 'completed':
        return t('files.completed')
      case 'failed':
        return t('files.failed')
      case 'canceled':
        return t('files.canceled')
      default:
        return t('files.transferring')
    }
  }

  /** Retry a failed browser-blob transfer. Server tasks are created server-side
   * and are not retried from the browser. */
  async function retryTransfer(id: string): Promise<void> {
    const item = store.tasks.get(id)
    if (!item || item.kind !== 'browser') return
    store.updateBrowserTask(id, { status: 'running', error: null, progress: 0 })
    try {
      if (item.direction === 'up' && item.file) {
        const remotePath = item.target_path || ''
        const result = await filesApi.uploadFileWithProgress(
          item.sessionId!,
          remotePath,
          item.file,
          (_pct, loaded) => {
            store.updateBrowserTask(id, {
              transferred: loaded,
              progress:
                item.size > 0 ? Math.round((loaded / item.size) * 100) : 0,
            })
          },
        )
        if (result?.upload_id) {
          store.updateBrowserTask(id, { uploadId: result.upload_id })
        }
        store.updateBrowserTask(id, {
          status: 'done',
          progress: 100,
          transferred: item.size,
        })
      } else {
        const remotePath = item.source_path || ''
        const blob = await filesApi.downloadFile(
          item.sessionId!,
          remotePath,
          item.transferred > 0 ? item.transferred : undefined,
        )
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = item.name
        a.click()
        URL.revokeObjectURL(url)
        store.updateBrowserTask(id, {
          status: 'done',
          progress: 100,
          transferred: item.size || blob.size,
        })
      }
    } catch (e) {
      store.updateBrowserTask(id, {
        status: 'error',
        error: e instanceof Error ? e.message : String(e),
      })
    }
  }

  /** Drop completed/failed/canceled tasks from the shared store. */
  function dismissCompleted(): void {
    store.dismissCompleted()
  }

  return reactive({
    store,
    showTransferQueue,
    queueItems,
    completedCount,
    activeCount,
    taskPath,
    taskIcon,
    phaseLabel,
    retryTransfer,
    dismissCompleted,
    pushBrowserTask: store.pushBrowserTask,
    updateBrowserTask: store.updateBrowserTask,
    move: store.move,
    copy: store.copy,
    monitor: store.monitor,
    cancel: store.cancel,
    connectWs: store.connectWs,
    disconnectWs: store.disconnectWs,
  })
}
