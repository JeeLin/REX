//! Shared file-transfer task store (v0.91.0 T2).
//! Single source of truth for server-side move/copy tasks. The browser never
//! holds file bytes: a task is created via POST /api/files/transfer/action and
//! progress is polled from GET /api/files/transfer/{id}. The store is a module
//! singleton so every component (FilesPage, FilesDrawer, …) shares one queue.
//!
//! Note on the GET response shape: progress fields are FLAT top-level
//! (`total_bytes`, `transferred_bytes`, `speed_bytes_per_sec`, `eta_seconds`),
//! not nested under a `progress` object (see `api/files.ts` TransferTaskRecord).

import { ref, type Ref } from 'vue'
import * as filesApi from '@/api/files'
import type {
  TransferConflict,
  TransferEndpoint,
  TransferOp,
  TransferTaskStatus,
} from '@/api/files'

export interface TransferTaskState {
  id: string
  op: TransferOp
  source_path: string
  target_path: string
  status: TransferTaskStatus
  total_bytes: number
  transferred_bytes: number
  speed_bytes_per_sec: number
  eta_seconds: number | null
  error: string | null
}

export interface TransferTaskHandle {
  id: string
  status: string
}

// --- shared store (module singleton) ---
const tasks: Ref<Map<string, TransferTaskState>> = ref(new Map())
const monitors = new Map<string, ReturnType<typeof setInterval>>()

const POLL_MS = 1000
const TERMINAL: ReadonlySet<TransferTaskStatus> = new Set(['completed', 'failed', 'canceled'])

/** Reassign the Map so Map-backed refs trigger reactivity on mutation. */
function commit(): void {
  tasks.value = new Map(tasks.value)
}

function stopMonitor(id: string): void {
  const iv = monitors.get(id)
  if (iv !== undefined) {
    clearInterval(iv)
    monitors.delete(id)
  }
}

function applyRecord(id: string, rec: filesApi.TransferTaskRecord): void {
  const task = tasks.value.get(id)
  if (!task) return
  task.status = rec.status
  task.total_bytes = rec.total_bytes
  task.transferred_bytes = rec.transferred_bytes
  task.speed_bytes_per_sec = rec.speed_bytes_per_sec
  task.eta_seconds = rec.eta_seconds
  task.error = rec.error
  commit()
  if (TERMINAL.has(rec.status)) stopMonitor(id)
}

/**
 * Poll GET /api/files/transfer/{id} ~1s until a terminal status, updating the
 * shared store. Idempotent — repeated calls for the same id are ignored.
 */
export function monitor(id: string): void {
  if (monitors.has(id)) return
  const tick = async (): Promise<void> => {
    if (!tasks.value.has(id)) {
      stopMonitor(id)
      return
    }
    try {
      applyRecord(id, await filesApi.getTransferTask(id))
    } catch (e) {
      const task = tasks.value.get(id)
      if (task) {
        task.status = 'failed'
        task.error = e instanceof Error ? e.message : String(e)
        commit()
      }
      stopMonitor(id)
    }
  }
  monitors.set(id, setInterval(tick, POLL_MS))
  tick()
}

async function submit(
  op: TransferOp,
  src: TransferEndpoint,
  dst: TransferEndpoint,
  conflict: TransferConflict = 'overwrite',
): Promise<TransferTaskHandle> {
  const created = await filesApi.transferAction({ op, src, dst, conflict })
  tasks.value.set(created.id, {
    id: created.id,
    op,
    source_path: src.path,
    target_path: dst.path,
    status: 'pending',
    total_bytes: 0,
    transferred_bytes: 0,
    speed_bytes_per_sec: 0,
    eta_seconds: 0,
    error: null,
  })
  commit()
  monitor(created.id)
  return created
}

/** Server-side move (source is removed after the transfer completes). */
export function move(
  src: TransferEndpoint,
  dst: TransferEndpoint,
  conflict?: TransferConflict,
): Promise<TransferTaskHandle> {
  return submit('move', src, dst, conflict)
}

/** Server-side copy (source is preserved). */
export function copy(
  src: TransferEndpoint,
  dst: TransferEndpoint,
  conflict?: TransferConflict,
): Promise<TransferTaskHandle> {
  return submit('copy', src, dst, conflict)
}

/** Cancel a running/pending task server-side (POST /api/files/transfer/{id}/cancel). */
export async function cancel(id: string): Promise<void> {
  stopMonitor(id)
  const task = tasks.value.get(id)
  if (task) {
    task.status = 'canceled'
    commit()
  }
}

/** Drop completed/failed/canceled tasks from the shared store. */
export function dismissCompleted(): void {
  tasks.value.forEach((task, id) => {
    if (TERMINAL.has(task.status)) tasks.value.delete(id)
  })
  commit()
}

/**
 * Convenience accessor used by components. The store itself is module-level,
 * so every `useTransfer()` call sees the same reactive `tasks` map.
 */
export function useTransfer(): {
  tasks: Ref<Map<string, TransferTaskState>>
  move: typeof move
  copy: typeof copy
  monitor: typeof monitor
  cancel: typeof cancel
  dismissCompleted: typeof dismissCompleted
} {
  return { tasks, move, copy, monitor, cancel, dismissCompleted }
}
