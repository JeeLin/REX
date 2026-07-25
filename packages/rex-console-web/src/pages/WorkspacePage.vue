<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, defineAsyncComponent, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { Splitpanes, Pane } from 'splitpanes'
import 'splitpanes/dist/splitpanes.css'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import { useKeyboardShortcuts } from '@/composables/useKeyboardShortcuts'
import ShortcutPanel from '@/features/workspace/ShortcutPanel.vue'
import ResourceProperties from '@/features/workspace/ResourceProperties.vue'
import CommandPalette from '@/features/workspace/CommandPalette.vue'
import TerminalView from '@/features/terminal/TerminalView.vue'
import FilesDrawer from '@/features/files/FilesDrawer.vue'
import { PROTOCOL_COLORS } from '@/features/resource/protocols'
import { useWorkspaceStore } from '@/stores/workspace'

// 懒加载重型组件，拆分 chunk
const SqlPage = defineAsyncComponent(() => import('@/features/sql/SqlPage.vue'))
const RedisPage = defineAsyncComponent(() => import('@/features/redis/RedisPage.vue'))
const FilesPage = defineAsyncComponent(() => import('@/features/files/FilesPage.vue'))

const { t } = useI18n()
const router = useRouter()
const wsStore = useWorkspaceStore()

interface Tab {
  id: string
  label: string
  protocol: 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3'
  resourceId?: string
  environmentId?: string
  connectionMode?: string
  agentId?: string
  host?: string
  port?: number
  username?: string
  password?: string
  database?: string
  status: 'connecting' | 'connected' | 'disconnected' | 'error'
  color?: string
  renaming?: boolean
  broadcast?: boolean
  // Terminal settings
  theme?: string
  fontSize?: number
  opacity?: number
  cursorStyle?: string
  cursorBlink?: boolean
  backgroundImage?: string
  encoding?: string
}

const tabs = ref<Tab[]>([])
const activeTab = ref<string>('')
const splitDirection = ref<'row' | 'column'>('row')
const paneTabs = ref<string[]>([])
const splitCount = ref(1)
const currentPane = ref(0)

watch(() => wsStore.pendingResource, (resource) => {
  if (!resource) return
  openResourceFromTree(resource)
  wsStore.consumePending()
}, { immediate: true })

// Command palette
const showCommandPalette = ref(false)

function handleKeydown(e: KeyboardEvent) {
  // Ctrl+K: Command palette
  if (e.ctrlKey && e.key === 'k') {
    e.preventDefault()
    showCommandPalette.value = !showCommandPalette.value
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleKeydown)
})

const now = ref(new Date().toLocaleTimeString('zh-CN', { hour12: false }))
const timer = setInterval(() => {
  now.value = new Date().toLocaleTimeString('zh-CN', { hour12: false })
}, 1000)
onBeforeUnmount(() => {
  clearInterval(timer)
  document.removeEventListener('mousemove', onSftpDrag)
  document.removeEventListener('mouseup', onSftpDragEnd)
})

const terminalSize = ref<{ cols: number; rows: number } | null>(null)

// SFTP drawer state (SSH Tab 内)
const showSftpDrawer = ref(false)
const sftpDrawerHeight = ref(240)
let sftpDragStartY = 0
let sftpDragStartH = 0

function toggleSftpDrawer() {
  showSftpDrawer.value = !showSftpDrawer.value
}

function startSftpDrag(e: MouseEvent) {
  sftpDragStartY = e.clientY
  sftpDragStartH = sftpDrawerHeight.value
  document.addEventListener('mousemove', onSftpDrag)
  document.addEventListener('mouseup', onSftpDragEnd)
  document.body.style.cursor = 'row-resize'
  document.body.style.userSelect = 'none'
}

function onSftpDrag(e: MouseEvent) {
  const delta = sftpDragStartY - e.clientY
  sftpDrawerHeight.value = Math.min(700, Math.max(120, sftpDragStartH + delta))
}

