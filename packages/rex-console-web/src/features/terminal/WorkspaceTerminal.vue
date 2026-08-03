<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { SearchAddon } from '@xterm/addon-search'
import '@xterm/xterm/css/xterm.css'
import { getTerminalTheme } from './terminal-themes'
import TerminalSearch from './TerminalSearch.vue'
import Modal from '@/components/ui/Modal.vue'
import Button from '@/components/ui/Button.vue'
import MobileTerminalBar from './MobileTerminalBar.vue'
import Toast from '@/components/ui/Toast.vue'

const { t } = useI18n()
const toast = ref<InstanceType<typeof Toast>>()

const props = defineProps<{
  tabId: string
  resourceId: string
  host?: string
  port?: number
  protocol?: string
  theme?: string
  fontSize?: number
  opacity?: number
  cursorStyle?: string
  cursorBlink?: boolean
  backgroundImage?: string
}>()

const emit = defineEmits<{
  'terminal-resize': [cols: number, rows: number]
  'toggle-sftp': []
  'encoding-change': [encoding: string]
  'update:status': [status: string]
}>()

// ── Refs ──────────────────────────────────────────────────
const containerRef = ref<HTMLElement>()
const connectionStatus = ref<'connecting' | 'connected' | 'disconnected' | 'error'>('disconnected')
const showDisconnectDialog = ref(false)
const termSize = ref({ cols: 80, rows: 24 })
const terminalEncoding = ref('UTF-8')

// ── Search ────────────────────────────────────────────────
const searchAddon = ref<SearchAddon | null>(null)
const showSearch = ref(false)

// ── Latency ───────────────────────────────────────────────
const latency = ref<number | null>(null)
let pingInterval: ReturnType<typeof setInterval> | null = null
let pingTimestamp = 0

const latencyClass = computed(() => {
  if (latency.value === null) return ''
  if (latency.value < 100) return 'low'
  if (latency.value < 300) return 'medium'
  return 'high'
})

// ── State ─────────────────────────────────────────────────
let terminal: Terminal | null = null
let fitAddon: FitAddon | null = null
let ws: WebSocket | null = null
let resizeObserver: ResizeObserver | null = null
let inputBuffer = ''

// Auto-reconnect
let reconnectAttempts = 0
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
const MAX_RECONNECT_ATTEMPTS = 5
const RECONNECT_DELAYS = [1000, 2000, 4000, 8000, 16000]
let manualDisconnect = false

// ── Context menu state ────────────────────────────────────
const ctxMenuVisible = ref(false)
const ctxMenuX = ref(0)
const ctxMenuY = ref(0)
const ctxMenuItems = ref<Array<{
  label: string
  action: () => void
  danger?: boolean
  disabled?: boolean
  separator?: boolean
}>>([])

function showContextMenu(x: number, y: number, items: typeof ctxMenuItems.value) {
  ctxMenuX.value = x
  ctxMenuY.value = y
  ctxMenuItems.value = items
  ctxMenuVisible.value = true
}

function hideContextMenu() {
  ctxMenuVisible.value = false
}

// ── Load saved encoding ───────────────────────────────────
if (props.resourceId) {
  const saved = localStorage.getItem(`rex-encoding-${props.resourceId}`)
  if (saved) terminalEncoding.value = saved
}

function handleSetEncoding(encoding: string) {
  terminalEncoding.value = encoding
  if (props.resourceId) {
    localStorage.setItem(`rex-encoding-${props.resourceId}`, encoding)
  }
  emit('encoding-change', encoding)
}

// ── Theme ─────────────────────────────────────────────────
function getTerminalSettings() {
  try {
    const raw = localStorage.getItem('rex-terminal-settings')
    return raw ? JSON.parse(raw) : {}
  } catch {
    return {}
  }
}

function getCurrentTheme() {
  const style = getComputedStyle(document.documentElement)
  return {
    background: style.getPropertyValue('--bg-deep').trim(),
    foreground: style.getPropertyValue('--text-primary').trim(),
    cursor: style.getPropertyValue('--accent').trim(),
    cursorAccent: style.getPropertyValue('--bg-deep').trim(),
  }
}

function applyTheme() {
  if (!terminal) return
  const themeName = props.theme || getTerminalSettings().theme
  if (themeName) {
    terminal.options.theme = getTerminalTheme(themeName)
  } else {
    terminal.options.theme = getCurrentTheme()
  }
}

