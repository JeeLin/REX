<script setup lang="ts">
import { ref, shallowRef, onMounted, onBeforeUnmount, watch } from 'vue'
import { useTerminal } from './useTerminal'
import { SearchAddon } from '@xterm/addon-search'
import TerminalSearch from './TerminalSearch.vue'
import TerminalContextMenu from './TerminalContextMenu.vue'
import MobileTerminalBar from './MobileTerminalBar.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'

const props = defineProps<{
  tabId: string
  resourceId: string
  /** 以下字段仅用于状态栏显示，不用于连接 */
  host?: string
  port?: number
  protocol?: string
}>()

const emit = defineEmits<{
  'terminal-resize': [cols: number, rows: number]
  'toggle-sftp': []
}>()

const containerRef = ref<HTMLDivElement>()
const { terminal, status, errorMessage, createTerminal, connect, disconnect, fit, dispose } =
  useTerminal()

// Search
const searchAddon = shallowRef<SearchAddon | null>(null)
const showSearch = ref(false)

// Context menu
const showContextMenu = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)

const statusDot = ref<StatusDotStatus>('offline')

watch(status, (s) => {
  switch (s) {
    case 'connected':
      statusDot.value = 'online'
      break
    case 'connecting':
      statusDot.value = 'connecting'
      break
    default:
      statusDot.value = 'offline'
  }
})

onMounted(() => {
  if (!containerRef.value) return

  const term = createTerminal(containerRef.value)

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
</script>

<template>
  <div class="terminal-view">
    <!-- 终端状态栏 -->
    <div class="tv-statusbar mono">
      <span class="tv-status-item">
        <StatusDot :status="statusDot" />
        <span v-if="status === 'connected'">{{ host }}:{{ port || 22 }}</span>
        <span v-else-if="status === 'connecting'">Connecting...</span>
        <span v-else-if="status === 'error'" class="tv-error">{{ errorMessage }}</span>
        <span v-else>Disconnected</span>
      </span>
      <span v-if="status === 'connected'" class="tv-status-item muted">{{ protocol?.toUpperCase() }}</span>
      <span v-if="status === 'connected'" class="tv-status-item tv-file-btn" @click.stop="emit('toggle-sftp')" title="Toggle file browser (Ctrl+B)">📁</span>
    </div>

    <!-- 终端容器 -->
    <div
      ref="containerRef"
      class="tv-container"
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
        @close="showContextMenu = false"
        @find="handleFind"
        @reconnect="handleReconnect"
        @disconnect="disconnect"
      />
    </div>

    <!-- 断开覆盖层 -->
    <div v-if="status === 'disconnected' || status === 'error'" class="tv-overlay">
      <div class="tv-overlay-content">
        <p class="tv-overlay-text mono">
          {{ status === 'error' ? errorMessage || 'Connection error' : 'Session disconnected' }}
        </p>
        <button class="tv-reconnect-btn" @click="handleReconnect">Reconnect</button>
      </div>
    </div>

    <!-- 移动端浮动工具栏 -->
    <MobileTerminalBar :terminal="terminal" />
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
  padding: 4px 0;
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