function onSftpDragEnd() {
  document.removeEventListener('mousemove', onSftpDrag)
  document.removeEventListener('mouseup', onSftpDragEnd)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}

function formatConnection(tab: Tab): string {
  const proto = tab.protocol.toUpperCase()
  const host = tab.host || '—'
  const port = tab.port || (tab.protocol === 'ssh' ? 22 : tab.protocol === 'mysql' ? 3306 : tab.protocol === 'redis' ? 6379 : 5432)
  return `${proto} ${host}:${port}`
}

function onTerminalResize(cols: number, rows: number) {
  terminalSize.value = { cols, rows }
}

function onEncodingChange(encoding: string) {
  const tab = tabs.value.find(t => t.id === activeTab.value)
  if (tab) tab.encoding = encoding
}

function currentPaneTabInfo(paneIndex: number) {
  const tabId = paneTabs.value[paneIndex]
  return tabs.value.find(t => t.id === tabId) ?? tabs.value.find(t => t.id === activeTab.value)
}
const activeTabInfo = computed(() => tabs.value.find(t => t.id === activeTab.value))
// Per-pane tab sync: keep paneTabs array aligned with splitCount
function syncPanes() {
  while (paneTabs.value.length < splitCount.value) {
    paneTabs.value.push(activeTab.value)
  }
  if (paneTabs.value.length > splitCount.value) {
    paneTabs.value.length = splitCount.value
  }
  if (currentPane.value >= splitCount.value) {
    currentPane.value = Math.max(0, splitCount.value - 1)
  }
}
watch(activeTab, syncPanes)
watch(splitCount, syncPanes)


function openResourceFromTree(node: {
  id: string
  name: string
  protocol?: string
  host?: string
  port?: number
  username?: string
  environmentId?: string
}) {
  // 去重：相同 resourceId 不重复打开
  const resourceId = node.id
  const existing = tabs.value.find(t => t.resourceId === resourceId)
  if (existing) {
    activeTab.value = existing.id
    return
  }

  const id = `tab-${Date.now()}`
  const protocol = (node.protocol || 'ssh') as Tab['protocol']
  tabs.value.push({
    id,
    label: node.name,
    protocol,
    resourceId,
    environmentId: node.environmentId,
    host: node.host,
    port: node.port,
    username: node.username,
    status: 'connecting',
  })
  activeTab.value = id
}

// Tab 右键菜单
const tabContextMenu = ref<{ show: boolean; x: number; y: number; tabId: string }>({ show: false, x: 0, y: 0, tabId: '' })
const tabColors = ['#f85149', '#3fb950', '#58a6ff', '#d29922', '#8b5cf6', '#e8912d', '#f0883e', '#a371f7']

function onTabContextMenu(e: MouseEvent, tabId: string) {
  e.preventDefault()
  tabContextMenu.value = { show: true, x: e.clientX, y: e.clientY, tabId }
}

function closeTab(id: string) {
  const idx = tabs.value.findIndex(t => t.id === id)
  if (idx < 0) return
  tabs.value.splice(idx, 1)
  if (tabs.value.length === 0) {
    activeTab.value = ''
    return
  }
  if (activeTab.value === id) {
    activeTab.value = tabs.value[Math.min(idx, tabs.value.length - 1)]!.id
  }
}

function closeOtherTabs(id: string) {
  tabs.value = tabs.value.filter(t => t.id === id)
  activeTab.value = id
}

function closeTabsRight(id: string) {
  const idx = tabs.value.findIndex(t => t.id === id)
  if (idx >= 0) tabs.value.splice(idx + 1)
  if (!tabs.value.find(t => t.id === activeTab.value)) {
    activeTab.value = tabs.value[tabs.value.length - 1]!.id
  }
}

function closeTabsLeft(id: string) {
  const idx = tabs.value.findIndex(t => t.id === id)
  if (idx > 0) tabs.value.splice(0, idx)
  if (!tabs.value.find(t => t.id === activeTab.value)) {
    activeTab.value = tabs.value[0]!.id
  }
}