// ── Container style (opacity + background) ────────────────
const BG_PRESETS: Record<string, string> = {
  grid: `linear-gradient(rgba(255,255,255,0.03) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.03) 1px, transparent 1px)`,
  dots: `radial-gradient(circle, rgba(255,255,255,0.05) 1px, transparent 1px)`,
  gradient: `linear-gradient(135deg, #0D1117 0%, #161B22 50%, #0D1117 100%)`,
}

const containerStyle = computed(() => {
  const op = props.opacity ?? 100
  const bg = props.backgroundImage || 'none'
  const style: Record<string, string> = {}
  if (op < 100) style.opacity = String(op / 100)
  if (bg && bg !== 'none') {
    const preset = BG_PRESETS[bg]
    if (preset) {
      style.backgroundImage = preset
      style.backgroundSize = bg === 'grid' ? '20px 20px' : '24px 24px'
    } else if (bg.startsWith('http') || bg.startsWith('data:')) {
      style.backgroundImage = `url(${bg})`
      style.backgroundSize = 'cover'
      style.backgroundPosition = 'center'
    }
  }
  return style
})

// ── Terminal init ──────────────────────────────────────────
function initTerminal() {
  if (!containerRef.value) return

  terminal = new Terminal({
    fontFamily: `'${getTerminalSettings().fontFamily || 'JetBrains Mono'}', 'Fira Code', monospace`,
    fontSize: props.fontSize || getTerminalSettings().fontSize || 14,
    theme: getCurrentTheme(),
    cursorBlink: props.cursorBlink ?? getTerminalSettings().cursorBlink ?? true,
    cursorStyle: (props.cursorStyle as 'block' | 'underline' | 'bar') || 'block',
    scrollback: 10000,
  })

  fitAddon = new FitAddon()
  terminal.loadAddon(fitAddon)

  const search = new SearchAddon()
  terminal.loadAddon(search)
  searchAddon.value = search

  terminal.open(containerRef.value)

  // Key event handler
  terminal.attachCustomKeyEventHandler((event: KeyboardEvent) => {
    if (event.type !== 'keydown') return true
    const ctrl = event.ctrlKey || event.metaKey

    // Ctrl+F → toggle search
    if (ctrl && event.key === 'f') {
      event.preventDefault()
      showSearch.value = !showSearch.value
      return false
    }
    // Ctrl+V → browser paste
    if (ctrl && event.key === 'v') {
      return false
    }
    // Ctrl+C → copy selection or SIGINT
    if (ctrl && !event.shiftKey && (event.key === 'c' || event.key === 'C')) {
      const selection = terminal?.getSelection()
      if (selection) {
        navigator.clipboard?.writeText(selection)
        toast.value?.push(t('terminal.clipboard.copied', 'Copied'), 'success')
        return false
      }
    }
    // Ctrl+Shift+C → force copy
    if (ctrl && event.shiftKey && (event.key === 'c' || event.key === 'C')) {
      const selection = terminal?.getSelection()
      if (selection) {
        navigator.clipboard?.writeText(selection)
        toast.value?.push(t('terminal.clipboard.copied', 'Copied'), 'success')
      }
      return false
    }
    // Ctrl+Shift+F → toggle SFTP
    if (ctrl && event.shiftKey && (event.key === 'f' || event.key === 'F')) {
      emit('toggle-sftp')
      return false
    }
    return true
  })

  // ResizeObserver
  resizeObserver = new ResizeObserver(() => {
    fitAddon?.fit()
  })
  resizeObserver.observe(containerRef.value)
  fitAddon.fit()

  // Terminal input → WebSocket
  terminal.onData((data: string) => {
    // Track command history
    if (data === '\r') {
      const cmd = inputBuffer.trim()
      if (cmd) {
        try {
          const history = JSON.parse(localStorage.getItem('rex-terminal-history') || '[]')
          history.push(cmd)
          if (history.length > 100) history.shift()
          localStorage.setItem('rex-terminal-history', JSON.stringify(history))
        } catch { /* ignore */ }
      }
      inputBuffer = ''
    } else if (data >= ' ' || data === '\x7f') {
      inputBuffer = data === '\x7f' ? inputBuffer.slice(0, -1) : inputBuffer + data
    }

    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'terminal.data',
        data: btoa(data),
      }))
    }
  })

  terminal.onResize(({ cols, rows }: { cols: number; rows: number }) => {
    termSize.value = { cols, rows }
    emit('terminal-resize', cols, rows)
    // Skip resize in alternate screen buffer
    if (terminal && terminal.buffer.active !== terminal.buffer.normal) return
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'terminal.resize',
        cols,
        rows,
      }))
    }
  })
}

