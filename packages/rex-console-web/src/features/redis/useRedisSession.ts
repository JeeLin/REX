import { ref, onUnmounted } from 'vue'
import type {
  RedisServerMsg,
  RedisHistoryEntry,
} from '@/api/redis'

// ── Redis 命令名列表（用于补全）────────────────────────────
const REDIS_COMMANDS = [
  'APPEND', 'AUTH', 'DBSIZE', 'DECR', 'DECRBY', 'DEL', 'ECHO',
  'EXISTS', 'EXPIRE', 'EXPIREAT', 'FLUSHALL', 'FLUSHDB', 'GET',
  'GETDEL', 'GETRANGE', 'GETSET', 'HDEL', 'HEXISTS', 'HGET',
  'HGETALL', 'HINCRBY', 'HKEYS', 'HLEN', 'HMGET', 'HMSET',
  'HSET', 'HSETNX', 'HVALS', 'INCR', 'INCRBY', 'INFO', 'KEYS',
  'LINDEX', 'LLEN', 'LPUSH', 'LPOP', 'LRANGE', 'MGET', 'MSET',
  'OBJECT', 'PEXPIRE', 'PING', 'PSETEX', 'PTTL', 'QUIT', 'RANDOMKEY',
  'RENAME', 'RESTORE', 'RPOP', 'RPUSH', 'SADD', 'SAVE', 'SCARD',
  'SDIFF', 'SET', 'SETEX', 'SETNX', 'SETRANGE', 'SHUTDOWN',
  'SINTER', 'SMEMBERS', 'SPOP', 'SRANDMEMBER', 'SREM',
  'STRLEN', 'SUBSCRIBE', 'SUBSTR', 'TTL', 'TYPE', 'UNSUBSCRIBE',
  'ZADD', 'ZCARD', 'ZCOUNT', 'ZINCRBY', 'ZRANGE', 'ZRANK',
  'ZREM', 'ZREVRANGE',
]

export function useRedisSession(resourceId: () => string) {
  // ── 状态 ────────────────────────────────────────────────
  const connected = ref(false)
  const serverInfo = ref<Record<string, string> | null>(null)
  const error = ref<string | null>(null)
  const history = ref<RedisHistoryEntry[]>([])
  const historyIndex = ref(-1)

  let ws: WebSocket | null = null
  const pendingCommands = new Map<string, {
    resolve: (value: RedisServerMsg) => void
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
      const url = `${protocol}//${location.host}/ws/redis/${rid}?token=${token}`

      ws = new WebSocket(url)

      ws.onopen = () => {
        error.value = null
      }

      ws.onmessage = (event) => {
        try {
          const msg: RedisServerMsg = JSON.parse(event.data)
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
        // reject all pending commands
        for (const [id, { reject: rej }] of pendingCommands) {
          rej(new Error('connection closed'))
          pendingCommands.delete(id)
        }
      }
    })
  }

  function handleServerMsg(
    msg: RedisServerMsg,
    connectResolve?: (value: void | PromiseLike<void>) => void,
  ) {
    switch (msg.type) {
      case 'connected':
        connected.value = true
        serverInfo.value = msg.server
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

  // ── 执行命令 ────────────────────────────────────────────
  function execute(command: string): Promise<RedisServerMsg> {
    return new Promise((resolve, reject) => {
      if (!ws || ws.readyState !== WebSocket.OPEN) {
        reject(new Error('not connected'))
        return
      }

      const id = `cmd-${++nextId}`
      const msg = JSON.stringify({ type: 'command', id, command })

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

  // ── 历史记录 ────────────────────────────────────────────
  function addToHistory(command: string) {
    history.value.unshift({
      id: `h-${Date.now()}`,
      command,
      timestamp: Date.now(),
    })
    // 保留最近 200 条
    if (history.value.length > 200) {
      history.value = history.value.slice(0, 200)
    }
    historyIndex.value = -1
  }

  function historyUp(): string | null {
    if (history.value.length === 0) return null
    historyIndex.value = Math.min(
      historyIndex.value + 1,
      history.value.length - 1,
    )
    return history.value[historyIndex.value]?.command ?? null
  }

  function historyDown(): string | null {
    if (historyIndex.value <= 0) {
      historyIndex.value = -1
      return ''
    }
    historyIndex.value--
    return history.value[historyIndex.value]?.command ?? null
  }

  function clearHistory() {
    history.value = []
    historyIndex.value = -1
  }

  // ── 清理 ────────────────────────────────────────────────
  onUnmounted(() => {
    disconnect()
  })

  return {
    connected,
    serverInfo,
    error,
    history,
    connect,
    disconnect,
    execute,
    addToHistory,
    historyUp,
    historyDown,
    clearHistory,
    REDIS_COMMANDS,
  }
}
