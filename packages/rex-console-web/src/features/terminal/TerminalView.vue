<script setup lang="ts">
import { ref, shallowRef, onMounted, onBeforeUnmount, watch } from 'vue'
import { useTerminal } from './useTerminal'
import { SearchAddon } from '@xterm/addon-search'
import TerminalSearch from './TerminalSearch.vue'
import TerminalContextMenu from './TerminalContextMenu.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'

const props = defineProps<{
  tabId: string
  host?: string
  port?: number
  username?: string
  protocol?: string
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

  // 加载搜索 addon
  const search = new SearchAddon()
  term.loadAddon(search)
  searchAddon.value = search

  // Ctrl+F 打开搜索栏
  term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
    if (e.ctrlKey && e.key === 'f' && e.type === 'keydown') {
      e.preventDefault()
      showSearch.value = !showSearch.value
    }
    return true
  })

  // 如果有连接信息，自动连接
  if (props.host && props.protocol === 'ssh') {
    connect({
      host: props.host,
      port: props.port || 22,
      username: props.username || 'root',
    })
  }

  // 监听窗口 resize
  const resizeObserver = new ResizeObserver(() => {
    fit()
  })
  resizeObserver.observe(containerRef.value)

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
  if (props.host && props.protocol === 'ssh') {
    connect({
      host: props.host,
      port: props.port || 22,
      username: props.username || 'root',
    })
  }
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
    </div>

    <!-- 终端容器 -->
    <div
      ref="containerRef"
      class="tv-container"
      @contextmenu.prevent="onContextMenu"
    >
      <!-- 终端内查找栏 -->
      <TerminalSearch
        :visible="showSearch"
        :search-addon="searchAddon"
        @close="showSearch = false"
      />
      <!-- 终端右键菜单 -->
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
  </div>
</template>

<style scoped>
.terminal-view {
  height: 100%;
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

.tv-container {
  flex: 1;
  min-height: 0;
  background: #0d1117; /* 终端背景色 */
  overflow: hidden;
  position: relative;
}

.tv-container :deep(.xterm) {
  padding: 4px 0;
}

.tv-overlay {
  position: absolute;
  inset: 24px 0 0 0; /* 避开状态栏 */
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