// ── Latency ───────────────────────────────────────────────
function startPing() {
  stopPing()
  pingInterval = setInterval(() => {
    if (ws?.readyState === WebSocket.OPEN) {
      pingTimestamp = Date.now()
      ws.send(JSON.stringify({ type: 'ping' }))
    }
  }, 5000)
}

function stopPing() {
  if (pingInterval) {
    clearInterval(pingInterval)
    pingInterval = null
  }
  latency.value = null
}

// ── Connection ────────────────────────────────────────────
function connectSession() {
  connectionStatus.value = 'connecting'
  manualDisconnect = false
  emit('update:status', 'connecting')

  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  const token = localStorage.getItem('rex-token') || ''
  const wsUrl = `${protocol}//${location.host}/ws/terminal?resourceId=${encodeURIComponent(props.resourceId)}&token=${encodeURIComponent(token)}`
  ws = new WebSocket(wsUrl)

  ws.onopen = () => {
    connectionStatus.value = 'connected'
    reconnectAttempts = 0
    emit('update:status', 'online')
    terminal?.focus()
    startPing()
    // Send actual size immediately
    if (terminal && ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'terminal.resize',
        cols: terminal.cols,
        rows: terminal.rows,
      }))
    }
  }

  ws.onmessage = (event: MessageEvent) => {
    try {
      const msg = JSON.parse(event.data as string)
      switch (msg.type) {
        case 'terminal.connected':
          // Session established
          break
        case 'terminal.data': {
          const data = atob(msg.payload.data)
          terminal?.write(data)
          break
        }
        case 'terminal.disconnected':
          terminal?.writeln(`\r\n\x1b[33m[Session disconnected: ${msg.payload.reason}]\x1b[0m`)
          break
        case 'terminal.error':
          terminal?.write(`\r\n\x1b[31m[Error: ${msg.payload.message}]\x1b[0m`)
          connectionStatus.value = 'error'
          emit('update:status', 'error')
          break
        case 'pong':
          if (pingTimestamp) {
            latency.value = Date.now() - pingTimestamp
          }
          break
      }
    } catch {
      // ignore non-JSON
    }
  }

  ws.onclose = () => {
    stopPing()
    connectionStatus.value = 'disconnected'
    emit('update:status', 'offline')

    // Auto-reconnect
    if (!manualDisconnect && reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
      const delay = RECONNECT_DELAYS[Math.min(reconnectAttempts, RECONNECT_DELAYS.length - 1)]
      reconnectAttempts++
      terminal?.write(`\r\n\x1b[33m[Reconnecting... ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS}]\x1b[0m\r\n`)
      reconnectTimer = setTimeout(() => {
        connectSession()
      }, delay)
    } else if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
      terminal?.write(`\r\n\x1b[31m[Connection lost. Click reconnect to try again.]\x1b[0m\r\n`)
    }
  }

  ws.onerror = () => {
    stopPing()
  }
}

// ── Disconnect ────────────────────────────────────────────
function doDisconnect() {
  showDisconnectDialog.value = false
  manualDisconnect = true
  stopPing()
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: 'terminal.disconnect' }))
  }
  ws?.close()
  ws = null
  connectionStatus.value = 'disconnected'
  emit('update:status', 'offline')
}

function handleReconnect() {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  reconnectAttempts = 0
  ws?.close()
  ws = null
  connectSession()
}

// ── Copy / Paste ──────────────────────────────────────────
function handleCopy() {
  const selection = terminal?.getSelection()
  if (selection) {
    navigator.clipboard?.writeText(selection)
    toast.value?.push(t('terminal.clipboard.copied', 'Copied'), 'success')
  }
}

async function handlePaste() {
  try {
    const text = await navigator.clipboard.readText()
    if (text && ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'terminal.data',
        data: btoa(text),
      }))
    }
  } catch {
    // clipboard access denied
  }
}

function clearTerminal() {
  terminal?.clear()
}

function handleCopyAddress() {
  const address = props.host ? `${props.host}:${props.port || 22}` : ''
  if (address) {
    navigator.clipboard.writeText(address)
    toast.value?.push(t('terminal.copyAddress', 'Address copied'), 'success')
  }
}

