<template>
  <div class="ws-terminal" @drop.prevent="handleDrop" @dragover.prevent="handleDragOver" @dragleave="dragOver = false">
    <!-- 工具栏 -->
    <div class="ws-term-toolbar" @contextmenu.prevent="handleToolbarContextMenu">
      <div class="ws-term-info">
        <span class="ws-term-status" :class="connectionStatus">●</span>
        <span class="ws-term-name">{{ resourceName }}</span>
      </div>
      <div class="ws-term-spacer"></div>
      <div class="ws-term-actions">
        <button class="btn btn-ghost btn-xs" @click="clearTerminal">清屏</button>
        <button class="btn btn-ghost btn-xs" @click="handlePaste">粘贴</button>
        <button class="btn btn-ghost btn-xs" :class="{ active: showSftp }" @click="toggleSftp">📁 SFTP</button>
        <button class="btn btn-xs btn-danger" @click="showDisconnectDialog = true">断开</button>
      </div>
    </div>

    <!-- 主体区域 -->
    <div class="ws-term-body">
      <!-- 终端区域 -->
      <div class="ws-term-main" :style="showSftp ? { flex: 1 } : { flex: '1 1 100%' }">
        <div ref="terminalContainer" class="ws-term-container" @contextmenu.prevent="handleContextMenu"></div>

        <!-- 拖拽高亮 -->
        <div v-if="dragOver" class="ws-term-dropzone">
          <span>释放以粘贴路径到终端</span>
        </div>

        <!-- 未连接时显示重连提示 -->
        <div v-if="connectionStatus === 'disconnected'" class="ws-term-reconnect">
          <div class="reconnect-icon">⚡</div>
          <div class="reconnect-text">连接已断开</div>
          <button class="btn btn-sm btn-primary" @click="connectSession">重新连接</button>
        </div>
      </div>

      <!-- 拖拽分隔条 -->
      <div v-if="showSftp" class="ws-term-divider" @mousedown="startResize"></div>

      <!-- SFTP 面板 -->
      <div v-if="showSftp" class="ws-term-sftp" :style="{ width: sftpWidth + 'px' }">
        <div class="sftp-header">
          <span class="sftp-path">{{ sftpPath }}</span>
          <button class="btn btn-ghost btn-xs" @click="showSftp = false">✕</button>
        </div>
        <FileList
          :entries="sftpEntries"
          :current-path="sftpPath"
          :selected-paths="[]"
          :loading="sftpLoading"
          @go-up="sftpGoUp"
          @open="sftpOpenDir"
          @context-menu="() => {}"
        />
      </div>
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
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import '@xterm/xterm/css/xterm.css'
import { createSession, deleteSession } from '@/api/terminal'
import { useContextMenu } from '@/composables/useContextMenu'
import { listFiles } from '@/api/files'
import type { FileEntry } from '@/api/files'
import FileList from '@/features/files/FileList.vue'

const { t } = useI18n()
const router = useRouter()
const { show: showMenu } = useContextMenu()

const props = defineProps<{
  resourceId: string
  resourceName: string
}>()

const emit = defineEmits<{
  (e: 'disconnect'): void
  (e: 'error', msg: string): void
  (e: 'newConnection'): void
}>()

const terminalContainer = ref<HTMLElement>()
const connectionStatus = ref<'connecting' | 'connected' | 'disconnected'>('disconnected')
const showDisconnectDialog = ref(false)

// SFTP panel state
const showSftp = ref(false)
const sftpWidth = ref(280)
const sftpPath = ref('/')
const sftpEntries = ref<FileEntry[]>([])
const sftpLoading = ref(false)

// Drag to terminal
const dragOver = ref(false)

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

// ── SFTP 面板 ──────────────────────────────────────

function toggleSftp() {
  showSftp.value = !showSftp.value
  if (showSftp.value && sftpEntries.value.length === 0) {
    loadSftpFiles('/')
  }
  // Wait for layout to settle, then refit terminal
  nextTick(() => { fitAddon?.fit() })
}

async function loadSftpFiles(path: string) {
  sftpLoading.value = true
  try {
    const result = await listFiles(props.resourceId, path)
    sftpPath.value = result.path
    sftpEntries.value = result.entries
  } catch {
    sftpEntries.value = []
  } finally {
    sftpLoading.value = false
  }
}

function sftpGoUp() {
  const parts = sftpPath.value.split('/').filter(Boolean)
  parts.pop()
  loadSftpFiles('/' + parts.join('/') || '/')
}

function sftpOpenDir(name: string) {
  const base = sftpPath.value.endsWith('/') ? sftpPath.value : sftpPath.value + '/'
  loadSftpFiles(base + name)
}

