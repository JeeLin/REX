<template>
  <div class="ws-terminal">
    <!-- 工具栏 -->
    <div class="ws-term-toolbar">
      <div class="ws-term-info">
        <span class="ws-term-status" :class="connectionStatus">●</span>
        <span class="ws-term-name">{{ resourceName }}</span>
      </div>
      <div class="ws-term-spacer"></div>
      <div class="ws-term-actions">
        <button class="btn btn-ghost btn-xs" @click="clearTerminal">清屏</button>
        <button class="btn btn-ghost btn-xs" @click="handlePaste">粘贴</button>
        <button class="btn btn-xs btn-danger" @click="showDisconnectDialog = true">断开</button>
      </div>
    </div>

    <!-- 终端容器 -->
    <div ref="terminalContainer" class="ws-term-container"></div>

    <!-- 未连接时显示重连提示 -->
    <div v-if="connectionStatus === 'disconnected'" class="ws-term-reconnect">
      <div class="reconnect-icon">⚡</div>
      <div class="reconnect-text">连接已断开</div>
      <button class="btn btn-sm btn-primary" @click="connectSession">重新连接</button>
    </div>

    <!-- 状态栏 -->
    <div class="ws-term-statusbar">
      <span>SSH</span>
      <span>·</span>
      <span>UTF-8</span>
      <span class="spacer"></span>
      <span v-if="connectionStatus === 'connected'" style="color: #000">已连接</span>
      <span v-else-if="connectionStatus === 'connecting'" style="color: #000">连接中...</span>
      <span v-else style="color: #000">未连接</span>
    </div>

    <!-- 断开确认弹窗 -->
    <div v-if="showDisconnectDialog" class="ws-term-modal-overlay" @click.self="showDisconnectDialog = false">
      <div class="ws-term-modal">
        <div class="ws-term-modal-title">断开连接？</div>
        <p class="ws-term-modal-desc">断开后当前会话将终止，未保存的工作可能会丢失。</p>
        <div class="ws-term-modal-actions">
          <button class="btn" @click="showDisconnectDialog = false">取消</button>
          <button class="btn btn-danger" @click="doDisconnect">断开</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import { createSession, deleteSession } from '@/api/terminal'

const props = defineProps<{
  resourceId: string
  resourceName: string
}>()

const emit = defineEmits<{
  (e: 'disconnect'): void
  (e: 'error', msg: string): void
}>()

const terminalContainer = ref<HTMLElement>()
const connectionStatus = ref<'connecting' | 'connected' | 'disconnected'>('disconnected')
const showDisconnectDialog = ref(false)

let terminal: Terminal | null = null
let fitAddon: FitAddon | null = null
let ws: WebSocket | null = null
let sessionId: string | null = null
let resizeObserver: ResizeObserver | null = null

function initTerminal() {
  if (!terminalContainer.value) return

  terminal = new Terminal({
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    fontSize: 13,
    theme: {
      background: '#0D1117',
      foreground: '#E6EDF3',
      cursor: '#E8912D',
      cursorAccent: '#0D1117',
    },
    cursorBlink: true,
  })

  fitAddon = new FitAddon()
  terminal.loadAddon(fitAddon)
  terminal.open(terminalContainer.value)

  // Use ResizeObserver instead of window resize for panel-level sizing
  resizeObserver = new ResizeObserver(() => {
    fitAddon?.fit()
  })
  resizeObserver.observe(terminalContainer.value)
  fitAddon.fit()

  terminal.onData((data: string) => {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'terminal.input',
        payload: { data: btoa(data) },
      }))
    }
  })

  terminal.onResize(({ cols, rows }: { cols: number; rows: number }) => {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'terminal.resize',
        payload: { cols, rows },
      }))
    }
  })
}

