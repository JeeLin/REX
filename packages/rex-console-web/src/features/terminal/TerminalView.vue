<script setup lang="ts">
import { ref, computed, shallowRef, onMounted, onBeforeUnmount, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTerminal } from './useTerminal'
import { getTerminalTheme } from './terminal-themes'
import { SearchAddon } from '@xterm/addon-search'
import TerminalSearch from './TerminalSearch.vue'
import TerminalContextMenu from './TerminalContextMenu.vue'
import MobileTerminalBar from './MobileTerminalBar.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import Toast from '@/components/ui/Toast.vue'

const { t } = useI18n()
const toast = ref<InstanceType<typeof Toast>>()

const props = defineProps<{
  tabId: string
  resourceId: string
  /** 以下字段仅用于状态栏显示，不用于连接 */
  host?: string
  port?: number
  protocol?: string
  /** 终端主题设置（来自 ResourceProperties） */
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

const containerRef = ref<HTMLDivElement>()
const { terminal, status, errorMessage, createTerminal, connect, disconnect, fit, dispose } =
  useTerminal()

// Sync terminal status to parent (WorkspacePage tab status)
watch(status, (s) => {
  const dotStatus = s === 'connected' ? 'online' : s === 'connecting' ? 'connecting' : s === 'error' ? 'error' : 'offline'
  emit('update:status', dotStatus)
})

// Search
const searchAddon = shallowRef<SearchAddon | null>(null)
const showSearch = ref(false)

// Context menu
const showContextMenu = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)

// Encoding state (per resource, persisted in localStorage)
const terminalEncoding = ref('UTF-8')
function handleSetEncoding(encoding: string) {
  terminalEncoding.value = encoding
  if (props.resourceId) {
    localStorage.setItem(`rex-encoding-${props.resourceId}`, encoding)
  }
  emit('encoding-change', encoding)
}
// Load saved encoding on mount
if (props.resourceId) {
  const saved = localStorage.getItem(`rex-encoding-${props.resourceId}`)
  if (saved) terminalEncoding.value = saved
}

const statusDot = ref<StatusDotStatus>('offline')

// Background image CSS presets
const BG_PRESETS: Record<string, string> = {
  grid: `linear-gradient(rgba(255,255,255,0.03) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.03) 1px, transparent 1px)`,
  dots: `radial-gradient(circle, rgba(255,255,255,0.05) 1px, transparent 1px)`,
  gradient: `linear-gradient(135deg, #0D1117 0%, #161B22 50%, #0D1117 100%)`,
}