function closeAllTabs() {
  tabs.value = []
  activeTab.value = ''
}

function duplicateTab(id: string) {
  const tab = tabs.value.find(t => t.id === id)
  if (!tab) return
  const newId = `tab-${Date.now()}`
  tabs.value.push({ ...tab, id: newId, status: 'connecting' })
  activeTab.value = newId
  tabContextMenu.value.show = false
}

// Tab broadcast mode
function toggleBroadcast(tabId: string) {
  const tab = tabs.value.find(t => t.id === tabId)
  if (tab) tab.broadcast = !tab.broadcast
  tabContextMenu.value.show = false
}

function getBroadcastTargets(): Tab[] {
  const current = tabs.value.find(t => t.id === activeTab.value)
  if (!current?.broadcast || current.protocol !== 'ssh') return []
  return tabs.value.filter(t => t.id !== current.id && t.protocol === 'ssh' && t.status === 'connected')
}

function onBroadcastInput(data: string) {
  const targets = getBroadcastTargets()
  for (const tab of targets) {
    const el = document.querySelector(`[data-tab-id="${tab.id}"]`)
    if (el) el.dispatchEvent(new CustomEvent('terminal-input', { detail: data }))
  }
}

// Tab 拖拽排序
const dragTabId = ref('')

function onTabDragStart(e: DragEvent, tabId: string) {
  dragTabId.value = tabId
  e.dataTransfer!.effectAllowed = 'move'
}

function onTabDragOver(e: DragEvent, targetId: string) {
  e.preventDefault()
  e.dataTransfer!.dropEffect = 'move'
}

function onTabDrop(e: DragEvent, targetId: string) {
  e.preventDefault()
  if (!dragTabId.value || dragTabId.value === targetId) return
  const fromIdx = tabs.value.findIndex(t => t.id === dragTabId.value)
  const toIdx = tabs.value.findIndex(t => t.id === targetId)
  if (fromIdx < 0 || toIdx < 0) return
  const moved = tabs.value.splice(fromIdx, 1)[0]
  if (moved) tabs.value.splice(toIdx, 0, moved)
  dragTabId.value = ''
}

function setTabColor(color: string) {
  const tab = tabs.value.find(t => t.id === tabContextMenu.value.tabId)
  if (tab) tab.color = color
  tabContextMenu.value.show = false
}

function startRename(id: string) {
  const tab = tabs.value.find(t => t.id === id)
  if (tab) tab.renaming = true
  tabContextMenu.value.show = false
}

function finishRename(id: string, newLabel: string) {
  const tab = tabs.value.find(t => t.id === id)
  if (tab) {
    tab.label = newLabel || tab.label
    tab.renaming = false
  }
}

// 资源属性
const showProps = ref(false)
const propsTabId = ref('')

function openProperties(tabId: string) {
  propsTabId.value = tabId
  showProps.value = true
  tabContextMenu.value.show = false
}

const propsResource = computed(() => {
  const tab = tabs.value.find(t => t.id === propsTabId.value)
  if (!tab) return undefined
  return {
    name: tab.label,
    protocol: tab.protocol,
    host: tab.host || '',
    port: tab.port?.toString() || '',
    user: tab.username || '',
    password: '',
    privateKey: '',
    passphrase: '',
    encoding: tab.encoding || 'UTF-8',
    scrollback: 10000,
    cursorStyle: tab.cursorStyle || 'block',
    cursorBlink: tab.cursorBlink ?? true,
    theme: tab.theme || 'default',
    fontSize: tab.fontSize || 14,
    opacity: tab.opacity ?? 100,
    backgroundImage: tab.backgroundImage || 'none',
    keepalive: true,
    keepaliveInterval: 60,
    color: tab.color || '',
    notes: '',
  }
})

function onPropsSave(data: any) {
  const tab = tabs.value.find(t => t.id === propsTabId.value)
  if (!tab) return
  tab.theme = data.theme
  tab.fontSize = data.fontSize
  tab.opacity = data.opacity
  tab.cursorStyle = data.cursorStyle
  tab.cursorBlink = data.cursorBlink
  tab.backgroundImage = data.backgroundImage
}

