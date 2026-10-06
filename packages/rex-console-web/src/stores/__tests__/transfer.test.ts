import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useTransferStore } from '../transfer'
import type { TransferEndpoint, TransferTaskRecord } from '@/api/files'

const {
  mockTransferAction,
  mockGetTransferTask,
  mockCancelTransferTask,
  mockGetSyncTask,
  mockCancelSyncTask,
  mockUploadFile,
  mockDownloadFile,
} = vi.hoisted(() => ({
  mockTransferAction: vi.fn(),
  mockGetTransferTask: vi.fn(),
  mockCancelTransferTask: vi.fn(),
  mockGetSyncTask: vi.fn(),
  mockCancelSyncTask: vi.fn(),
  mockUploadFile: vi.fn(),
  mockDownloadFile: vi.fn(),
}))

vi.mock('@/api/files', () => ({
  transferAction: (...args: unknown[]) => mockTransferAction(...args),
  getTransferTask: (...args: unknown[]) => mockGetTransferTask(...args),
  cancelTransferTask: (...args: unknown[]) => mockCancelTransferTask(...args),
  getSyncTask: (...args: unknown[]) => mockGetSyncTask(...args),
  cancelSyncTask: (...args: unknown[]) => mockCancelSyncTask(...args),
  uploadFileWithProgress: (...args: unknown[]) => mockUploadFile(...args),
  downloadFile: (...args: unknown[]) => mockDownloadFile(...args),
}))

const POLL_MS = 1000

/** Minimal WebSocket double: the store only needs OPEN, send/close and handlers. */
class FakeSocket {
  static readonly OPEN = 1
  static instances: FakeSocket[] = []
  readyState = 0
  sent: string[] = []
  onmessage: ((ev: { data: string }) => void) | null = null
  onopen: (() => void) | null = null
  onclose: (() => void) | null = null
  onerror: (() => void) | null = null
  constructor(readonly url: string) {
    FakeSocket.instances.push(this)
  }
  send(data: string): void {
    this.sent.push(data)
  }
  close(): void {
    this.readyState = 3
    this.onclose?.()
  }
  /** Drive the handshake, which makes the store subscribe every tracked task. */
  open(): void {
    this.readyState = FakeSocket.OPEN
    this.onopen?.()
  }
  /** Deliver a `/ws/files` frame. */
  emit(frame: unknown): void {
    this.onmessage?.({ data: JSON.stringify(frame) })
  }
  subscriptions(): string[] {
    return this.sent
      .filter((s) => s.includes('"subscribe"'))
      .map((s) => (JSON.parse(s) as { task_id: string }).task_id)
  }
}

const SRC: TransferEndpoint = { resource_id: 'res-1', path: '/src/dir/' }
const DST: TransferEndpoint = { resource_id: 'res-1', path: '/dst/dir/' }

function record(over: Partial<TransferTaskRecord> = {}): TransferTaskRecord {
  return {
    id: 'sync-1',
    source_resource_id: 'res-1',
    target_resource_id: 'res-1',
    source_path: SRC.path,
    target_path: DST.path,
    conflict_policy: 'overwrite',
    kind: 'sync',
    status: 'pending',
    total_bytes: 0,
    transferred_bytes: 0,
    speed_bytes_per_sec: 0,
    eta_seconds: null,
    error: null,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...over,
  }
}

function progressFrame(status: string, total = 0, transferred = 0): unknown {
  return {
    type: 'progress',
    payload: {
      task_id: 'sync-1',
      status,
      total_bytes: total,
      transferred_bytes: transferred,
      speed_bytes_per_sec: 0,
      eta_seconds: null,
      error: null,
    },
  }
}

/** Let the immediate monitor tick settle before asserting on stored items. */
async function settle(): Promise<void> {
  await new Promise((r) => setTimeout(r, 0))
}

let originalWebSocket: unknown

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  FakeSocket.instances = []
  originalWebSocket = globalThis.WebSocket
  globalThis.WebSocket = FakeSocket as unknown as typeof WebSocket
  mockGetSyncTask.mockResolvedValue(record())
  mockGetTransferTask.mockResolvedValue(record({ id: 'tr-1', kind: 'transfer', status: 'running' }))
  mockTransferAction.mockResolvedValue({ id: 'tr-1', status: 'pending' })
})

afterEach(() => {
  globalThis.WebSocket = originalWebSocket as typeof WebSocket
  vi.useRealTimers()
})

