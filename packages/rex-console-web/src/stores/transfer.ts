//! Unified file-transfer store (v0.91.0 T5).
//! Single source of truth for both server-side tasks (created via the Hub
//! HTTP API, polled via GET /api/files/transfer/{id}) and browser-blob local
//! transfers (upload/download that push bytes directly from the browser).
//!
//! Server-side tasks: a task is created via POST /api/files/transfer/action and
//! progress is polled from GET /api/files/transfer/{id}. File bytes never
//! transit the browser. Cancellation is best-effort via POST /cancel.
//!
//! Browser-blob tasks: local panel I/O (upload from browser → remote, or
//! download from remote → browser). Progress is driven by XHR upload/download
//! events and pushed into the same queue via updateBrowserTask.

import { ref, type Ref } from 'vue'
import { defineStore } from 'pinia'
import * as filesApi from '@/api/files'
import type {
  TransferConflict,
  TransferEndpoint,
  TransferOp,
  TransferTaskRecord,
} from '@/api/files'

// --- preserved from original useTransfer (fields unchanged) ---

export interface TransferTaskState {
  id: string
  op: TransferOp
  source_path: string
  target_path: string
  status: filesApi.TransferTaskStatus
  total_bytes: number
  transferred_bytes: number
  speed_bytes_per_sec: number
  eta_seconds: number | null
  error: string | null
}

// --- unified TransferItem model (server + browser) ---

export type TransferKind = 'server' | 'browser'
export type TransferDirection = 'up' | 'down' | 'copy'
export type TransferStatus = 'pending' | 'running' | 'paused' | 'done' | 'error' | 'canceled'

export interface TransferItem {
  id: string
  kind: TransferKind
  direction: TransferDirection
  name: string
  size: number
  transferred: number
  progress: number
  speed: number
  status: TransferStatus
  eta?: number | null
  source_path?: string
  target_path?: string
  op?: TransferOp
  task_id?: string
  error?: string | null
  // browser-only
  file?: File
  xhr?: XMLHttpRequest
  uploadId?: string
  sessionId?: string
}

const TERMINAL: ReadonlySet<TransferStatus> = new Set(['done', 'error', 'canceled'])
const POLL_MS = 1000

// --- WebSocket-driven updates (T5.5) ---
// The Hub exposes `/ws/files?resource_id=<id>&token=<jwt>`. Frames are text JSON:
//
//   { "type": "transfer.progress", "task_id": "...", "total_bytes": ..., "transferred_bytes": ..., "speed_bytes_per_sec": ..., "eta_seconds": ... }
//   { "type": "transfer.done",     "task_id": "...", "status": "completed" | "failed" | "canceled", "error": "..." | null }
//
// WS is the *preferred* live channel; polling (`monitor`) always remains as a
// fallback so a dropped WS connection never stalls task tracking.

export type FilesWsMessage =
  | {
      type: 'transfer.progress'
      task_id: string
      total_bytes?: number
      transferred_bytes?: number
      speed_bytes_per_sec?: number
      eta_seconds?: number | null
      error?: string | null
    }
  | {
      type: 'transfer.done'
      task_id: string
      status: filesApi.TransferTaskStatus
      error?: string | null
    }

function isFilesWsMessage(v: unknown): v is FilesWsMessage {
  return (
    typeof v === 'object' &&
    v !== null &&
    typeof (v as { type?: unknown }).type === 'string'
  )
}

/** Build a WebSocket URL using the same scheme/host as the page (Hub origin). */
function filesWsUrl(resourceId: string): string {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws'
  const token = localStorage.getItem('rex-token') || ''
  return `${proto}//${location.host}/ws/files?resource_id=${encodeURIComponent(resourceId)}&token=${encodeURIComponent(token)}`
}

/** Map server-side status enum to unified status enum. */
function mapServerStatus(status: filesApi.TransferTaskStatus): TransferStatus {
  switch (status) {
    case 'completed':
      return 'done'
    case 'failed':
      return 'error'
    default:
      return status
  }
}

function baseName(path: string): string {
  return path.split('/').pop() || path
}