// 分栏操作
function splitHorizontal() {
  splitCount.value++
  splitDirection.value = 'row'
}
function splitVertical() {
  splitCount.value++
  splitDirection.value = 'column'
}
function closePane(idx: number) {
  if (splitCount.value > 1) {
    splitCount.value--
    if (currentPane.value >= splitCount.value) {
      currentPane.value = splitCount.value - 1
    }
  }
}

// 快捷键面板
const showShortcuts = ref(false)

// Tab status 更新
function onTabStatusChange(tabId: string, status: Tab['status']) {
  const tab = tabs.value.find(t => t.id === tabId)
  if (tab) tab.status = status
}

// 布局预设
type LayoutPreset = 'single' | 'left-right' | 'top-bottom' | 'grid-four' | 'main-side'
const currentLayout = ref<LayoutPreset>('single')

function applyLayout(preset: LayoutPreset) {
  currentLayout.value = preset
  switch (preset) {
    case 'single':
      splitCount.value = 1
      splitDirection.value = 'row'
      break
    case 'left-right':
      splitCount.value = 2
      splitDirection.value = 'row'
      break
    case 'top-bottom':
      splitCount.value = 2
      splitDirection.value = 'column'
      break
    case 'grid-four':
      splitCount.value = 4
      splitDirection.value = 'row'
      break
    case 'main-side':
      splitCount.value = 2
      splitDirection.value = 'row'
      break
  }
}

// 协议状态点颜色
function statusColor(status: Tab['status']): StatusDotStatus {
  switch (status) {
    case 'connected': return 'online'
    case 'connecting': return 'connecting'
    case 'error': return 'error'
    default: return 'offline'
  }
}

// 快捷键
useKeyboardShortcuts([
  { key: 't', ctrl: true, handler: () => {
    // Ctrl+T 新建 SSH tab
    const id = `tab-${Date.now()}`
    tabs.value.push({ id, label: 'New Tab', protocol: 'ssh', status: 'connecting' })
    activeTab.value = id
  } },
  { key: 'w', ctrl: true, handler: () => {
    const idx = tabs.value.findIndex(t => t.id === activeTab.value)
    if (idx >= 0 && tabs.value.length > 1) {
      tabs.value.splice(idx, 1)
      activeTab.value = tabs.value[Math.max(0, idx - 1)]!.id
    }
  } },
  { key: 'Tab', ctrl: true, handler: () => {
    const idx = tabs.value.findIndex(t => t.id === activeTab.value)
    activeTab.value = tabs.value[(idx + 1) % tabs.value.length]!.id
  } },
  { key: '\\', ctrl: true, handler: splitHorizontal },
  { key: '\\', ctrl: true, shift: true, handler: splitVertical },
  { key: '1', alt: true, handler: () => applyLayout('single') },
  { key: '2', alt: true, handler: () => applyLayout('left-right') },
  { key: '3', alt: true, handler: () => applyLayout('top-bottom') },
  { key: '4', alt: true, handler: () => applyLayout('grid-four') },
  { key: '5', alt: true, handler: () => applyLayout('main-side') },
  { key: 'F1', handler: () => { showShortcuts.value = !showShortcuts.value } },
  { key: 'b', ctrl: true, handler: () => {
    if (activeTabInfo.value?.protocol === 'ssh') toggleSftpDrawer()
  } },
  { key: 'B', ctrl: true, shift: true, handler: () => {
    if (activeTab.value) toggleBroadcast(activeTab.value)
  } },
  // Ctrl+N: 新建连接
  { key: 'n', ctrl: true, handler: () => { router.push('/resource-new') } },
  // Alt+6-9: 跳转到第 6-9 个标签
  { key: '6', alt: true, handler: () => { if (tabs.value[5]) activeTab.value = tabs.value[5].id } },
  { key: '7', alt: true, handler: () => { if (tabs.value[6]) activeTab.value = tabs.value[6].id } },
  { key: '8', alt: true, handler: () => { if (tabs.value[7]) activeTab.value = tabs.value[7].id } },
  { key: '9', alt: true, handler: () => { if (tabs.value[8]) activeTab.value = tabs.value[8].id } },
])
</script>