describe('transfer store — folder sync tracking (v0.92.0)', () => {
  it('registers a created sync task as a server row with source → target', () => {
    const store = useTransferStore()

    const item = store.trackSync('sync-1', SRC, DST)

    expect(item.kind).toBe('server')
    expect(item.task_kind).toBe('sync')
    expect(item.status).toBe('pending')
    expect(item.phase).toBe('pending')
    expect(item.source_path).toBe('/src/dir/')
    expect(item.target_path).toBe('/dst/dir/')
    expect(item.name).toBe('dir')
    expect(store.tasks.get('sync-1')?.id).toBe('sync-1')
  })

  it('subscribes the sync task on the shared /ws/files channel', async () => {
    const store = useTransferStore()
    store.trackSync('sync-1', SRC, DST)
    await settle()

    const socket = FakeSocket.instances.at(-1)!
    expect(socket.url).toContain('/ws/files')
    socket.open()
    expect(socket.subscriptions()).toEqual(['sync-1'])
  })

  it('keeps the raw phase while every in-flight phase renders as running', async () => {
    const store = useTransferStore()
    store.trackSync('sync-1', SRC, DST)
    await settle()
    const socket = FakeSocket.instances.at(-1)!
    socket.open()

    for (const phase of ['scanning', 'planning', 'verifying'] as const) {
      socket.emit(progressFrame(phase))
      const item = store.tasks.get('sync-1')!
      expect(item.status).toBe('running')
      expect(item.phase).toBe(phase)
    }
  })

  it('updates bytes and percentage from progress events, then settles as done', async () => {
    const store = useTransferStore()
    store.trackSync('sync-1', SRC, DST)
    await settle()
    const socket = FakeSocket.instances.at(-1)!
    socket.open()

    socket.emit(progressFrame('running', 1000, 400))
    expect(store.tasks.get('sync-1')).toMatchObject({
      size: 1000,
      transferred: 400,
      progress: 40,
      phase: 'running',
    })

    socket.emit(progressFrame('completed', 1000, 1000))
    expect(store.tasks.get('sync-1')).toMatchObject({
      status: 'done',
      progress: 100,
      phase: 'completed',
    })
  })

  it('surfaces a failure with its message and the bytes already copied', async () => {
    const store = useTransferStore()
    store.trackSync('sync-1', SRC, DST)
    await settle()
    const socket = FakeSocket.instances.at(-1)!
    socket.open()

    socket.emit({
      type: 'progress',
      payload: {
        task_id: 'sync-1',
        status: 'failed',
        total_bytes: 1000,
        transferred_bytes: 250,
        speed_bytes_per_sec: 0,
        eta_seconds: null,
        error: 'copy failed: /dst/dir/b.bin',
      },
    })

    const item = store.tasks.get('sync-1')!
    expect(item.status).toBe('error')
    expect(item.progress).toBe(25)
    expect(item.error).toBe('copy failed: /dst/dir/b.bin')
  })

  it('polls the sync endpoint as the WS fallback, not the transfer one', async () => {
    vi.useFakeTimers()
    const store = useTransferStore()

    store.trackSync('sync-1', SRC, DST)
    await vi.advanceTimersByTimeAsync(0)
    expect(mockGetSyncTask).toHaveBeenCalledTimes(1)
    expect(mockGetTransferTask).not.toHaveBeenCalled()

    mockGetSyncTask.mockResolvedValue(record({ status: 'running', total_bytes: 10, transferred_bytes: 5 }))
    await vi.advanceTimersByTimeAsync(POLL_MS)

    expect(mockGetSyncTask).toHaveBeenCalledTimes(2)
    expect(mockGetTransferTask).not.toHaveBeenCalled()
    expect(store.tasks.get('sync-1')).toMatchObject({ status: 'running', progress: 50 })
  })

  it('cancels a sync task through DELETE /api/files/sync/{id}', async () => {
    const store = useTransferStore()
    store.trackSync('sync-1', SRC, DST)
    await settle()

    await store.cancel('sync-1')

    expect(mockCancelSyncTask).toHaveBeenCalledWith('sync-1')
    expect(store.tasks.get('sync-1')!.status).toBe('canceled')
  })

  it('keeps single-file transfers on the transfer endpoints', async () => {
    const store = useTransferStore()
    const item = await store.copy(SRC, DST)
    await settle()

    await store.cancel(item.id)

    expect(mockCancelTransferTask).toHaveBeenCalledWith('tr-1')
    expect(mockCancelSyncTask).not.toHaveBeenCalled()
    expect(store.tasks.get('tr-1')!.task_kind).toBe('transfer')
  })

  it('stops tracking once the task reaches a terminal status', async () => {
    vi.useFakeTimers()
    const store = useTransferStore()
    store.trackSync('sync-1', SRC, DST)
    await vi.advanceTimersByTimeAsync(0)

    mockGetSyncTask.mockResolvedValue(record({ status: 'completed', total_bytes: 8, transferred_bytes: 8 }))
    await vi.advanceTimersByTimeAsync(POLL_MS)
    expect(mockGetSyncTask).toHaveBeenCalledTimes(2)

    await vi.advanceTimersByTimeAsync(POLL_MS * 5)
    expect(mockGetSyncTask).toHaveBeenCalledTimes(2)
    expect(store.tasks.get('sync-1')!.status).toBe('done')
  })

  // The Hub sends `error` only on failed events (`skip_serializing_if`), so a
  // running/terminal-success frame must arrive with the key absent — and must
  // not wipe a reason already recorded on the row.
  it('keeps a recorded reason when a later frame carries no error key', async () => {
    const store = useTransferStore()
    store.trackSync('sync-1', SRC, DST)
    await settle()
    const socket = FakeSocket.instances.at(-1)!
    socket.open()

    socket.emit({
      type: 'progress',
      payload: {
        task_id: 'sync-1',
        status: 'failed',
        total_bytes: 10,
        transferred_bytes: 3,
        speed_bytes_per_sec: 0,
        eta_seconds: null,
        error: 'upload failed: connection reset',
      },
    })
    expect(store.tasks.get('sync-1')!.error).toBe('upload failed: connection reset')

    // Same shape the Hub emits without `error` (running / completed).
    const noErrorFrame = progressFrame('completed', 10, 10) as {
      payload: Record<string, unknown>
    }
    expect('error' in noErrorFrame.payload).toBe(true)
    delete noErrorFrame.payload.error
    socket.emit(noErrorFrame)

    expect(store.tasks.get('sync-1')!.error).toBe('upload failed: connection reset')
  })

  // Polling is the fallback when the WS never delivers the terminal frame (or
  // the socket is down). The reason comes from the task record, not the event,
  // so the row must still show why it failed.
  it('fills the failure reason from the polled task record when WS is silent', async () => {
    vi.useFakeTimers()
    const store = useTransferStore()
    store.trackSync('sync-1', SRC, DST)
    await vi.advanceTimersByTimeAsync(0)

    mockGetSyncTask.mockResolvedValue(
      record({
        status: 'failed',
        total_bytes: 10,
        transferred_bytes: 3,
        error: 'verify failed: size mismatch: src=10 dst=0',
      }),
    )
    await vi.advanceTimersByTimeAsync(POLL_MS)

    const item = store.tasks.get('sync-1')!
    expect(item.status).toBe('error')
    expect(item.error).toBe('verify failed: size mismatch: src=10 dst=0')
  })

  // The backend persists an empty string for "no error" (`error.unwrap_or("")`),
  // so the polled reason can be falsy — the row must fall back to its generic
  // "failed" label rather than rendering an empty cell.
  it('leaves the reason empty when the polled record has none', async () => {
    vi.useFakeTimers()
    const store = useTransferStore()
    store.trackSync('sync-1', SRC, DST)
    await vi.advanceTimersByTimeAsync(0)

    mockGetSyncTask.mockResolvedValue(record({ status: 'failed', error: '' }))
    await vi.advanceTimersByTimeAsync(POLL_MS)

    const item = store.tasks.get('sync-1')!
    expect(item.status).toBe('error')
    expect(item.error).toBeFalsy()
  })

  // Single-file transfers must get the same treatment as sync tasks.
  it('surfaces the failure reason on a single-file transfer row', async () => {
    vi.useFakeTimers()
    const store = useTransferStore()
    await store.copy(SRC, DST)
    await vi.advanceTimersByTimeAsync(0)

    const id = [...store.tasks.keys()][0]!
    mockGetTransferTask.mockResolvedValue({
      ...record({ id, kind: 'transfer' }),
      status: 'failed',
      error: 'source stat failed: no such file',
    })
    await vi.advanceTimersByTimeAsync(POLL_MS)

    expect(store.tasks.get(id)).toMatchObject({
      status: 'error',
      error: 'source stat failed: no such file',
    })
  })
})