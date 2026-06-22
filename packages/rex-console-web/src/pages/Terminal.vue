<template>
  <div class="terminal-layout" @contextmenu.prevent="showContextMenu">
    <!-- 工具栏 -->
    <div class="terminal-toolbar">
      <div class="toolbar-info">
        <span class="toolbar-status">●</span>
        <span class="toolbar-label">{{ resourceName }}</span>
      </div>
      <div class="toolbar-spacer"></div>
      <div class="toolbar-actions">
        <button class="btn btn-ghost btn-sm" @click="clearTerminal">清屏</button>
        <button class="btn btn-ghost btn-sm" @click="handleCopy">复制</button>
        <button class="btn btn-ghost btn-sm" @click="handlePaste">粘贴</button>
        <div class="toolbar-sep"></div>
        <button
          class="toolbar-btn-sftp"
          :class="{ active: sftpVisible }"
          @click="toggleSftp"
          title="SFTP 面板 (Ctrl+Shift+F)"
        >📁 SFTP</button>
        <button class="btn btn-sm btn-ghost" @click="toggleFullscreen" :title="isFullscreen ? '退出全屏 (F11)' : '全屏 (F11)'">⛶</button>
        <button class="btn btn-sm btn-danger" @click="confirmDisconnect">断开</button>
      </div>
    </div>

    <!-- 终端 + SFTP Split -->
    <div class="terminal-sftp-wrap">
      <!-- 终端容器 -->
      <div ref="terminalContainer" class="terminal-container" @contextmenu.prevent="showContextMenu"></div>

      <!-- SFTP 面板 -->
      <TerminalSftp
        v-if="sftpVisible"
        :resource-id="resourceId"
        @close="sftpVisible = false"
        @drag-path="handleSftpDragPath"
      />
    </div>

    <!-- 状态栏 -->
    <div class="terminal-statusbar">
      <span>SSH</span>
      <span>·</span>
      <span>UTF-8</span>
      <span class="spacer"></span>
      <span style="font-size: 11px; opacity: 0.7">Ctrl+Shift+C 复制 · Ctrl+Shift+V 粘贴</span>
      <span class="spacer"></span>
      <span v-if="connectionStatus === 'connected'" style="color: var(--success)">已连接</span>
      <span v-else-if="connectionStatus === 'connecting'" style="color: var(--warning)">连接中...</span>
      <span v-else style="color: var(--danger)">未连接</span>
    </div>

    <!-- 右键菜单 -->
    <div
      v-if="contextMenu.visible"
      class="ctx-menu"
      :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
    >
      <button class="ctx-menu-item" @click="handleCopy">复制</button>
      <button class="ctx-menu-item" @click="handlePaste">粘贴</button>
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
import { ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import { createSession, deleteSession } from '@/api/terminal'
import { terminalSettings } from '@/stores/settings'
import TerminalSftp from '@/features/terminal/TerminalSftp.vue'

const route = useRoute()
const router = useRouter()
const resourceId = route.params.resourceId as string

const terminalContainer = ref<HTMLElement>()
const connectionStatus = ref<'connecting' | 'connected' | 'disconnected'>('disconnected')
const showDisconnectDialog = ref(false)
const resourceName = ref(resourceId)
const sftpVisible = ref(false)
const isFullscreen = ref(false)
const contextMenu = ref({ visible: false, x: 0, y: 0 })

let terminal: Terminal | null = null
let fitAddon: FitAddon | null = null
let ws: WebSocket | null = null
let sessionId: string | null = null

function initTerminal() {
  if (!terminalContainer.value) return

  terminal = new Terminal({
    fontFamily: `'${terminalSettings.fontFamily}', 'Fira Code', monospace`,
    fontSize: terminalSettings.fontSize,
    theme: {
      background: '#0D1117',
      foreground: '#E6EDF3',
      cursor: '#E8912D',
      cursorAccent: '#0D1117',
    },
    cursorBlink: terminalSettings.cursorBlink,
  })

  fitAddon = new FitAddon()
  terminal.loadAddon(fitAddon)
  terminal.open(terminalContainer.value)
  fitAddon.fit()

  // 拦截 Ctrl+V：阻止 xterm.js 内部调用 navigator.clipboard.readText()（需要 HTTPS）
  // 返回 false 让 xterm 不处理该按键，浏览器原生粘贴行为会在 textarea 上触发 DOM paste 事件
  terminal.attachCustomKeyEventHandler((event: KeyboardEvent) => {
    // Ctrl+V / Cmd+V → 不让 xterm 处理，交给浏览器原生 paste
    if ((event.ctrlKey || event.metaKey) && event.key === 'v' && event.type === 'keydown') {
      return false
    }
    // Ctrl+Shift+C → 由自定义 handleKeydown 处理
    if ((event.ctrlKey || event.metaKey) && event.shiftKey && (event.key === 'C' || event.key === 'c') && event.type === 'keydown') {
      return false
    }
    return true
  })

  // 在 xterm 内部 textarea 上监听 paste 事件
  // 当 xterm 不处理 Ctrl+V 时，浏览器会触发原生 paste 事件
  terminal.textarea?.addEventListener('paste', (e: ClipboardEvent) => {
    const text = e.clipboardData?.getData('text')
    if (text && ws?.readyState === WebSocket.OPEN) {
      e.preventDefault()
      ws.send(JSON.stringify({
        type: 'terminal.input',
        payload: { data: btoa(text) },
      }))
    }
  }, true)

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

function handleCopy() {
  if (!terminal) return
  const text = terminal.getSelection()
  if (text) {
    navigator.clipboard.writeText(text).catch(() => {})
  }
}

async function handlePaste() {
  try {
    // 优先尝试 Clipboard API（需要 HTTPS 或 localhost）
    const text = await navigator.clipboard.readText()
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'terminal.input',
        payload: { data: btoa(text) },
      }))
    }
  } catch {
    // HTTP 环境下 Clipboard API 不可用，提示用户使用 Ctrl+V 粘贴
    terminal?.write('\r\n\x1b[33m提示: 请使用 Ctrl+V 粘贴内容\x1b[0m')
  }
}