<template>
  <div class="workspace">
    <!-- Tab bar -->
    <div class="ws-tabs">
      <div
        v-for="tab in tabs"
        :key="tab.id"
        class="ws-tab mono"
        :class="{ 'ws-tab--active': activeTab === tab.id }"
        draggable="true"
        @click="activeTab = tab.id; paneTabs[currentPane] = tab.id"
        @contextmenu="onTabContextMenu($event, tab.id)"
        @dragstart="onTabDragStart($event, tab.id)"
        @dragover="onTabDragOver($event, tab.id)"
        @drop="onTabDrop($event, tab.id)"
      >
        <span v-if="tab.color" class="ws-tab-color" :style="{ background: tab.color }" />
        <span class="ws-tab-dot" :style="{ background: PROTOCOL_COLORS[tab.protocol] || 'var(--text-muted)' }" />
        <input
          v-if="tab.renaming"
          class="ws-tab-rename-input mono"
          :value="tab.label"
          autofocus
          @blur="finishRename(tab.id, ($event.target as HTMLInputElement).value)"
          @keydown.enter="($event.target as HTMLInputElement).blur()"
          @keydown.escape="finishRename(tab.id, tab.label)"
          @click.stop
        />
        <span v-else>{{ tab.label }}</span>
        <span v-if="tab.broadcast" class="ws-tab-broadcast" title="Broadcast mode active">📡</span>
        <StatusDot :status="statusColor(tab.status)" style="margin-left: 4px" />
        <button class="ws-tab-close" @click.stop="closeTab(tab.id)">×</button>
      </div>
      <button class="ws-tab-add" title="New connection (Ctrl+T)">+</button>
    </div>

    <!-- Tab context menu -->
    <Teleport to="body">
      <div v-if="tabContextMenu.show" class="tab-ctx-overlay" @click="tabContextMenu.show = false" @contextmenu.prevent="tabContextMenu.show = false" />
      <div v-if="tabContextMenu.show" class="tab-ctx-menu" :style="{ top: tabContextMenu.y + 'px', left: tabContextMenu.x + 'px' }">
        <div class="tab-ctx-item" @click="startRename(tabContextMenu.tabId)">✏️ {{ t('workspace.rename') }}</div>
        <div class="tab-ctx-item" @click="duplicateTab(tabContextMenu.tabId)">📋 {{ t('workspace.duplicate') }}</div>
        <div class="tab-ctx-item" @click="toggleBroadcast(tabContextMenu.tabId)">
          {{ tabs.find(tab => tab.id === tabContextMenu.tabId)?.broadcast ? '📡 ' + t('workspace.stopBroadcast') : '📡 ' + t('workspace.broadcastInput') }}
        </div>
        <div class="tab-ctx-separator" />
        <div class="tab-ctx-item" @click="closeTab(tabContextMenu.tabId)">{{ t('workspace.close') }}</div>
        <div class="tab-ctx-item" @click="closeOtherTabs(tabContextMenu.tabId)">{{ t('workspace.closeOthers') }}</div>
        <div class="tab-ctx-item" @click="closeTabsLeft(tabContextMenu.tabId)">{{ t('workspace.closeLeft') }}</div>
        <div class="tab-ctx-item" @click="closeTabsRight(tabContextMenu.tabId)">{{ t('workspace.closeRight') }}</div>
        <div class="tab-ctx-item" @click="closeAllTabs()">{{ t('workspace.closeAll') }}</div>
        <div class="tab-ctx-separator" />
        <div class="tab-ctx-item" @click="openProperties(tabContextMenu.tabId)">⚙ {{ t('workspace.properties') }}</div>
        <div class="tab-ctx-separator" />
        <div class="tab-ctx-label muted">{{ t('workspace.color') }}</div>
        <div class="tab-ctx-colors">
          <button
            v-for="c in tabColors"
            :key="c"
            class="tab-ctx-color"
            :style="{ background: c }"
            @click="setTabColor(c)"
          />
        </div>
      </div>
    </Teleport>


    <div class="ws-main-area">
      <!-- Split panes -->
      <div class="ws-body">
        <Splitpanes
          :horizontal="splitDirection === 'column'"
          class="ws-split"
        >
          <Pane v-for="i in splitCount" :key="i" :size="100 / splitCount" :min-size="20" @click="currentPane = i - 1">
            <div class="ws-pane" :class="{ 'ws-pane--active': currentPane === i - 1 }">
              <div class="ws-pane-header mono">
                <span>{{ currentPaneTabInfo(i - 1)?.label || t('workspace.noTabOpen') }}</span>
                <div class="ws-pane-actions">
                  <button class="ws-pane-btn" :title="t('workspace.splitH')" @click="splitHorizontal">⊞</button>
                  <button class="ws-pane-btn" :title="t('workspace.splitV')" @click="splitVertical">⊟</button>
                  <button v-if="splitCount > 1" class="ws-pane-btn" :title="t('workspace.closePane')" @click="closePane(i - 1)">×</button>
                </div>
              </div>

              <!-- Terminal (SSH) + SFTP Drawer -->
              <div v-if="currentPaneTabInfo(i - 1)?.protocol === 'ssh'" class="ws-ssh-area">
                <TerminalView
                  :tab-id="paneTabs[i - 1]!"
                  :resource-id="currentPaneTabInfo(i - 1)?.resourceId || ''"
                  :host="currentPaneTabInfo(i - 1)?.host"
                  :port="currentPaneTabInfo(i - 1)?.port"
                  :protocol="currentPaneTabInfo(i - 1)?.protocol"
                  :theme="currentPaneTabInfo(i - 1)?.theme"
                  :font-size="currentPaneTabInfo(i - 1)?.fontSize"
                  :opacity="currentPaneTabInfo(i - 1)?.opacity"
                  :cursor-style="currentPaneTabInfo(i - 1)?.cursorStyle"
                  :cursor-blink="currentPaneTabInfo(i - 1)?.cursorBlink"
                  :background-image="currentPaneTabInfo(i - 1)?.backgroundImage"
                  @update:status="onTabStatusChange(paneTabs[i - 1]!, $event === 'online' ? 'connected' : $event === 'connecting' ? 'connecting' : $event === 'error' ? 'error' : 'disconnected')"
                  @terminal-resize="onTerminalResize"
                  @encoding-change="onEncodingChange"
                  @toggle-sftp="toggleSftpDrawer"
                />
                <div v-if="showSftpDrawer" class="ws-sftp-drawer" :style="{ height: sftpDrawerHeight + 'px' }">
                  <div class="ws-sftp-drag-handle" @mousedown.prevent="startSftpDrag" />
                  <FilesDrawer
                    :resource-id="currentPaneTabInfo(i - 1)?.resourceId"
                    :host="currentPaneTabInfo(i - 1)?.host"
                    :port="currentPaneTabInfo(i - 1)?.port"
                    :username="currentPaneTabInfo(i - 1)?.username"
                    :password="currentPaneTabInfo(i - 1)?.password"
                  />
                </div>
              </div>

              <!-- SQL (MySQL / PostgreSQL / SQLite) -->
              <SqlPage
                v-else-if="['mysql', 'postgresql', 'sqlite'].includes(currentPaneTabInfo(i - 1)?.protocol || '')"
                :resource-id="currentPaneTabInfo(i - 1)?.resourceId"
                :host="currentPaneTabInfo(i - 1)?.host"
                :port="currentPaneTabInfo(i - 1)?.port"
                :username="currentPaneTabInfo(i - 1)?.username"
                :password="currentPaneTabInfo(i - 1)?.password"
                :database="currentPaneTabInfo(i - 1)?.database"
                :db-type="currentPaneTabInfo(i - 1)?.protocol"
                :protocol="currentPaneTabInfo(i - 1)?.protocol"
              />

              <!-- Redis -->
              <RedisPage
                v-else-if="currentPaneTabInfo(i - 1)?.protocol === 'redis'"
                :resource-id="currentPaneTabInfo(i - 1)?.resourceId"
                :host="currentPaneTabInfo(i - 1)?.host"
                :port="currentPaneTabInfo(i - 1)?.port"
                :password="currentPaneTabInfo(i - 1)?.password"
              />

              <!-- Files (SFTP / S3) -->
              <FilesPage
                v-else-if="['sftp', 's3'].includes(currentPaneTabInfo(i - 1)?.protocol || '')"
                :resource-id="currentPaneTabInfo(i - 1)?.resourceId"
                :protocol="currentPaneTabInfo(i - 1)?.protocol === 's3' ? 's3' : 'sftp'"
                :host="currentPaneTabInfo(i - 1)?.host"
                :port="currentPaneTabInfo(i - 1)?.port"
                :username="currentPaneTabInfo(i - 1)?.username"
                :password="currentPaneTabInfo(i - 1)?.password"
              />

              <!-- Empty state -->
              <div v-else class="ws-component-placeholder">
                <div class="ws-placeholder-text muted">
                  {{ t('workspace.noConnectionDesc') }}
                </div>
              </div>
            </div>
          </Pane>
        </Splitpanes>
      </div>
    </div>

    <!-- Status bar -->
    <div class="ws-statusbar mono">
      <span class="ws-status-item">
        <StatusDot :status="activeTabInfo ? statusColor(activeTabInfo.status) : 'offline'" />
        {{ activeTabInfo ? formatConnection(activeTabInfo) : t('workspace.noConnection') }}
      </span>
      <span v-if="activeTabInfo?.protocol === 'ssh' && terminalSize" class="ws-status-item">
        {{ terminalSize.cols }}×{{ terminalSize.rows }}
      </span>
      <span v-if="activeTabInfo?.protocol === 'ssh'" class="ws-status-item">{{ activeTabInfo.encoding || 'UTF-8' }}</span>
      <span v-if="activeTabInfo?.broadcast" class="ws-status-item ws-broadcast-indicator">📡 {{ t('workspace.broadcastIndicator') }}</span>
      <span class="ws-status-spacer" />
      <span class="ws-status-item ws-quick-actions">
        <button class="ws-action-btn" title="Split horizontal" @click="splitHorizontal">⊞</button>
        <button class="ws-action-btn" title="Split vertical" @click="splitVertical">⊟</button>
      </span>
      <span class="ws-status-item">{{ now }}</span>
    </div>

    <!-- Shortcut panel -->
    <ShortcutPanel :show="showShortcuts" @close="showShortcuts = false" />

    <!-- Resource properties dialog -->
    <ResourceProperties
      v-model:show="showProps"
      :resource="propsResource"
      @save="onPropsSave"
    />

    <!-- Command palette -->
    <CommandPalette
      :visible="showCommandPalette"
      @close="showCommandPalette = false"
    />
  </div>