export const useTransferStore = defineStore('transfer', () => {
  const tasks: Ref<Map<string, TransferItem>> = ref(new Map())
  const monitors = new Map<string, ReturnType<typeof setInterval>>()
  // Per-resource WebSocket sockets keyed by resource_id. Multiple FilesPage
  // instances (one per open tab) may connect different resources; each gets its
  // own socket so frames route to the right tasks.
  const wsConnections = new Map<string, WebSocket>()

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

  /** Apply a WebSocket frame to the matching task (server tasks only). */
  function handleWsMessage(raw: string): void {
    let msg: FilesWsMessage
    try {
      const parsed: unknown = JSON.parse(raw)
      if (!isFilesWsMessage(parsed)) return
      msg = parsed
    } catch {
      return
    }
    if (msg.type === 'transfer.progress') {
      applyRecord(msg.task_id, {
        total_bytes: msg.total_bytes,
        transferred_bytes: msg.transferred_bytes,
        speed_bytes_per_sec: msg.speed_bytes_per_sec,
        eta_seconds: msg.eta_seconds,
        error: msg.error,
      })
    } else if (msg.type === 'transfer.done') {
      applyRecord(msg.task_id, {
        status: msg.status,
        error: msg.error,
      })
    }
  }

  /**
   * Open a WebSocket to `/ws/files` for a resource so the store is updated in
   * real time. Safe to call repeatedly — a connection is only created once per
   * resource while it stays open. If the endpoint is unavailable the polling
   * `monitor` fallback keeps tasks in sync.
   */
  function connectWs(resourceId: string): void {
    const existing = wsConnections.get(resourceId)
    if (existing && existing.readyState === WebSocket.OPEN) return
    if (existing) wsConnections.delete(resourceId)
    let socket: WebSocket
    try {
      socket = new WebSocket(filesWsUrl(resourceId))
    } catch {
      // Non-browser/construct failure: rely on polling fallback.
      return
    }
    wsConnections.set(resourceId, socket)
    socket.onmessage = (ev: MessageEvent) => {
      if (typeof ev.data === 'string') handleWsMessage(ev.data)
    }
    socket.onclose = () => {
      wsConnections.delete(resourceId)
    }
    socket.onerror = () => {
      socket.close()
      wsConnections.delete(resourceId)
    }
  }

  /** Close (and forget) the WebSocket for a resource, if any. */
  function disconnectWs(resourceId: string): void {
    const socket = wsConnections.get(resourceId)
    if (socket) {
      socket.close()
      wsConnections.delete(resourceId)
    }
  }

  // Accepts a *partial* record so it can be driven both by the polling
  // endpoint (full TransferTaskRecord) and by WebSocket progress/done frames
  // (only the fields that changed).
  function applyRecord(id: string, rec: Partial<TransferTaskRecord>): void {
    const task = tasks.value.get(id)
    if (!task || task.kind !== 'server') return
    if (rec.status !== undefined) task.status = mapServerStatus(rec.status)
    if (rec.total_bytes !== undefined) task.size = rec.total_bytes
    if (rec.transferred_bytes !== undefined) task.transferred = rec.transferred_bytes
    if (rec.speed_bytes_per_sec !== undefined) task.speed = rec.speed_bytes_per_sec
    if (rec.eta_seconds !== undefined) task.eta = rec.eta_seconds
    if (rec.error !== undefined) task.error = rec.error
    if (task.size > 0) {
      task.progress = Math.round((task.transferred / task.size) * 100)
    }
    commit()
    if (TERMINAL.has(task.status)) stopMonitor(id)
  }

  /**
   * Poll GET /api/files/transfer/{id} ~1s until a terminal status, updating the
   * shared store. Idempotent — repeated calls for the same id are ignored.
   */
  async function monitor(id: string): Promise<void> {
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
          task.status = 'error'
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
  ): Promise<TransferItem> {
    const created = await filesApi.transferAction({ op, src, dst, conflict })
    const item: TransferItem = {
      id: created.id,
      kind: 'server',
      direction: 'copy',
      name: baseName(dst.path),
      size: 0,
      transferred: 0,
      progress: 0,
      speed: 0,
      status: 'pending',
      eta: null,
      source_path: src.path,
      target_path: dst.path,
      op,
      task_id: created.id,
      error: null,
    }
    tasks.value.set(created.id, item)
    commit()
    monitor(created.id)
    return item
  }

  /** Server-side move (source is removed after the transfer completes). */
  function move(
    src: TransferEndpoint,
    dst: TransferEndpoint,
    conflict?: TransferConflict,
  ): Promise<TransferItem> {
    return submit('move', src, dst, conflict)
  }

  /** Server-side copy (source is preserved). */
  function copy(
    src: TransferEndpoint,
    dst: TransferEndpoint,
    conflict?: TransferConflict,
  ): Promise<TransferItem> {
    return submit('copy', src, dst, conflict)
  }

  /** Cancel a running/pending task. Server tasks notify the backend;
   * browser tasks abort the in-flight XHR if available. */
  async function cancel(id: string): Promise<void> {
    stopMonitor(id)
    const task = tasks.value.get(id)
    if (!task) return
    if (task.kind === 'server' && task.task_id) {
      try {
        await filesApi.cancelTransferTask(task.task_id)
      } catch (e) {
        console.error('Cancel request failed:', e)
      }
    }
    if (task.xhr) {
      task.xhr.abort()
    }
    task.status = 'canceled'
    commit()
  }

  /** Drop completed/failed/canceled tasks from the shared store. */
  function dismissCompleted(): void {
    tasks.value.forEach((task, id) => {
      if (TERMINAL.has(task.status)) tasks.value.delete(id)
    })
    commit()
  }

  /* --- browser-blob local-transfer tracking (progress-driven) --- */

  /** Push a browser-blob transfer item into the unified queue. */
  function pushBrowserTask(item: Partial<TransferItem> & { id: string }): TransferItem {
    const full: TransferItem = {
      kind: 'browser',
      direction: 'up',
      name: '',
      size: 0,
      transferred: 0,
      progress: 0,
      speed: 0,
      status: 'pending',
      eta: null,
      error: null,
      ...item,
    }
    tasks.value.set(full.id, full)
    commit()
    return full
  }

  /** Update fields on a browser-blob transfer item in the unified queue. */
  function updateBrowserTask(id: string, updates: Partial<TransferItem>): void {
    const task = tasks.value.get(id)
    if (!task) return
    Object.assign(task, updates)
    commit()
  }

  return {
    tasks,
    pushBrowserTask,
    updateBrowserTask,
    move,
    copy,
    monitor,
    cancel,
    dismissCompleted,
    connectWs,
    disconnectWs,
  }
})
