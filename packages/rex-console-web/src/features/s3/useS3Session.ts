import { ref, onUnmounted } from 'vue'

export interface S3BucketInfo {
  name: string
  creation_date: string | null
}

export interface S3ObjectInfo {
  key: string
  size: number
  last_modified: string | null
  etag: string | null
  content_type: string | null
  storage_class: string | null
  is_dir: boolean
}

export type S3ServerMsg =
  | { type: 'connected'; server: Record<string, string> }
  | { type: 'response'; id: string; data: unknown }
  | { type: 'error'; id: string; message: string }
  | { type: 'pong' }
  | { type: 'disconnected'; reason: string }

export function useS3Session(resourceId: () => string) {
  // ── 状态 ────────────────────────────────────────────────
  const connected = ref(false)
  const serverInfo = ref<Record<string, string> | null>(null)
  const error = ref<string | null>(null)

  let ws: WebSocket | null = null
  const pendingCommands = new Map<string, {
    resolve: (value: S3ServerMsg) => void
    reject: (reason: Error) => void
  }>()
  let nextId = 0

  // ── 连接 ────────────────────────────────────────────────
  function connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (ws) {
        ws.close()
        ws = null
      }

      const token = localStorage.getItem('token') || ''
      const rid = resourceId()
      const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
      const url = `${protocol}//${location.host}/ws/s3/${rid}?token=${token}`

      ws = new WebSocket(url)

      ws.onopen = () => {
        error.value = null
      }

      ws.onmessage = (event) => {
        try {
          const msg: S3ServerMsg = JSON.parse(event.data)
          handleServerMsg(msg, resolve)
        } catch {
          // ignore parse errors
        }
      }

      ws.onerror = () => {
        error.value = 'WebSocket connection error'
        reject(new Error('WebSocket connection error'))
      }

      ws.onclose = () => {
        connected.value = false
        for (const [id, { reject: rej }] of pendingCommands) {
          rej(new Error('connection closed'))
          pendingCommands.delete(id)
        }
      }
    })
  }

  function handleServerMsg(
    msg: S3ServerMsg,
    connectResolve?: (value: void | PromiseLike<void>) => void,
  ) {
    switch (msg.type) {
      case 'connected':
        connected.value = true
        serverInfo.value = msg.server
        error.value = null
        connectResolve?.()
        break
      case 'response': {
        const pending = pendingCommands.get(msg.id)
        if (pending) {
          pending.resolve(msg)
          pendingCommands.delete(msg.id)
        }
        break
      }
      case 'error': {
        const pending = pendingCommands.get(msg.id)
        if (pending) {
          pending.resolve(msg)
          pendingCommands.delete(msg.id)
        }
        break
      }
      case 'pong':
        break
      case 'disconnected':
        connected.value = false
        error.value = msg.reason
        break
    }
  }

  // ── 断开连接 ────────────────────────────────────────────
  function disconnect() {
    if (ws) {
      ws.close()
      ws = null
    }
    connected.value = false
    serverInfo.value = null
  }

  // ── 发送命令 ────────────────────────────────────────────
  function sendCommand(action: string, params: Record<string, unknown> = {}): Promise<S3ServerMsg> {
    return new Promise((resolve, reject) => {
      if (!ws || ws.readyState !== WebSocket.OPEN) {
        reject(new Error('not connected'))
        return
      }

      const id = `cmd-${++nextId}`
      const msg = JSON.stringify({ type: 'command', id, action, params })

      pendingCommands.set(id, { resolve, reject })
      ws.send(msg)

      // 30s 超时
      setTimeout(() => {
        if (pendingCommands.has(id)) {
          pendingCommands.delete(id)
          reject(new Error('command timeout'))
        }
      }, 30000)
    })
  }

  // ── Bucket 操作 ─────────────────────────────────────────
  async function listBuckets(): Promise<S3BucketInfo[]> {
    const msg = await sendCommand('buckets')
    if (msg.type === 'response') {
      return Array.isArray(msg.data) ? (msg.data as S3BucketInfo[]) : []
    }
    throw new Error(msg.type === 'error' ? msg.message : 'unknown error')
  }

  // ── Object 操作 ─────────────────────────────────────────
  async function listObjects(bucket: string, prefix = ''): Promise<S3ObjectInfo[]> {
    const msg = await sendCommand('objects', { bucket, prefix })
    if (msg.type === 'response') {
      return Array.isArray(msg.data) ? (msg.data as S3ObjectInfo[]) : []
    }
    throw new Error(msg.type === 'error' ? msg.message : 'unknown error')
  }

  async function getObjectInfo(bucket: string, key: string): Promise<S3ObjectInfo> {
    const msg = await sendCommand('info', { bucket, key })
    if (msg.type === 'response') {
      return msg.data as S3ObjectInfo
    }
    throw new Error(msg.type === 'error' ? msg.message : 'unknown error')
  }

  async function uploadObject(bucket: string, key: string, data: ArrayBuffer): Promise<void> {
    const bytes = new Uint8Array(data)
    let binary = ''
    for (let i = 0; i < bytes.byteLength; i++) {
      binary += String.fromCharCode(bytes[i])
    }
    const b64 = btoa(binary)
    const msg = await sendCommand('upload', { bucket, key, data: b64 })
    if (msg.type === 'error') throw new Error(msg.message)
  }

  async function downloadObject(bucket: string, key: string): Promise<ArrayBuffer> {
    const msg = await sendCommand('download', { bucket, key })
    if (msg.type === 'response' && typeof msg.data === 'object' && msg.data !== null && 'data' in msg.data) {
      const b64 = (msg.data as { data: string }).data
      const binary = atob(b64)
      const bytes = new Uint8Array(binary.length)
      for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.charCodeAt(i)
      }
      return bytes.buffer
    }
    throw new Error(msg.type === 'error' ? msg.message : 'unknown error')
  }

  async function deleteObject(bucket: string, key: string): Promise<void> {
    const msg = await sendCommand('delete', { bucket, key })
    if (msg.type === 'error') throw new Error(msg.message)
  }

  // ── 清理 ────────────────────────────────────────────────
  onUnmounted(() => {
    disconnect()
  })

  return {
    connected,
    serverInfo,
    error,
    connect,
    disconnect,
    listBuckets,
    listObjects,
    getObjectInfo,
    uploadObject,
    downloadObject,
    deleteObject,
  }
}
