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
//!
//! T5.5: A WebSocket connection to `/ws/files` carries live progress broadcasts
//! from TransferCoordinator. WS is preferred; HTTP polling remains as fallback.

import { ref, type Ref } from 'vue'
import { defineStore } from 'pinia'
import * as filesApi from '@/api/files'
import type {
  TransferConflict,
  TransferEndpoint,
  TransferOp,
  TransferTaskKind,
  TransferTaskRecord,
  TransferTaskStatus,
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
  /** Server task kind (`transfer_task.kind`): transfer (single file) | sync (v0.92.0). */
  task_kind?: TransferTaskKind
  /**
   * Raw server status, kept alongside the coarse `status`: sync phases
   * (scanning / planning / verifying) all map to `running`, so the queue shows
   * the real phase word instead of a generic "running".
   */
  phase?: TransferTaskStatus
  error?: string | null
  // browser-only
  file?: File
  xhr?: XMLHttpRequest
  uploadId?: string
  sessionId?: string
}

const TERMINAL: ReadonlySet<TransferStatus> = new Set(['done', 'error', 'canceled'])
const POLL_MS = 1000
const RECONNECT_BASE_MS = 1000
const RECONNECT_MAX_MS = 30000
const RECONNECT_FACTOR = 1.5

/** Progress event embedded in the `payload` field of a backend WS frame. */
interface WsProgressPayload {
  task_id: string
  transferred_bytes: number
  total_bytes: number
  speed_bytes_per_sec: number
  status: string
  eta_seconds?: number | null
  error?: string | null
}

/** Normalized WS message: both backend `progress` and spec `transfer.*` shapes. */
type WsMessage =
  | { type: 'progress'; payload: WsProgressPayload }
  | { type: 'transfer.progress'; task_id: string } & Partial<WsProgressPayload>
  | { type: 'transfer.done'; task_id: string } & Partial<WsProgressPayload>

/** Build a WebSocket URL using the same scheme/host as the page (Hub origin). */
function filesWsUrl(): string {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws'
  const token = localStorage.getItem('rex-token') || ''
  return `${proto}//${location.host}/ws/files?token=${encodeURIComponent(token)}`
}