// ── 拖拽分隔条 ──────────────────────────────────────

let resizeStartX = 0
let resizeStartWidth = 0

function startResize(e: MouseEvent) {
  resizeStartX = e.clientX
  resizeStartWidth = sftpWidth.value
  document.addEventListener('mousemove', onResize)
  document.addEventListener('mouseup', stopResize)
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
}

function onResize(e: MouseEvent) {
  const delta = resizeStartX - e.clientX
  sftpWidth.value = Math.max(200, Math.min(500, resizeStartWidth + delta))
}

function stopResize() {
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  fitAddon?.fit()
}

// ── 拖拽文件到终端 ──────────────────────────────────────

function handleDragOver(e: DragEvent) {
  e.preventDefault()
  dragOver.value = true
}

function handleDrop(e: DragEvent) {
  dragOver.value = false
  const path = e.dataTransfer?.getData('text/plain')
  if (path && ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({
      type: 'terminal.input',
      payload: { data: btoa(path) },
    }))
    terminal?.focus()
  }
}

// ── 工具栏右键菜单 ──────────────────────────────────────

function handleToolbarContextMenu(event: MouseEvent) {
  showMenu(event, [
    {
      label: t('ws.terminal.toolbar.ctx.copyLatency'),
      action: () => { navigator.clipboard.writeText(`${props.resourceName} · ${connectionStatus.value}`) },
    },
    {
      label: t('ws.terminal.toolbar.ctx.openConnectionDetail'),
      action: () => { router.push({ name: 'resource', params: { id: props.resourceId } }) },
    },
    { separator: true },
    {
      label: t('ws.terminal.toolbar.ctx.toggleFullscreen'),
      action: () => {
        if (document.fullscreenElement) document.exitFullscreen()
        else document.documentElement.requestFullscreen()
      },
    },
  ])
}

// ── 右键菜单 ──────────────────────────────────────

function handleContextMenu(event: MouseEvent) {
  if (!terminal) return
  const selection = terminal.getSelection()
  showMenu(event, [
    {
      label: t('ws.terminal.ctx.copy'),
      action: () => { navigator.clipboard.writeText(selection) },
      disabled: !selection,
    },
    {
      label: t('ws.terminal.ctx.paste'),
      action: () => { handlePaste() },
    },
    {
      label: t('ws.terminal.ctx.selectAll'),
      action: () => { terminal?.selectAll() },
    },
    { separator: true },
    {
      label: t('ws.terminal.ctx.clear'),
      action: () => { clearTerminal() },
    },
    {
      label: t('ws.terminal.ctx.reconnect'),
      action: async () => {
        await doDisconnect()
        await connectSession()
      },
    },
    { separator: true },
    {
      label: t('ws.terminal.ctx.openSftp'),
      action: () => { toggleSftp() },
    },
    {
      label: t('ws.terminal.ctx.newConnection'),
      action: () => { emit('newConnection') },
    },
    {
      label: t('ws.terminal.ctx.copyAddress'),
      action: () => { navigator.clipboard.writeText(props.resourceName) },
    },
    { separator: true },
    {
      label: t('ws.terminal.ctx.disconnect'),
      danger: true,
      action: () => { showDisconnectDialog.value = true },
    },
  ])
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
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
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

.ws-term-actions .active {
  background: var(--accent-muted);
  color: var(--accent);
}

/* ── 主体区域（split view） ── */
.ws-term-body {
  flex: 1;
  display: flex;
  overflow: hidden;
  min-height: 0;
}

.ws-term-main {
  position: relative;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.ws-term-container {
  flex: 1;
  padding: 4px;
  overflow: hidden;
}

/* 拖拽高亮 */
.ws-term-dropzone {
  position: absolute;
  inset: 4px;
  border: 2px dashed var(--accent);
  border-radius: var(--radius-md);
  background: rgba(37, 99, 235, 0.08);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--accent);
  font-size: var(--fs-sm);
  z-index: 5;
  pointer-events: none;
}

/* ── 拖拽分隔条 ── */
.ws-term-divider {
  width: 4px;
  cursor: col-resize;
  background: var(--border);
  flex-shrink: 0;
  transition: background 0.15s;
}

.ws-term-divider:hover {
  background: var(--accent);
}

/* ── SFTP 面板 ── */
.ws-term-sftp {
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--border);
  background: var(--bg-surface);
  overflow: hidden;
  flex-shrink: 0;
}

.sftp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--sp-sm);
  height: 28px;
  background: var(--bg-elevated);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.sftp-path {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

/* ── 重连提示 ── */
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

/* ── 状态栏 ── */
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

/* ── Modal ── */
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