// Apply opacity to terminal background
const containerStyle = computed(() => {
  // Depend on globalSettingsVersion to re-evaluate when settings change
  void globalSettingsVersion.value
  const globalSettings = getGlobalTerminalSettings()
  const op = props.opacity ?? globalSettings?.opacity ?? 100
  const bg = props.backgroundImage || globalSettings?.backgroundImage || 'none'
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

watch(status, (s) => {
  switch (s) {
    case 'connected':
      statusDot.value = 'online'
      break
    case 'connecting':
      statusDot.value = 'connecting'
      break
    case 'error':
      statusDot.value = 'error'
      break
    default:
      statusDot.value = 'offline'
  }
})

// Read global terminal settings from localStorage (cached by SettingsPage)
const globalSettingsVersion = ref(0)
function getGlobalTerminalSettings() {
  try {
    const raw = localStorage.getItem('rex-terminal-settings')
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

// Default terminal font family
const DEFAULT_FONT_FAMILY = "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace"

// Listen for settings changes from SettingsPage
function onTerminalSettingsChanged(e: Event) {
  const detail = (e as CustomEvent).detail
  if (detail) {
    // Force containerStyle to re-evaluate
    globalSettingsVersion.value++
    // Apply theme change immediately
    if (terminal.value && detail.theme) {
      terminal.value.options.theme = getTerminalTheme(detail.theme)
    }
    // Apply font change immediately
    if (terminal.value) {
      if (detail.fontFamily) {
        terminal.value.options.fontFamily = detail.fontFamily
      }
      if (detail.fontSize) {
        terminal.value.options.fontSize = detail.fontSize
      }
    }
  }
}

onMounted(() => {
  if (!containerRef.value) return

  const globalSettings = getGlobalTerminalSettings()

  // Build terminal options: per-resource props > global settings > defaults
  const termOptions: Record<string, unknown> = {}
  const theme = props.theme || globalSettings?.theme
  if (theme) termOptions.theme = getTerminalTheme(theme)
  if (props.fontSize) termOptions.fontSize = props.fontSize
  else if (globalSettings?.fontSize) termOptions.fontSize = Number(globalSettings.fontSize) || 14
  termOptions.fontFamily = globalSettings?.fontFamily || DEFAULT_FONT_FAMILY
  if (props.cursorStyle) termOptions.cursorStyle = props.cursorStyle
  if (props.cursorBlink !== undefined) termOptions.cursorBlink = props.cursorBlink

  const term = createTerminal(containerRef.value, termOptions)

  const search = new SearchAddon()
  term.loadAddon(search)
  searchAddon.value = search

  term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
    if (e.ctrlKey && e.key === 'f' && e.type === 'keydown') {
      e.preventDefault()
      showSearch.value = !showSearch.value
    }
    return true
  })

  // Hub 自动判断直连/Agent 隧道
  connect({ resourceId: props.resourceId })

  const resizeObserver = new ResizeObserver(() => {
    fit()
    // 通知父组件终端尺寸
    emit('terminal-resize', term.cols, term.rows)
  })
  resizeObserver.observe(containerRef.value)

  // 初始尺寸
  emit('terminal-resize', term.cols, term.rows)

  onBeforeUnmount(() => {
    resizeObserver.disconnect()
    dispose()
  })
})

onMounted(() => {
  window.addEventListener('terminal-settings-changed', onTerminalSettingsChanged)
})

onBeforeUnmount(() => {
  window.removeEventListener('terminal-settings-changed', onTerminalSettingsChanged)
})

function onContextMenu(e: MouseEvent) {
  contextMenuX.value = e.clientX
  contextMenuY.value = e.clientY
  showContextMenu.value = true
}

function handleFind() {
  showSearch.value = true
}

function handleReconnect() {
  connect({ resourceId: props.resourceId })
}

function handleCopyAddress() {
  const address = props.host ? `${props.host}:${props.port || 22}` : ''
  if (address) {
    navigator.clipboard.writeText(address)
    toast.value?.push(t('terminal.copyAddress'), 'success')
  }
}

function handleOpenSftp() {
  emit('toggle-sftp')
}

</script>

<template>
  <div class="terminal-view">
    <!-- 终端状态栏 -->
    <div class="tv-statusbar mono">
      <span class="tv-status-item">
        <StatusDot :status="statusDot" />
        <span v-if="status === 'connected'">{{ host }}:{{ port || 22 }}</span>
        <span v-else-if="status === 'connecting'">{{ t('terminal.connecting') }}</span>
        <span v-else-if="status === 'error'" class="tv-error">{{ errorMessage }}</span>
        <span v-else>{{ t('terminal.disconnected') }}</span>
      </span>
      <span v-if="status === 'connected'" class="tv-status-item muted">{{ protocol?.toUpperCase() }}</span>
      <span v-if="status === 'connected'" class="tv-status-item tv-file-btn" :title="t('terminal.toggleFileBrowser')" @click.stop="emit('toggle-sftp')">📁</span>
    </div>

    <!-- 终端容器 -->
    <div
      ref="containerRef"
      class="tv-container"
      :style="containerStyle"
      @contextmenu.prevent="onContextMenu"
    >
      <TerminalSearch
        :visible="showSearch"
        :search-addon="searchAddon"
        @close="showSearch = false"
      />
      <TerminalContextMenu
        :visible="showContextMenu"
        :x="contextMenuX"
        :y="contextMenuY"
        :terminal="terminal"
        :encoding="terminalEncoding"
        @close="showContextMenu = false"
        @find="handleFind"
        @reconnect="handleReconnect"
        @disconnect="disconnect"
        @set-encoding="handleSetEncoding"
        @copy-address="handleCopyAddress"
        @open-sftp="handleOpenSftp"
      />
    </div>

    <!-- 断开覆盖层 -->
    <div v-if="status === 'disconnected' || status === 'error'" class="tv-overlay">
      <div class="tv-overlay-content">
        <p class="tv-overlay-text mono">
          {{ status === 'error' ? errorMessage || t('terminal.connectionError') : t('terminal.sessionDisconnected') }}
        </p>
        <button class="tv-reconnect-btn" @click="handleReconnect">{{ t('terminal.reconnect') }}</button>
      </div>
    </div>

    <!-- 移动端浮动工具栏 -->
    <MobileTerminalBar :terminal="terminal" />
    <Toast ref="toast" />
  </div>
</template>

<style scoped>
.terminal-view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  position: relative;
}

.tv-statusbar {
  height: 24px;
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: 0 var(--space-3);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  font-size: var(--text-xs);
  color: var(--text-muted);
  flex-shrink: 0;
}

.tv-status-item {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}

.tv-error {
  color: var(--danger);
}

.tv-file-btn {
  cursor: pointer;
  margin-left: auto;
  opacity: 0.6;
  transition: opacity var(--transition);
}
.tv-file-btn:hover {
  opacity: 1;
}

.tv-container {
  flex: 1;
  min-height: 0;
  background: #0d1117;
  overflow: hidden;
  position: relative;
}

.tv-container :deep(.xterm) {
  padding: 0;
}

.tv-overlay {
  position: absolute;
  inset: 24px 0 0 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(2px);
  z-index: 10;
}

.tv-overlay-content {
  text-align: center;
}

.tv-overlay-text {
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin-bottom: var(--space-3);
}

.tv-reconnect-btn {
  padding: var(--space-2) var(--space-4);
  background: var(--accent);
  color: var(--text-on-accent);
  border: none;
  border-radius: var(--radius);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: opacity var(--transition);
}

.tv-reconnect-btn:hover {
  opacity: 0.9;
}
</style>