</template>

<style scoped>
.workspace {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-deep);
}

/* Tab bar */
.ws-tabs {
  height: var(--tabbar-height);
  display: flex;
  align-items: stretch;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  overflow-x: auto;
  scrollbar-width: none;
}
.ws-tabs::-webkit-scrollbar { display: none; }
.ws-tab {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: 0 var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-muted);
  border-right: 1px solid var(--border);
  cursor: pointer;
  white-space: nowrap;
  transition: color var(--transition), background var(--transition);
}
.ws-tab:hover {
  color: var(--text-secondary);
  background: var(--bg-hover);
}
.ws-tab--active {
  color: var(--text-primary);
  background: var(--bg-deep);
  border-bottom: 2px solid var(--accent);
}
.ws-tab-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}
.ws-tab-color {
  width: 4px;
  height: 16px;
  border-radius: 2px;
  flex-shrink: 0;
}
.ws-tab-rename-input {
  background: var(--bg-deep);
  border: 1px solid var(--accent);
  border-radius: 2px;
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  padding: 0 4px;
  width: 120px;
  outline: none;
}
.ws-tab-close {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-md);
  cursor: pointer;
  padding: 0 2px;
  line-height: 1;
  border-radius: var(--radius-sm);
  transition: color var(--transition), background var(--transition);
}
.ws-tab-close:hover {
  color: var(--danger);
  background: rgba(248, 81, 73, 0.15);
}
.ws-tab-add {
  padding: 0 var(--space-3);
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-lg);
  cursor: pointer;
  transition: color var(--transition);
}
.ws-tab-add:hover {
  color: var(--accent);
}
.ws-tab-broadcast {
  font-size: 10px;
  margin-left: 2px;
}


