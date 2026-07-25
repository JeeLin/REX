import { ref, shallowRef, onBeforeUnmount } from 'vue'
import { Terminal, type IDisposable } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { getTerminalTheme } from './terminal-themes'

interface TerminalOptions {
  /** 资源 ID — Hub 从 DB 读取所有连接信息，自动判断直连/Agent 隧道 */
  resourceId: string
}

type TerminalStatus = 'disconnected' | 'connecting' | 'connected' | 'error'

export function useTerminal() {
  const terminal = shallowRef<Terminal | null>(null)
  const fitAddon = shallowRef<FitAddon | null>(null)
  const status = ref<TerminalStatus>('disconnected')
  const errorMessage = ref('')
  const sessionId = ref('')

  let ws: WebSocket | null = null
  let disposed = false
  let dataSub: IDisposable | null = null
  let resizeSub: IDisposable | null = null
  let heartbeatTimer: ReturnType<typeof setInterval> | null = null

  /** 创建终端实例并挂载到 DOM */
  function createTerminal(container: HTMLElement, options?: Partial<Terminal['options']>) {
    const term = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
      theme: getTerminalTheme('default'),
      allowProposedApi: true,
      ...options,
    })

    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(container)

    requestAnimationFrame(() => {
      fit.fit()
    })

    terminal.value = term
    fitAddon.value = fit

    return term
  }

  /** 启动心跳（每30秒发送 ping） */
  function startHeartbeat() {
    stopHeartbeat()
    heartbeatTimer = setInterval(() => {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'ping' }))
      }
    }, 30000)
  }

  /** 停止心跳 */
  function stopHeartbeat() {
    if (heartbeatTimer) {
      clearInterval(heartbeatTimer)
      heartbeatTimer = null
    }
  }

  /** 连接到后端 WebSocket → SSH（Hub 自动判断直连/Agent） */
  function connect(opts: TerminalOptions) {
    if (ws) {
      ws.close()
      ws = null
    }

    dataSub?.dispose()
    resizeSub?.dispose()
    dataSub = null
    resizeSub = null
    stopHeartbeat()

    status.value = 'connecting'
    errorMessage.value = ''

    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
    const token = localStorage.getItem('rex-token') || ''
    const wsUrl = `${protocol}//${location.host}/ws/terminal?resourceId=${encodeURIComponent(opts.resourceId)}&token=${encodeURIComponent(token)}`
    ws = new WebSocket(wsUrl)

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data as string)
        switch (msg.type) {
          case 'terminal.connected':
            sessionId.value = msg.payload.sessionId
            status.value = 'connected'
            startHeartbeat()
            // SSH starts at 80x24 — send actual size immediately
            if (terminal.value && ws?.readyState === WebSocket.OPEN) {
              ws.send(JSON.stringify({
                type: 'terminal.resize',
                cols: terminal.value.cols,
                rows: terminal.value.rows,
              }))
            }
            break
          case 'terminal.data': {
            const data = atob(msg.payload.data)
            terminal.value?.write(data)
            break
          }
          case 'terminal.disconnected':
            status.value = 'disconnected'
            terminal.value?.writeln('\r\n\x1b[33m[Session disconnected: ' + msg.payload.reason + ']\x1b[0m')
            break
          case 'terminal.error':
            status.value = 'error'
            errorMessage.value = msg.payload.message
            terminal.value?.writeln('\r\n\x1b[31m[Error: ' + msg.payload.message + ']\x1b[0m')
            break
        }
      } catch {
        // 忽略非 JSON 消息
      }
    }

    ws.onclose = () => {
      stopHeartbeat()
      if (status.value !== 'error') {
        status.value = 'disconnected'
      }
    }

    ws.onerror = () => {
      stopHeartbeat()
      status.value = 'error'
      errorMessage.value = 'WebSocket connection failed'
    }

    // 终端输入 → WebSocket
    const term = terminal.value
    if (term) {
      dataSub = term.onData((data) => {
        if (ws?.readyState === WebSocket.OPEN) {
          ws.send(
            JSON.stringify({
              type: 'terminal.data',
              data: btoa(data),
            }),
          )
        }
      })

      resizeSub = term.onResize(({ cols, rows }) => {
        if (ws?.readyState === WebSocket.OPEN) {
          ws.send(
            JSON.stringify({
              type: 'terminal.resize',
              cols,
              rows,
            }),
          )
        }
      })
    }
  }

  /** 断开连接 */
  function disconnect() {
    stopHeartbeat()
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'terminal.disconnect' }))
    }
    ws?.close()
    ws = null
    status.value = 'disconnected'
  }

  /** 调整终端大小 */
  function fit() {
    fitAddon.value?.fit()
  }

  /** 切换终端主题 */
  function setTheme(themeName: string) {
    const theme = getTerminalTheme(themeName)
    if (terminal.value) {
      terminal.value.options.theme = theme
    }
  }

  /** 销毁终端 */
  function dispose() {
    disposed = true
    stopHeartbeat()
    disconnect()
    dataSub?.dispose()
    resizeSub?.dispose()
    dataSub = null
    resizeSub = null
    terminal.value?.dispose()
    terminal.value = null
    fitAddon.value = null
  }

  onBeforeUnmount(() => {
    if (!disposed) dispose()
  })

  return {
    terminal,
    fitAddon,
    status,
    errorMessage,
    sessionId,
    createTerminal,
    connect,
    disconnect,
    fit,
    setTheme,
    dispose,
  }
}