function toggleSftp() {
  sftpVisible.value = !sftpVisible.value
}

function handleSftpDragPath(path: string) {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({
      type: 'terminal.input',
      payload: { data: btoa(path) },
    }))
    terminal?.focus()
  }
}

function toggleFullscreen() {
  if (document.fullscreenElement) {
    document.exitFullscreen()
    isFullscreen.value = false
  } else {
    document.documentElement.requestFullscreen()
    isFullscreen.value = true
  }
}

function showContextMenu(e: MouseEvent) {
  contextMenu.value = { visible: true, x: e.clientX, y: e.clientY }

  const close = () => {
    contextMenu.value.visible = false
    document.removeEventListener('click', close)
  }
  // 延迟绑定，避免当前点击立即关闭
  setTimeout(() => document.addEventListener('click', close), 0)
}

function handleKeydown(e: KeyboardEvent) {
  // Ctrl+Shift+C → 复制
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'C') {
    e.preventDefault()
    handleCopy()
    return
  }
  // Ctrl+Shift+V → 粘贴
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'V') {
    e.preventDefault()
    handlePaste()
    return
  }
  // Ctrl+Shift+F → 切换 SFTP 面板
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'F' || e.key === 'f')) {
    e.preventDefault()
    toggleSftp()
    return
  }
  // F11 → 切换全屏
  if (e.key === 'F11') {
    e.preventDefault()
    toggleFullscreen()
    return
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

function handleFullscreenChange() {
  isFullscreen.value = !!document.fullscreenElement
}

// ── Watch terminal settings ──
watch(() => terminalSettings.fontSize, (val) => {
  if (terminal) terminal.options.fontSize = val
})
watch(() => terminalSettings.fontFamily, (val) => {
  if (terminal) terminal.options.fontFamily = `'${val}', 'Fira Code', monospace`
})
watch(() => terminalSettings.cursorBlink, (val) => {
  if (terminal) terminal.options.cursorBlink = val
})

onMounted(async () => {
  initTerminal()
  await connectSession()
  window.addEventListener('keydown', handleKeydown)
  document.addEventListener('fullscreenchange', handleFullscreenChange)
})

onBeforeUnmount(() => {
  ws?.close()
  if (sessionId) {
    deleteSession(sessionId).catch(() => {})
  }
  window.removeEventListener('resize', handleResize)
  window.removeEventListener('keydown', handleKeydown)
  document.removeEventListener('fullscreenchange', handleFullscreenChange)
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

.toolbar-sep {
  width: 1px;
  height: 20px;
  background: var(--border);
  margin: 0 var(--sp-xs);
}

.toolbar-btn-sftp {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-sm);
  height: 30px;
  padding: 0 var(--sp-md);
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-secondary);
  font-size: var(--fs-xs);
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}

.toolbar-btn-sftp:hover {
  background: var(--bg-hover);
  color: var(--accent-purple);
}

.toolbar-btn-sftp.active {
  background: rgba(139, 92, 246, 0.12);
  color: var(--accent-purple);
  border-color: var(--accent-purple);
}

.terminal-sftp-wrap {
  display: flex;
  flex: 1;
  overflow: hidden;
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

.ctx-menu {
  position: fixed;
  z-index: 1000;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  padding: 4px 0;
  min-width: 120px;
}

.ctx-menu-item {
  display: block;
  width: 100%;
  padding: 6px 12px;
  font-size: var(--fs-sm);
  color: var(--text-primary);
  background: none;
  border: none;
  text-align: left;
  cursor: pointer;
  white-space: nowrap;
}

.ctx-menu-item:hover {
  background: var(--bg-hover);
}
</style>