/* Main area */
.ws-main-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

/* Split panes */
.ws-body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.ws-split {
  height: 100%;
}
:deep(.splitpanes__splitter) {
  background-color: var(--border);
  min-width: 3px;
  min-height: 3px;
}
:deep(.splitpanes__splitter:hover) {
  background-color: var(--accent);
}

/* Pane */
.ws-pane {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-deep);
}
.ws-pane--active {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.ws-pane-header {
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--space-3);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  font-size: var(--text-xs);
  color: var(--text-muted);
}
.ws-pane-actions {
  display: flex;
  gap: var(--space-1);
}
.ws-pane-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-xs);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: var(--radius-sm);
  transition: color var(--transition);
}
.ws-pane-btn:hover {
  color: var(--accent);
}

/* SSH Tab + SFTP drawer */
.ws-ssh-area {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.ws-sftp-drawer {
  flex-shrink: 0;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.ws-sftp-drag-handle {
  height: 4px;
  cursor: row-resize;
  background: var(--border);
  flex-shrink: 0;
  transition: background var(--transition);
}
.ws-sftp-drag-handle:hover {
  background: var(--accent);
}

/* Placeholder for non-SSH protocols */
.ws-component-placeholder {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
}
.ws-placeholder-text {
  font-size: var(--text-md);
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.ws-placeholder-sub {
  font-size: var(--text-xs);
}

/* Status bar */
.ws-statusbar {
  height: var(--statusbar-height);
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: 0 var(--space-3);
  background: var(--bg-elevated);
  border-top: 1px solid var(--border);
  font-size: var(--text-xs);
  color: var(--text-muted);
}
.ws-status-item {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}
.ws-status-spacer {
  flex: 1;
}
.ws-quick-actions {
  display: flex;
  gap: var(--space-1);
}
.ws-action-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-xs);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: var(--radius-sm);
  transition: color var(--transition), background var(--transition);
}
.ws-action-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}
.ws-broadcast-indicator {
  color: var(--accent, #E8912D);
  font-weight: 600;
}

/* 手机端适配 */
@media (max-width: 768px) {
  .ws-statusbar .ws-status-item:nth-child(n+2) { display: none; }
  .ws-statusbar .ws-status-item:last-child { display: flex; }
  .ws-quick-actions { display: flex !important; }
}

/* Tab 右键菜单 */
.tab-ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 200;
}
.tab-ctx-menu {
  position: fixed;
  z-index: 210;
  min-width: 180px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  padding: var(--space-1) 0;
}
.tab-ctx-item {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-primary);
  cursor: pointer;
  transition: background var(--transition);
}
.tab-ctx-item:hover {
  background: var(--bg-hover);
}
.tab-ctx-separator {
  height: 1px;
  background: var(--border);
  margin: var(--space-1) 0;
}
.tab-ctx-label {
  padding: var(--space-1) var(--space-3);
  font-size: var(--text-xs);
}
.tab-ctx-colors {
  display: flex;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-3) var(--space-2);
}
.tab-ctx-color {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition: border-color var(--transition);
}
.tab-ctx-color:hover {
  border-color: var(--text-primary);
}
</style>
