<script setup lang="ts">
import { ref, computed, onBeforeUnmount } from 'vue'
import { Splitpanes, Pane } from 'splitpanes'
import 'splitpanes/dist/splitpanes.css'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import { useKeyboardShortcuts } from '@/composables/useKeyboardShortcuts'
import ConnectionTree from '@/features/workspace/ConnectionTree.vue'
import ShortcutPanel from '@/features/workspace/ShortcutPanel.vue'
import ResourceProperties from '@/features/workspace/ResourceProperties.vue'
import TerminalView from '@/features/terminal/TerminalView.vue'
import SqlPage from '@/features/sql/SqlPage.vue'
import RedisPage from '@/features/redis/RedisPage.vue'
import FilesPage from '@/features/files/FilesPage.vue'
import { PROTOCOL_COLORS } from '@/features/resource/protocols'

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
}

interface SplitPane {
  id: string
  direction: 'row' | 'column'
  children: (SplitPane | string)[]
}

const tabs = ref<Tab[]>([])
const activeTab = ref<string>('')
const splitDirection = ref<'row' | 'column'>('row')
const panes = ref<string[]>([])
const splitCount = ref(1)

const now = ref(new Date().toLocaleTimeString('zh-CN', { hour12: false }))
const timer = setInterval(() => {
  now.value = new Date().toLocaleTimeString('zh-CN', { hour12: false })
}, 1000)
onBeforeUnmount(() => clearInterval(timer))

const activeTabInfo = computed(() => tabs.value.find(t => t.id === activeTab.value))

