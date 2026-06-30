import { ref, onUnmounted } from 'vue'

export interface SqliteResult {
  columns: string[]
  rows: unknown[][]
  affected_rows: number
  elapsed_ms: number
}

export interface SqliteColumnInfo {
  cid: number
  name: string
  type: string
  notnull: boolean
  default_value: string | null
  pk: boolean
}

type SqliteServerMsg =
  | { type: 'response'; id: string; data: unknown }
  | { type: 'error'; id: string; message: string }
  | { type: 'pong' }
  | { type: 'connected'; server: Record<string, string> }
  | { type: 'disconnected'; reason: string }

export function useSqliteSession(resourceId: () => string) {
  // ── 状态 ────────────────────────────────────────────────
  const connected = ref(false)
  const serverInfo = ref<Record<string, string> | null>(null)
  const error = ref<string | null>(null)

  let ws: WebSocket | null = null
  const pendingCommands = new Map<string, {
    resolve: (value: SqliteServerMsg) => void
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

      const token = localStorage.getItem('rex-token') || ''
      const rid = resourceId()
      const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
      const url = `${protocol}//${location.host}/ws/sqlite/${rid}?token=${token}`

      ws = new WebSocket(url)

      ws.onopen = () => {
        error.value = null
      }

      ws.onmessage = (event) => {
        try {
          const msg: SqliteServerMsg = JSON.parse(event.data)
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
    msg: SqliteServerMsg,
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
        error.value = msg.message
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
  function sendCommand(action: string, params: Record<string, unknown> = {}): Promise<SqliteServerMsg> {
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

  // ── SQL 操作 ────────────────────────────────────────────
  async function executeSql(sql: string): Promise<SqliteResult> {
    const msg = await sendCommand('execute', { sql })
    if (msg.type === 'response') return msg.data as SqliteResult
    throw new Error(msg.type === 'error' ? msg.message : 'unknown error')
  }

  async function listTables(): Promise<string[]> {
    const msg = await sendCommand('tables')
    if (msg.type === 'response') {
      const data = msg.data as { tables: string[] }
      return data.tables ?? []
    }
    throw new Error(msg.type === 'error' ? msg.message : 'unknown error')
  }

  async function getTableInfo(table: string): Promise<SqliteColumnInfo[]> {
    const msg = await sendCommand('columns', { table })
    if (msg.type === 'response') return msg.data as SqliteColumnInfo[]
    throw new Error(msg.type === 'error' ? msg.message : 'unknown error')
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
    executeSql,
    listTables,
    getTableInfo,
  }
}