/** Map server-side status enum to unified status enum. */
function mapServerStatus(status: filesApi.TransferTaskStatus): TransferStatus {
  switch (status) {
    case 'completed':
      return 'done'
    case 'failed':
      return 'error'
    // Sync phases (scanning/planning/verifying) and paused states all render
    // as an in-flight row; canceling is on its way to canceled.
    case 'scanning':
    case 'planning':
    case 'verifying':
      return 'running'
    case 'canceled':
      return 'canceled'
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
  const ws: Ref<WebSocket | null> = ref(null)

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

  /** Normalize either backend `progress` or spec `transfer.*` frame into a task_id + partial record. */
  function normalizeWsMessage(msg: WsMessage): { task_id: string; record: Partial<TransferTaskRecord> } | null {
    if (msg.type === 'progress' && msg.payload) {
      const p = msg.payload
      return { task_id: p.task_id, record: { status: p.status as filesApi.TransferTaskStatus, total_bytes: p.total_bytes, transferred_bytes: p.transferred_bytes, speed_bytes_per_sec: p.speed_bytes_per_sec, eta_seconds: p.eta_seconds, error: p.error } }
    }
    // spec format: { type: 'transfer.progress'|'transfer.done', task_id, ...fields }
    const { task_id, type, ...rest } = msg as { type: string; task_id: string } & Partial<WsProgressPayload>
    if (!task_id) return null
    const isDone = type === 'transfer.done'
    return {
      task_id,
      record: {
        // transfer.done is terminal; default to completed if no explicit status given.
        status: isDone && !rest.status
          ? ('completed' satisfies filesApi.TransferTaskStatus)
          : (rest.status as filesApi.TransferTaskStatus | undefined),
        total_bytes: rest.total_bytes,
        transferred_bytes: rest.transferred_bytes,
        speed_bytes_per_sec: rest.speed_bytes_per_sec,
        eta_seconds: rest.eta_seconds,
        error: rest.error,
      },
    }
  }

  let reconnectAttempts = 0
  let wsIntentionallyClosed = false

  /**
   * Open a WebSocket to `/ws/files` and subscribe every server task that is
   * currently tracked. Safe to call repeatedly — a single socket is reused.
   * If the endpoint is unavailable the polling `monitor` fallback keeps tasks
   * in sync. On disconnect, reconnects with exponential backoff (1s → 1.5x → 30s cap).
   */
  function connectWs(): void {
    if (ws.value && ws.value.readyState === WebSocket.OPEN) return
    if (ws.value) {
      ws.value.close()
      ws.value = null
    }
    wsIntentionallyClosed = false
    let socket: WebSocket
    try {
      socket = new WebSocket(filesWsUrl())
    } catch {
      // Non-browser/construct failure: rely on polling fallback.
      reconnectWs()
      return
    }
    ws.value = socket
    socket.onmessage = (ev: MessageEvent) => {
      if (typeof ev.data !== 'string') return
      let msg: unknown
      try {
        msg = JSON.parse(ev.data)
      } catch {
        return
      }
      const normalized = normalizeWsMessage(msg as WsMessage)
      if (normalized) applyRecord(normalized.task_id, normalized.record)
    }
    socket.onopen = () => {
      // Reset backoff on successful connection.
      reconnectAttempts = 0
      // Subscribe to every tracked server task.
      tasks.value.forEach((t) => {
        if (t.kind === 'server' && t.task_id) {
          socket.send(JSON.stringify({ type: 'subscribe', task_id: t.task_id }))
        }
      })
    }
    socket.onclose = () => {
      ws.value = null
      if (!wsIntentionallyClosed) reconnectWs()
    }
    socket.onerror = () => {
      socket.close()
      ws.value = null
      if (!wsIntentionallyClosed) reconnectWs()
    }
  }

  /** Reconnect with exponential backoff: 1s, 1.5x, capped at 30s. */
  function reconnectWs(): void {
    reconnectAttempts++
    const delay = Math.min(
      RECONNECT_BASE_MS * Math.pow(RECONNECT_FACTOR, reconnectAttempts - 1),
      RECONNECT_MAX_MS,
    )
    setTimeout(() => connectWs(), delay)
  }

  /** Close the WebSocket and stop reconnecting. */
  function disconnectWs(): void {
    wsIntentionallyClosed = true
    if (ws.value) {
      ws.value.close()
      ws.value = null
    }
    reconnectAttempts = 0
  }

  // Accepts a *partial* record so it can be driven both by the polling
  // endpoint (full TransferTaskRecord) and by WebSocket progress frames
  // (only the fields that changed).
  function applyRecord(id: string, rec: Partial<TransferTaskRecord>): void {
    const task = tasks.value.get(id)
    if (!task || task.kind !== 'server') return
    if (rec.kind !== undefined) task.task_kind = rec.kind
    if (rec.status !== undefined) {
      task.status = mapServerStatus(rec.status)
      // 同步的扫描/规划/校验阶段都塌缩成 `running`，原始状态留在 `phase` 上。
      task.phase = rec.status
    }
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
   * Track an already-created sync task (POST /api/files/sync done by the caller).
   * Progress arrives over the shared `/ws/files` channel, with ~1s polling
   * (GET /api/files/sync/{id}) as the fallback. File bytes never transit the
   * browser — the Hub diffs and copies between the two connectors.
   */
  function trackSync(
    id: string,
    source: TransferEndpoint,
    target: TransferEndpoint,
  ): TransferItem {
    const item: TransferItem = {
      id,
      kind: 'server',
      // A sync has no single-file direction: the phase/direction words come
      // from `phase` and the source → target path pair.
      direction: 'copy',
      // Target roots are directories (`/dst/dir/`): trim the trailing slash so
      // the row name is the directory name instead of the whole path again.
      name: baseName(target.path.replace(/\/+$/, '')),
      size: 0,
      transferred: 0,
      progress: 0,
      speed: 0,
      status: 'pending',
      eta: null,
      source_path: source.path,
      target_path: target.path,
      task_id: id,
      task_kind: 'sync',
      phase: 'pending',
      error: null,
    }
    tasks.value.set(id, item)
    commit()
    monitor(id)
    return item
  }

  /**
   * Poll the task ~1s until a terminal status, updating the shared store.
   * Idempotent — repeated calls for the same id are ignored. Sync tasks poll
   * their own endpoint so the queue keeps working if transfer routes change.
   * Also opens the WS socket so progress arrives in real time.
   */
  async function monitor(id: string): Promise<void> {
    if (monitors.has(id)) return
    const tick = async (): Promise<void> => {
      if (!tasks.value.has(id)) {
        stopMonitor(id)
        return
      }
      try {
        const isSync = tasks.value.get(id)?.task_kind === 'sync'
        applyRecord(id, await (isSync ? filesApi.getSyncTask(id) : filesApi.getTransferTask(id)))
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
    // Lazily open WS so a single socket serves all tracked tasks.
    connectWs()
    // If WS already open, subscribe this specific task now.
    if (ws.value && ws.value.readyState === WebSocket.OPEN) {
      ws.value.send(JSON.stringify({ type: 'subscribe', task_id: id }))
    }
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
      task_kind: 'transfer',
      phase: 'pending',
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

  /** Cancel a running/pending task. Server tasks notify the backend (sync tasks
   * through DELETE /api/files/sync/{id}); browser tasks abort the in-flight XHR
   * if available. */
  async function cancel(id: string): Promise<void> {
    stopMonitor(id)
    const task = tasks.value.get(id)
    if (!task) return
    if (task.kind === 'server' && task.task_id) {
      try {
        if (task.task_kind === 'sync') {
          await filesApi.cancelSyncTask(task.task_id)
        } else {
          await filesApi.cancelTransferTask(task.task_id)
        }
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
    trackSync,
    monitor,
    cancel,
    dismissCompleted,
    connectWs,
    reconnectWs,
    disconnectWs,
  }
})
