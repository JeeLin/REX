//! Transfer store wrapper (v0.91.0 T6).
//! Re-creates the composable that FilesPage consumes, backed by the shared
//! Pinia transfer store. Keeps the unified queue (server + browser-blob tasks)
//! and its derived helpers in one place.

import { ref, computed, reactive } from 'vue'
import * as filesApi from '@/api/files'
import { useTransferStore } from '@/stores/transfer'
import type { TransferItem } from '@/stores/transfer'

export function useTransfer() {
  const store = useTransferStore()
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