// ── Context menu ──────────────────────────────────────────
function handleContextMenu(event: MouseEvent) {
  if (!terminal) return
  const selection = terminal.getSelection()
  showContextMenu(event.clientX, event.clientY, [
    { label: t('terminal.ctx.copy', 'Copy'), action: handleCopy, disabled: !selection },
    { label: t('terminal.ctx.paste', 'Paste'), action: handlePaste },
    { label: t('terminal.ctx.selectAll', 'Select All'), action: () => terminal?.selectAll() },
    { separator: true, label: '', action: () => {} },
    { label: t('terminal.ctx.clear', 'Clear'), action: clearTerminal },
    { label: t('terminal.ctx.find', 'Find'), action: () => { showSearch.value = true } },
    { separator: true, label: '', action: () => {} },
    { label: t('terminal.ctx.reconnect', 'Reconnect'), action: handleReconnect },
    { label: t('terminal.ctx.copyAddress', 'Copy Address'), action: handleCopyAddress },
    { label: t('terminal.ctx.openSftp', 'Open SFTP'), action: () => emit('toggle-sftp') },
    { separator: true, label: '', action: () => {} },
    { label: t('terminal.ctx.disconnect', 'Disconnect'), action: () => { showDisconnectDialog.value = true }, danger: true },
  ])
}

function handleToolbarContextMenu(event: MouseEvent) {
  showContextMenu(event.clientX, event.clientY, [
    { label: t('terminal.ctx.copyAddress', 'Copy Address'), action: handleCopyAddress },
    { label: t('terminal.ctx.openSftp', 'Open SFTP'), action: () => emit('toggle-sftp') },
    { separator: true, label: '', action: () => {} },
    { label: t('terminal.ctx.reconnect', 'Reconnect'), action: handleReconnect },
    { label: t('terminal.ctx.disconnect', 'Disconnect'), action: () => { showDisconnectDialog.value = true }, danger: true },
  ])
}

// ── Status dot ────────────────────────────────────────────
const statusDotClass = computed(() => {
  switch (connectionStatus.value) {
    case 'connected': return 'online'
    case 'connecting': return 'connecting'
    case 'error': return 'error'
    default: return 'offline'
  }
})

// ── Watch settings ────────────────────────────────────────
watch(() => props.theme, () => { applyTheme() })
watch(() => props.fontSize, (val) => { if (terminal && val) terminal.options.fontSize = val })
watch(() => props.cursorBlink, (val) => { if (terminal) terminal.options.cursorBlink = val ?? true })
watch(() => props.cursorStyle, (val) => { if (terminal && val) terminal.options.cursorStyle = val as any })

function onTerminalSettingsChanged(e: Event) {
  const detail = (e as CustomEvent).detail
  if (detail && terminal) {
    if (detail.theme) terminal.options.theme = getTerminalTheme(detail.theme)
    if (detail.fontFamily) terminal.options.fontFamily = `'${detail.fontFamily}', 'Fira Code', monospace`
    if (detail.fontSize) terminal.options.fontSize = detail.fontSize
  }
}

// ── Lifecycle ─────────────────────────────────────────────
onMounted(async () => {
  await nextTick()
  initTerminal()
  connectSession()

  window.addEventListener('terminal-settings-changed', onTerminalSettingsChanged)
  document.addEventListener('click', hideContextMenu)
})

onBeforeUnmount(() => {
  inputBuffer = ''
  stopPing()
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  ws?.close()
  ws = null
  resizeObserver?.disconnect()
  terminal?.dispose()
  terminal = null
  fitAddon = null
  window.removeEventListener('terminal-settings-changed', onTerminalSettingsChanged)
  document.removeEventListener('click', hideContextMenu)
})
</script>