// 连接树
const treeCollapsed = ref(false)

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
  if (idx >= 0 && tabs.value.length > 1) {
    tabs.value.splice(idx, 1)
    if (activeTab.value === id) {
      activeTab.value = tabs.value[Math.max(0, idx - 1)]!.id
    }
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
    encoding: 'UTF-8',
    color: tab.color || '',
    notes: '',
  }
})

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
    // Ctrl+T 打开 Quick Connect 或新 SSH tab
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
        @click="activeTab = tab.id"
        @contextmenu="onTabContextMenu($event, tab.id)"
      >
        <span class="ws-tab-color" v-if="tab.color" :style="{ background: tab.color }" />
        <span class="ws-tab-dot" :style="{ background: PROTOCOL_COLORS[tab.protocol] || 'var(--text-muted)' }" />
        <input
          v-if="tab.renaming"
          class="ws-tab-rename-input mono"
          :value="tab.label"
          @blur="finishRename(tab.id, ($event.target as HTMLInputElement).value)"
          @keydown.enter="($event.target as HTMLInputElement).blur()"
          @keydown.escape="finishRename(tab.id, tab.label)"
          @click.stop
          autofocus
        />
        <span v-else>{{ tab.label }}</span>
        <StatusDot :status="statusColor(tab.status)" style="margin-left: 4px" />
        <button class="ws-tab-close" @click.stop="closeTab(tab.id)">×</button>
      </div>
      <button class="ws-tab-add" title="New connection (Ctrl+T)">+</button>
    </div>

    <!-- Tab context menu -->
    <Teleport to="body">
      <div v-if="tabContextMenu.show" class="tab-ctx-overlay" @click="tabContextMenu.show = false" @contextmenu.prevent="tabContextMenu.show = false" />
      <div v-if="tabContextMenu.show" class="tab-ctx-menu" :style="{ top: tabContextMenu.y + 'px', left: tabContextMenu.x + 'px' }">
        <div class="tab-ctx-item" @click="startRename(tabContextMenu.tabId)">✏️ Rename</div>
        <div class="tab-ctx-separator" />
        <div class="tab-ctx-item" @click="closeTab(tabContextMenu.tabId)">Close</div>
        <div class="tab-ctx-item" @click="closeOtherTabs(tabContextMenu.tabId)">Close Others</div>
        <div class="tab-ctx-item" @click="closeTabsRight(tabContextMenu.tabId)">Close Right</div>
        <div class="tab-ctx-separator" />
        <div class="tab-ctx-item" @click="openProperties(tabContextMenu.tabId)">⚙ Properties</div>
        <div class="tab-ctx-separator" />
        <div class="tab-ctx-label muted">Color</div>
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

    <!-- Connection tree sidebar -->
    <div v-show="!treeCollapsed" class="ws-tree" :style="{ width: '220px' }">
      <ConnectionTree @open-resource="openResourceFromTree" />
    </div>
    <button class="ws-tree-toggle" @click="treeCollapsed = !treeCollapsed" :title="treeCollapsed ? 'Show tree' : 'Hide tree'">
      {{ treeCollapsed ? '»' : '«' }}
    </button>

    <div class="ws-main-area">
      <!-- Split panes -->
      <div class="ws-body">
        <Splitpanes
          :horizontal="splitDirection === 'column'"
          class="ws-split"
          @resized="() => {}"
        >
          <Pane v-for="i in splitCount" :key="i" :size="100 / splitCount" :min-size="20">
            <div class="ws-pane">
              <div class="ws-pane-header mono">
                <span>{{ activeTabInfo?.label || 'No tab open' }}</span>
                <div class="ws-pane-actions">
                  <button class="ws-pane-btn" @click="splitHorizontal" title="Split horizontal (Ctrl+\)">⊞</button>
                  <button class="ws-pane-btn" @click="splitVertical" title="Split vertical (Ctrl+Shift+\)">⊟</button>
                  <button v-if="splitCount > 1" class="ws-pane-btn" @click="closePane(i - 1)" title="Close pane">×</button>
                </div>
              </div>

              <!-- Terminal (SSH) -->
              <TerminalView
                v-if="activeTabInfo?.protocol === 'ssh'"
                :tab-id="activeTab"
                :host="activeTabInfo?.host"
                :port="activeTabInfo?.port"
                :username="activeTabInfo?.username"
                :protocol="activeTabInfo?.protocol"
                :agent-mode="activeTabInfo?.connectionMode === 'agent'"
                :agent-id="activeTabInfo?.agentId"
                :resource-id="activeTabInfo?.resourceId"
                @update:status="onTabStatusChange(activeTab, $event === 'online' ? 'connected' : $event === 'connecting' ? 'connecting' : $event === 'error' ? 'error' : 'disconnected')"
              />

              <!-- SQL (MySQL / PostgreSQL / SQLite) -->
              <SqlPage
                v-else-if="['mysql', 'postgresql', 'sqlite'].includes(activeTabInfo?.protocol || '')"
                :resource-id="activeTabInfo?.resourceId"
                :host="activeTabInfo?.host"
                :port="activeTabInfo?.port"
                :username="activeTabInfo?.username"
                :password="activeTabInfo?.password"
                :database="activeTabInfo?.database"
                :db-type="activeTabInfo?.protocol"
                :protocol="activeTabInfo?.protocol"
              />

              <!-- Redis -->
              <RedisPage
                v-else-if="activeTabInfo?.protocol === 'redis'"
                :resource-id="activeTabInfo?.resourceId"
                :host="activeTabInfo?.host"
                :port="activeTabInfo?.port"
                :password="activeTabInfo?.password"
              />

              <!-- Files (SFTP / S3) -->
              <FilesPage
                v-else-if="['sftp', 's3'].includes(activeTabInfo?.protocol || '')"
                :resource-id="activeTabInfo?.resourceId"
                :protocol="activeTabInfo?.protocol as 'sftp' | 's3'"
                :host="activeTabInfo?.host"
                :port="activeTabInfo?.port"
                :username="activeTabInfo?.username"
                :password="activeTabInfo?.password"
              />

              <!-- Empty state -->
              <div v-else class="ws-component-placeholder">
                <div class="ws-placeholder-text muted">
                  No connection open. Click a resource in the sidebar or use Quick Connect.
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
        {{ activeTabInfo ? `${activeTabInfo.protocol.toUpperCase()} · ${activeTabInfo.host || 'no host'}` : 'No tab' }}
      </span>
      <span v-if="activeTabInfo?.protocol === 'ssh'" class="ws-status-item">UTF-8</span>
      <span class="ws-status-spacer" />
      <span class="ws-status-item ws-quick-actions">
        <button class="ws-action-btn" @click="splitHorizontal" title="Split horizontal">⊞</button>
        <button class="ws-action-btn" @click="splitVertical" title="Split vertical">⊟</button>
        <button class="ws-action-btn" title="Find">🔍</button>
      </span>
      <span class="ws-status-item">{{ now }}</span>
    </div>

    <!-- Shortcut panel -->
    <ShortcutPanel :show="showShortcuts" @close="showShortcuts = false" />

    <!-- Resource properties dialog -->
    <ResourceProperties
      v-model:show="showProps"
      :resource="propsResource"
    />
  </div>
</template>

<style scoped>
.workspace {
  height: 100%;
  display: flex;
  flex-direction: column;
  margin: calc(-1 * var(--space-5));
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

/* Connection tree sidebar */
.ws-tree {
  flex-shrink: 0;
  background: var(--bg-surface);
  border-right: 1px solid var(--border);
  overflow: hidden;
}
.ws-tree-toggle {
  width: 16px;
  flex-shrink: 0;
  background: var(--bg-surface);
  border: none;
  border-right: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 10px;
  cursor: pointer;
  transition: color var(--transition);
}
.ws-tree-toggle:hover {
  color: var(--accent);
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
