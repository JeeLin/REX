import { ref, onUnmounted } from 'vue'
import type {
  DockerServerMsg,
  DockerContainerInfo,
} from '@/api/docker'

export interface DockerLogEntry {
  id: string
  text: string
  timestamp: number
}

export function useDockerSession(resourceId: () => string) {
  // ── 状态 ────────────────────────────────────────────────
  const connected = ref(false)
  const serverInfo = ref<Record<string, string> | null>(null)
  const error = ref<string | null>(null)
  const containers = ref<DockerContainerInfo[]>([])

  let ws: WebSocket | null = null
  const pendingCommands = new Map<string, {
    resolve: (value: DockerServerMsg) => void
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
      const url = `${protocol}//${location.host}/ws/docker/${rid}?token=${token}`

      ws = new WebSocket(url)

      ws.onopen = () => {
        error.value = null
      }

      ws.onmessage = (event) => {
        try {
          const msg: DockerServerMsg = JSON.parse(event.data)
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
    msg: DockerServerMsg,
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
    containers.value = []
  }

  // ── 发送命令 ────────────────────────────────────────────
  function sendCommand(action: string, params: Record<string, unknown> = {}): Promise<DockerServerMsg> {
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

  // ── 容器操作 ────────────────────────────────────────────
  async function listContainers(all = false): Promise<DockerContainerInfo[]> {
    const msg = await sendCommand('list', { all })
    if (msg.type === 'response') {
      const list = Array.isArray(msg.data) ? msg.data : []
      containers.value = list as DockerContainerInfo[]
      return containers.value
    }
    throw new Error(msg.type === 'error' ? msg.message : 'unknown error')
  }

  async function inspectContainer(id: string): Promise<unknown> {
    const msg = await sendCommand('inspect', { id })
    if (msg.type === 'response') return msg.data
    throw new Error(msg.type === 'error' ? msg.message : 'unknown error')
  }

  async function startContainer(id: string): Promise<void> {
    const msg = await sendCommand('start', { id })
    if (msg.type === 'error') throw new Error(msg.message)
  }

  async function stopContainer(id: string): Promise<void> {
    const msg = await sendCommand('stop', { id })
    if (msg.type === 'error') throw new Error(msg.message)
  }

  async function restartContainer(id: string): Promise<void> {
    const msg = await sendCommand('restart', { id })
    if (msg.type === 'error') throw new Error(msg.message)
  }

  async function removeContainer(id: string): Promise<void> {
    const msg = await sendCommand('remove', { id })
    if (msg.type === 'error') throw new Error(msg.message)
  }

  async function getLogs(id: string, tail = 100): Promise<string> {
    const msg = await sendCommand('logs', { id, tail })
    if (msg.type === 'response' && typeof msg.data === 'object' && msg.data !== null && 'logs' in msg.data) {
      return (msg.data as { logs: string }).logs
    }
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
    containers,
    connect,
    disconnect,
    listContainers,
    inspectContainer,
    startContainer,
    stopContainer,
    restartContainer,
    removeContainer,
    getLogs,
  }
}
