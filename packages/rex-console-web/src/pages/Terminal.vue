<template>
  <div class="terminal-layout">
    <!-- 工具栏 -->
    <div class="terminal-toolbar">
      <div class="toolbar-info">
        <span class="toolbar-status">●</span>
        <span class="toolbar-label">{{ resourceName }}</span>
      </div>
      <div class="toolbar-spacer"></div>
      <div class="toolbar-actions">
        <button class="btn btn-ghost btn-sm" @click="clearTerminal">清屏</button>
        <button class="btn btn-ghost btn-sm" @click="handlePaste">粘贴</button>
        <button class="btn btn-sm btn-danger" @click="confirmDisconnect">断开</button>
      </div>
    </div>

    <!-- 终端容器 -->
    <div ref="terminalContainer" class="terminal-container"></div>

    <!-- 状态栏 -->
    <div class="terminal-statusbar">
      <span>SSH</span>
      <span>·</span>
      <span>UTF-8</span>
      <span class="spacer"></span>
      <span v-if="connectionStatus === 'connected'" style="color: var(--success)">已连接</span>
      <span v-else-if="connectionStatus === 'connecting'" style="color: var(--warning)">连接中...</span>
      <span v-else style="color: var(--danger)">未连接</span>
    </div>

    <!-- 断开确认弹窗 -->
    <div v-if="showDisconnectDialog" class="modal-overlay" @click.self="showDisconnectDialog = false">
      <div class="modal">
        <div class="modal-title">断开连接？</div>
        <p class="modal-desc">断开后当前会话将终止，未保存的工作可能会丢失。</p>
        <div class="modal-actions">
          <button class="btn" @click="showDisconnectDialog = false">取消</button>
          <button class="btn btn-danger" @click="doDisconnect">断开</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import { createSession, deleteSession } from '@/api/terminal'

const route = useRoute()
const router = useRouter()
const resourceId = route.params.resourceId as string

const terminalContainer = ref<HTMLElement>()
const connectionStatus = ref<'connecting' | 'connected' | 'disconnected'>('disconnected')
const showDisconnectDialog = ref(false)
const resourceName = ref(resourceId)

let terminal: Terminal | null = null
let fitAddon: FitAddon | null = null
let ws: WebSocket | null = null
let sessionId: string | null = null

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

  window.addEventListener('resize', handleResize)
}

function handleResize() {
  fitAddon?.fit()
}

async function connectSession() {
  connectionStatus.value = 'connecting'

  try {
    const cols = terminal?.cols ?? 80
    const rows = terminal?.rows ?? 24
    const result = await createSession({ resource_id: resourceId, cols, rows })
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
            break
          case 'terminal.closed':
            terminal?.write('\r\n\x1b[33m连接已关闭\x1b[0m\r\n')
            connectionStatus.value = 'disconnected'
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
      terminal?.write('\r\n\x1b[31mWebSocket 连接失败\x1b[0m\r\n')
    }
  } catch (err: any) {
    connectionStatus.value = 'disconnected'
    terminal?.write(`\r\n\x1b[31m会话创建失败: ${err?.response?.data?.error?.message ?? err?.message ?? '未知错误'}\x1b[0m\r\n`)
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

function confirmDisconnect() {
  showDisconnectDialog.value = true
}

async function doDisconnect() {
  showDisconnectDialog.value = false
  ws?.close()
  if (sessionId) {
    try { await deleteSession(sessionId) } catch { /* ignore */ }
  }
  router.back()
}

onMounted(async () => {
  initTerminal()
  await connectSession()
})

onBeforeUnmount(() => {
  ws?.close()
  if (sessionId) {
    deleteSession(sessionId).catch(() => {})
  }
  window.removeEventListener('resize', handleResize)
  terminal?.dispose()
})
</script>

<style scoped>
.terminal-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg-deep);
}

.terminal-toolbar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-md);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 38px;
  flex-shrink: 0;
}

.toolbar-info {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  font-size: var(--fs-sm);
}

.toolbar-status {
  color: var(--success);
}

.toolbar-label {
  color: var(--text-primary);
}

.toolbar-spacer {
  flex: 1;
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
}

.terminal-container {
  flex: 1;
  padding: var(--sp-sm);
  overflow: hidden;
}

.terminal-statusbar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-md);
  background: var(--accent);
  color: #000;
  height: 22px;
  flex-shrink: 0;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  font-weight: 500;
  gap: var(--sp-md);
}

.terminal-statusbar .spacer {
  flex: 1;
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: var(--z-modal);
}

.modal {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  max-width: 400px;
  width: 90%;
}

.modal-title {
  font-size: var(--fs-md);
  font-weight: 600;
  margin-bottom: var(--sp-sm);
}

.modal-desc {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  margin-bottom: var(--sp-lg);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
}
</style>