<template>
  <div class="workspace-terminal">
    <!-- Toolbar -->
    <div class="wt-toolbar" @contextmenu.prevent="handleToolbarContextMenu">
      <div class="wt-info">
        <span class="wt-status-dot" :class="statusDotClass">●</span>
        <span class="wt-resource-name">{{ host || resourceId }}</span>
        <span v-if="latency !== null" class="wt-latency" :class="latencyClass">{{ latency }}ms</span>
      </div>
      <div class="wt-spacer"></div>
      <div class="wt-actions">
        <button class="wt-btn" @click="clearTerminal" :title="t('terminal.clear', 'Clear')">⌫</button>
        <button class="wt-btn" @click="handlePaste" :title="t('terminal.paste', 'Paste')">📋</button>
        <button class="wt-btn" :class="{ active: showSearch }" @click="showSearch = !showSearch" :title="t('terminal.find', 'Find')">🔍</button>
        <span class="wt-sep"></span>
        <span class="wt-protocol">{{ protocol?.toUpperCase() || 'SSH' }}</span>
        <span class="wt-encoding">{{ terminalEncoding }}</span>
        <span v-if="termSize" class="wt-size">{{ termSize.cols }}×{{ termSize.rows }}</span>
      </div>
    </div>

    <!-- Terminal container -->
    <div
      ref="containerRef"
      class="wt-container"
      :style="containerStyle"
      @contextmenu.prevent="handleContextMenu"
    >
      <TerminalSearch
        :visible="showSearch"
        :search-addon="searchAddon"
        @close="showSearch = false"
      />

      <!-- Disconnected overlay -->
      <div v-if="connectionStatus === 'disconnected' || connectionStatus === 'error'" class="wt-overlay">
        <div class="wt-overlay-content">
          <div class="wt-overlay-icon">⚡</div>
          <p class="wt-overlay-text">
            {{ connectionStatus === 'error' ? t('terminal.connectionError', 'Connection error') : t('terminal.sessionDisconnected', 'Session disconnected') }}
          </p>
          <button class="wt-reconnect-btn" @click="handleReconnect">
            {{ t('terminal.reconnect', 'Reconnect') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Status bar -->
    <div class="wt-statusbar">
      <span>SSH</span>
      <span>·</span>
      <span>{{ terminalEncoding }}</span>
      <span>·</span>
      <span>{{ termSize.cols }}×{{ termSize.rows }}</span>
      <span v-if="latency !== null" class="wt-latency-bar" :class="latencyClass">{{ latency }}ms</span>
      <span class="wt-statusbar-spacer"></span>
      <span v-if="connectionStatus === 'connected'" style="color: #000">{{ t('terminal.statusbar.connected', 'Connected') }}</span>
      <span v-else-if="connectionStatus === 'connecting'" style="color: #000">{{ t('terminal.statusbar.connecting', 'Connecting') }}</span>
      <span v-else style="color: #fff; background: rgba(0,0,0,0.3); padding: 0 4px; border-radius: 3px">{{ t('terminal.statusbar.disconnected', 'Disconnected') }}</span>
    </div>

    <!-- Mobile toolbar -->
    <MobileTerminalBar :terminal="terminal" />

    <!-- Context menu -->
    <div
      v-if="ctxMenuVisible"
      class="wt-ctx-menu"
      :style="{ left: ctxMenuX + 'px', top: ctxMenuY + 'px' }"
      @click.stop
    >
      <template v-for="(item, idx) in ctxMenuItems" :key="idx">
        <div v-if="item.separator" class="wt-ctx-sep"></div>
        <button
          v-else
          class="wt-ctx-item"
          :class="{ danger: item.danger, disabled: item.disabled }"
          :disabled="item.disabled"
          @click="item.action(); hideContextMenu()"
        >
          {{ item.label }}
        </button>
      </template>
    </div>

    <!-- Disconnect dialog -->
    <Modal :model-value="showDisconnectDialog" @update:model-value="showDisconnectDialog = $event">
      <template #title>{{ t('terminal.disconnect.title', 'Disconnect') }}</template>
      <p style="color: var(--text-secondary); margin-bottom: 16px">
        {{ t('terminal.disconnect.desc', 'Are you sure you want to disconnect?') }}
      </p>
      <div class="form-actions">
        <Button variant="secondary" @click="showDisconnectDialog = false">{{ t('common.cancel', 'Cancel') }}</Button>
        <Button variant="danger" @click="doDisconnect">{{ t('common.confirm', 'Confirm') }}</Button>
      </div>
    </Modal>

    <Toast ref="toast" />
  </div>
</template>

<style scoped>
.workspace-terminal {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-deep, #0d1117);
  position: relative;
  overflow: hidden;
}

/* ── Toolbar ── */
.wt-toolbar {
  display: flex;
  align-items: center;
  padding: 0 8px;
  background: var(--bg-surface, #1c2128);
  border-bottom: 1px solid var(--border, #30363d);
  height: 32px;
  flex-shrink: 0;
}

.wt-info {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
}

.wt-status-dot { font-size: 10px; }
.wt-status-dot.online { color: var(--success, #3fb950); }
.wt-status-dot.connecting { color: var(--warning, #d29922); }
.wt-status-dot.error { color: var(--danger, #f85149); }
.wt-status-dot.offline { color: var(--text-muted, #8b949e); }

.wt-resource-name {
  color: var(--text-primary, #e6edf3);
  font-family: var(--font-mono, monospace);
  font-weight: 500;
}

.wt-latency {
  font-family: var(--font-mono, monospace);
  font-size: 10px;
  font-weight: 500;
  padding: 0 4px;
  border-radius: 3px;
  background: rgba(255, 255, 255, 0.06);
}
.wt-latency.low { color: var(--success, #3fb950); }
.wt-latency.medium { color: var(--warning, #d29922); }
.wt-latency.high { color: var(--danger, #f85149); }

.wt-spacer { flex: 1; }

.wt-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
}

.wt-btn {
  background: none;
  border: none;
  color: var(--text-muted, #8b949e);
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
  transition: all 0.15s;
}
.wt-btn:hover {
  background: var(--bg-hover, #21262d);
  color: var(--text-primary, #e6edf3);
}
.wt-btn.active {
  background: var(--accent-muted, rgba(232, 145, 45, 0.15));
  color: var(--accent, #e8912d);
}

.wt-sep {
  width: 1px;
  height: 14px;
  background: var(--border, #30363d);
  margin: 0 4px;
}

.wt-protocol, .wt-encoding, .wt-size {
  font-family: var(--font-mono, monospace);
  font-size: 10px;
  color: var(--text-muted, #8b949e);
  padding: 0 4px;
}

/* ── Terminal container ── */
.wt-container {
  flex: 1;
  min-height: 0;
  background: #0d1117;
  overflow: hidden;
  position: relative;
}

.wt-container :deep(.xterm) {
  padding: 0;
}

/* ── Overlay ── */
.wt-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(2px);
  z-index: 10;
}

.wt-overlay-content {
  text-align: center;
}

.wt-overlay-icon {
  font-size: 24px;
  opacity: 0.5;
  margin-bottom: 8px;
}

.wt-overlay-text {
  font-size: 13px;
  color: var(--text-muted, #8b949e);
  margin-bottom: 12px;
}

.wt-reconnect-btn {
  padding: 6px 16px;
  background: var(--accent, #e8912d);
  color: #fff;
  border: none;
  border-radius: 6px;
  font-family: var(--font-mono, monospace);
  font-size: 13px;
  cursor: pointer;
  transition: opacity 0.15s;
}
.wt-reconnect-btn:hover { opacity: 0.9; }

/* ── Status bar ── */
.wt-statusbar {
  display: flex;
  align-items: center;
  padding: 0 8px;
  background: var(--accent, #e8912d);
  color: #000;
  height: 22px;
  flex-shrink: 0;
  font-size: 11px;
  font-family: var(--font-mono, monospace);
  font-weight: 500;
  gap: 8px;
}

.wt-statusbar-spacer { flex: 1; }

.wt-latency-bar {
  font-weight: 700;
}
.wt-latency-bar.low { color: #000; }
.wt-latency-bar.medium { color: #000; }
.wt-latency-bar.high { color: #fff; background: rgba(0,0,0,0.3); padding: 0 4px; border-radius: 3px; }

/* ── Context menu ── */
.wt-ctx-menu {
  position: fixed;
  z-index: 1000;
  background: var(--bg-surface, #1c2128);
  border: 1px solid var(--border, #30363d);
  border-radius: 6px;
  padding: 4px 0;
  min-width: 160px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}

.wt-ctx-item {
  display: block;
  width: 100%;
  padding: 6px 12px;
  background: none;
  border: none;
  color: var(--text-primary, #e6edf3);
  font-size: 12px;
  text-align: left;
  cursor: pointer;
  transition: background 0.1s;
}
.wt-ctx-item:hover {
  background: var(--bg-hover, #21262d);
}
.wt-ctx-item.danger {
  color: var(--danger, #f85149);
}
.wt-ctx-item.disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.wt-ctx-sep {
  height: 1px;
  background: var(--border, #30363d);
  margin: 4px 0;
}
</style>