async function connectSession() {
  connectionStatus.value = 'connecting'

  try {
    const cols = terminal?.cols ?? 80
    const rows = terminal?.rows ?? 24
    const result = await createSession({ resource_id: props.resourceId, cols, rows })
    sessionId = result.session_id

    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
    const token = localStorage.getItem('rex-token') ?? ''
    ws = new WebSocket(`${protocol}//${location.host}/ws/terminal/${sessionId}?token=${token}`)

    ws.onopen = () => {
      connectionStatus.value = 'connected'
      terminal?.focus()
    }

    ws.onmessage = (event: MessageEvent) => {
      try {
        const msg = JSON.parse(event.data)
        switch (msg.type) {
          case 'terminal.output':
            terminal?.write(atob(msg.payload.data))
            break
          case 'terminal.error':
            terminal?.write(`\r\n\x1b[31m错误: ${msg.payload.message}\x1b[0m\r\n`)
            connectionStatus.value = 'disconnected'
            emit('error', msg.payload.message)
            break
          case 'terminal.closed':
            terminal?.write('\r\n\x1b[33m连接已关闭\x1b[0m\r\n')
            connectionStatus.value = 'disconnected'
            emit('disconnect')
            break
        }
      } catch {
        // ignore parse errors
      }
    }

    ws.onclose = () => {
      connectionStatus.value = 'disconnected'
    }

    ws.onerror = () => {
      connectionStatus.value = 'disconnected'
      emit('error', 'WebSocket 连接失败')
    }
  } catch (err: any) {
    connectionStatus.value = 'disconnected'
    const msg = err?.response?.data?.error?.message ?? err?.message ?? '未知错误'
    terminal?.write(`\r\n\x1b[31m会话创建失败: ${msg}\x1b[0m\r\n`)
    emit('error', msg)
  }
}

function clearTerminal() {
  terminal?.clear()
}

async function handlePaste() {
  try {
    const text = await navigator.clipboard.readText()
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'terminal.input',
        payload: { data: btoa(text) },
      }))
    }
  } catch {
    // clipboard access denied
  }
}

async function doDisconnect() {
  showDisconnectDialog.value = false
  ws?.close()
  if (sessionId) {
    try { await deleteSession(sessionId) } catch { /* ignore */ }
  }
  connectionStatus.value = 'disconnected'
  emit('disconnect')
}

onMounted(async () => {
  await nextTick()
  initTerminal()
  await connectSession()
})

onBeforeUnmount(() => {
  ws?.close()
  if (sessionId) {
    deleteSession(sessionId).catch(() => {})
  }
  resizeObserver?.disconnect()
  terminal?.dispose()
})
</script>

<style scoped>
.ws-terminal {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-deep);
  position: relative;
}

.ws-term-toolbar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-sm);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 32px;
  flex-shrink: 0;
}

.ws-term-info {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--fs-xs);
}

.ws-term-status { font-size: 10px; }
.ws-term-status.connected { color: var(--success); }
.ws-term-status.connecting { color: var(--warning); }
.ws-term-status.disconnected { color: var(--danger); }

.ws-term-name {
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-weight: 500;
}

.ws-term-spacer { flex: 1; }

.ws-term-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.ws-term-container {
  flex: 1;
  padding: 4px;
  overflow: hidden;
}

.ws-term-reconnect {
  position: absolute;
  inset: 32px 0 22px 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-sm);
  background: rgba(13, 17, 23, 0.9);
  z-index: 10;
}

.reconnect-icon {
  font-size: 24px;
  opacity: 0.5;
}

.reconnect-text {
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.ws-term-statusbar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-sm);
  background: var(--accent);
  color: #000;
  height: 22px;
  flex-shrink: 0;
  font-size: 11px;
  font-family: var(--font-mono);
  font-weight: 500;
  gap: var(--sp-sm);
}

.ws-term-statusbar .spacer { flex: 1; }

/* Modal */
.ws-term-modal-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20;
}

.ws-term-modal {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  max-width: 320px;
  width: 90%;
}

.ws-term-modal-title {
  font-size: var(--fs-md);
  font-weight: 600;
  margin-bottom: var(--sp-sm);
}

.ws-term-modal-desc {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  margin-bottom: var(--sp-lg);
}

.ws-term-modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
}
</style>
