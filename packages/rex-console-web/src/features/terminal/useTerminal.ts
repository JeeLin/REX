import { ref, shallowRef, onBeforeUnmount } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'

interface TerminalOptions {
  host: string
  port?: number
  username: string
  password?: string
  privateKey?: string
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

  /** 创建终端实例并挂载到 DOM */
  function createTerminal(container: HTMLElement, options?: Partial<Terminal['options']>) {
    const term = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
      theme: {
        background: '#0D1117',
        foreground: '#E6EDF3',
        cursor: '#E6EDF3',
        selectionBackground: '#264F78',
        black: '#484F58',
        red: '#FF7B72',
        green: '#3FB950',
        yellow: '#D29922',
        blue: '#58A6FF',
        magenta: '#BC8CFF',
        cyan: '#39C5CF',
        white: '#B1BAC4',
        brightBlack: '#6E7681',
        brightRed: '#FFA198',
        brightGreen: '#56D364',
        brightYellow: '#E3B341',
        brightBlue: '#79C0FF',
        brightMagenta: '#D2A8FF',
        brightCyan: '#56D4DD',
        brightWhite: '#F0F6FC',
      },
      allowProposedApi: true,
      ...options,
    })

    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(container)

    // 延迟一帧后 fit，确保 DOM 已渲染
    requestAnimationFrame(() => {
      fit.fit()
    })

    terminal.value = term
    fitAddon.value = fit

    return term
  }

  /** 连接到后端 WebSocket → SSH */
  function connect(opts: TerminalOptions) {
    if (ws) {
      ws.close()
      ws = null
    }

    status.value = 'connecting'
    errorMessage.value = ''

    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
    const wsUrl = `${protocol}//${location.host}/ws/terminal`
    ws = new WebSocket(wsUrl)

    ws.onopen = () => {
      // 发送 SSH 连接请求
      ws?.send(
        JSON.stringify({
          type: 'terminal.connect',
          host: opts.host,
          port: opts.port || 22,
          username: opts.username,
          password: opts.password,
          privateKey: opts.privateKey,
        }),
      )
    }

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data as string)
        switch (msg.type) {
          case 'terminal.connected':
            sessionId.value = msg.payload.sessionId
            status.value = 'connected'
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
      if (status.value !== 'error') {
        status.value = 'disconnected'
      }
    }

    ws.onerror = () => {
      status.value = 'error'
      errorMessage.value = 'WebSocket connection failed'
    }

    // 监听终端输入 → 发送到 WebSocket
    const term = terminal.value
    if (term) {
      term.onData((data) => {
        if (ws?.readyState === WebSocket.OPEN) {
          ws.send(
            JSON.stringify({
              type: 'terminal.data',
              data: btoa(data),
            }),
          )
        }
      })

      // 监听终端 resize → 发送到 WebSocket
      term.onResize(({ cols, rows }) => {
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
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'terminal.disconnect' }))
    }
    ws?.close()
    ws = null
    status.value = 'disconnected'
  }

  /** 调整终端大小（由 FitAddon 触发） */
  function fit() {
    fitAddon.value?.fit()
  }

  /** 销毁终端 */
  function dispose() {
    disposed = true
    disconnect()
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
    dispose,
  }
}
