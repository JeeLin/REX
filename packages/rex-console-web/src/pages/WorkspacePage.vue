<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch, defineOptions, provide } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useWorkspacePersistence } from '@/composables/useWorkspacePersistence'
import { usePaneLayout } from '@/composables/usePaneLayout'
import { useTabs, nextTabId, type Tab } from '@/composables/useTabs'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import ContextMenu from '@/components/ui/ContextMenu.vue'
import { useKeyboardShortcuts } from '@/composables/useKeyboardShortcuts'
import { useSftpDrawer } from '@/composables/useSftpDrawer'
import ShortcutPanel from '@/features/workspace/ShortcutPanel.vue'
import ResourceProperties from '@/features/workspace/ResourceProperties.vue'
import CommandPalette from '@/features/workspace/CommandPalette.vue'
import PaneNode from '@/features/workspace/PaneNode.vue'
import { PROTOCOL_COLORS, PROTOCOL_ICONS } from '@/features/resource/protocols'
import { PANE_CTX, type PaneCtx } from '@/features/workspace/paneContext'
import { useWorkspaceStore } from '@/stores/workspace'

defineOptions({ name: 'WorkspacePage' })

const { t } = useI18n()
const router = useRouter()
const wsStore = useWorkspaceStore()

const dragOverPane = ref<string | null>(null)

// 树状布局
const {
  root: paneLayoutRoot,
  activePaneId,
  allLeaves,
  lastFocusedPaneId,
  focusPane,
  splitPane,
  closePane: treeClosePane,
  applyLayoutPreset,
  setPaneTab,
  serialize: serializeLayout,
  deserialize: deserializeLayout,
} = usePaneLayout()

// Tab 管理
const {
  tabs,
  activeTab,
  activeTabInfo,
  tabContextMenu,
  dragTabId,
  tabColors,
  findTab,
  formatConnection,
  openResource,
  closeTab,
  toggleBroadcast,
  finishRename,
  setTabColor,
  onTabStatusChange: onTabStatusChangeFromTabs,
  onTabContextMenu,
  handleTabCtxAction,
  onTabDragStart,
  onTabDragOver,
  onTabDrop,
  onTabDragEnd,
} = useTabs({ activePaneId, setPaneTab })

watch(() => wsStore.pendingResource, (resource) => {
  if (!resource) return
  openResource(resource)
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

// 工作区状态保活：切换页面回来时恢复 tab
const { restore } = useWorkspacePersistence({ tabs, activeTab, paneLayoutSerialize: serializeLayout, paneLayoutDeserialize: deserializeLayout, allLeaves, setPaneTab })

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
  // 从 localStorage 恢复上次的工作区状态
  restore()
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
})

const terminalSize = ref<{ cols: number; rows: number } | null>(null)

// SFTP drawer
const { show: showSftpDrawer, height: sftpDrawerHeight, toggle: toggleSftpDrawer, startDrag: startSftpDrag } = useSftpDrawer()

function onTerminalResize(cols: number, rows: number) {
  terminalSize.value = { cols, rows }
}

function onEncodingChange(encoding: string) {
  const tab = findTab(activeTab.value)
  if (tab) tab.encoding = encoding
}

// 提供分栏渲染上下文给递归的 PaneNode / PaneLeaf
provide<PaneCtx>(PANE_CTX, {
  activePaneId,
  allLeaves,
  focusPane,
  dragOverPane,
  splitHorizontal,
  splitVertical,
  closePane: treeClosePane,
  setPaneTab,
  findTab,
  activeTabInfo,
  onPaneContextMenu,
  onPaneDragEnter,
  onPaneDragLeave,
  onPaneDrop,
  onTabStatusChange: onTabStatusChangeFromTabs,
  onTerminalResize,
  onEncodingChange,
  showSftpDrawer,
  sftpDrawerHeight,
  toggleSftpDrawer,
  startSftpDrag,
})

// Tab 右键菜单相关本地 UI 状态
const showQuickConnect = ref(false)

const paneContextMenu = ref<{ show: boolean; x: number; y: number; paneId: string }>({ show: false, x: 0, y: 0, paneId: '' })

function onPaneContextMenu(e: MouseEvent, paneId: string) {
  e.preventDefault()
  e.stopPropagation()
  paneContextMenu.value = { show: true, x: e.clientX, y: e.clientY, paneId }
}

function handlePaneCtxAction(action: string) {
  const paneId = paneContextMenu.value.paneId
  if (!paneId) return
  switch (action) {
    case 'splitRight': splitPane(paneId, 'right'); break
    case 'splitDown': splitPane(paneId, 'down'); break
    case 'close': treeClosePane(paneId); break
  }
  paneContextMenu.value.show = false
}

// 关闭 pane 按钮（header 上的 ×）使用模板内直接调用 treeClosePane

// 资源属性
const showProps = ref(false)
const propsTabId = ref('')

function openProperties(tabId: string) {
  propsTabId.value = tabId
  showProps.value = true
  tabContextMenu.value.show = false
}

function disconnectTab(tabId: string) {
  tabContextMenu.value.show = false
  closeTab(tabId)
}

// 委托纯 tab 动作给 useTabs，本地只处理涉及本页 UI 状态的项
function localHandleTabCtxAction(action: string) {
  const id = tabContextMenu.value.tabId
  if (!id) return
  switch (action) {
    case 'new': showQuickConnect.value = true; break
    case 'props': openProperties(id); break
    case 'disconnect': disconnectTab(id); break
    default:
      // rename/duplicate/broadcast/close/closeOthers/closeLeft/closeRight/closeAll
      handleTabCtxAction(action)
  }
  tabContextMenu.value.show = false
}

// Double-click tab to split pane
function onTabDoubleClick(tabId: string) {
  if (allLeaves.value.length !== 1) return
  currentLayout.value = 'left-right'
  applyLayoutPreset('left-right')
  setPaneTab(activePaneId.value, tabId)
}

// Pane drag & drop handlers
function onPaneDragEnter(paneId: string) {
  dragOverPane.value = paneId
}

function onPaneDragLeave(paneId: string) {
  if (dragOverPane.value === paneId) {
    dragOverPane.value = null
  }
}

function onPaneDrop(e: DragEvent, targetPaneId: string) {
  e.preventDefault()
  dragOverPane.value = null
  const tabId = e.dataTransfer!.getData('text/tab-id')
  if (!tabId) return
  // 清除源 pane 中的 tab
  for (const leaf of allLeaves.value) {
    if (leaf.tabId === tabId && leaf.id !== targetPaneId) {
      setPaneTab(leaf.id, null)
    }
  }
  const targetLeaf = allLeaves.value.find((l) => l.id === targetPaneId)
  if (targetLeaf) {
    setPaneTab(targetLeaf.id, tabId)
    activePaneId.value = targetLeaf.id
  }
}

const propsResource = computed(() => {
  const tab = tabs.value.find(t => t.id === propsTabId.value)
  if (!tab) return undefined
  return {
    name: tab.label,
    protocol: tab.protocol,
    host: '',
    port: '',
    user: '',
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

function onPropsSave(data: Pick<Tab, 'theme' | 'fontSize' | 'opacity' | 'cursorStyle' | 'cursorBlink' | 'backgroundImage'>) {
  const tab = tabs.value.find(t => t.id === propsTabId.value)
  if (!tab) return
  tab.theme = data.theme
  tab.fontSize = data.fontSize
  tab.opacity = data.opacity
  tab.cursorStyle = data.cursorStyle
  tab.cursorBlink = data.cursorBlink
  tab.backgroundImage = data.backgroundImage
}

// 分栏操作：带参时作用于参数 pane；不带参时优先用最近聚焦的 pane，
// 使状态栏按钮 / Ctrl+\ 作用于用户正在交互的 pane，而非陈旧的 activePaneId。
function splitHorizontal(paneId?: string) {
  splitPane(paneId || lastFocusedPaneId.value || activePaneId.value, 'right')
}
function splitVertical(paneId?: string) {
  splitPane(paneId || lastFocusedPaneId.value || activePaneId.value, 'down')
splitPane(paneId || lastFocusedPaneId.value || activePaneId.value, 'down')
}

function toggleFullscreen() {
  if (!document.fullscreenElement) {
    document.documentElement.requestFullscreen()
  } else {
    document.exitFullscreen()
  }
}

// 快捷键面板
const showShortcuts = ref(false)

// 布局预设
type LayoutPreset = 'single' | 'left-right' | 'top-bottom' | 'grid-four' | 'main-side'
const currentLayout = ref<LayoutPreset>('single')

function applyLayout(preset: LayoutPreset) {
  currentLayout.value = preset
  applyLayoutPreset(preset)
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
    const id = nextTabId()
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
  // 移动端隐藏桌面风格快捷键面板（触屏无键盘快捷键，改触屏友好交互）
  { key: 'F1', handler: () => { if (window.innerWidth >= 768) showShortcuts.value = !showShortcuts.value } },
  { key: 'b', ctrl: true, handler: () => {
    if (activeTabInfo.value?.protocol === 'ssh') toggleSftpDrawer()
  } },
  { key: 'B', ctrl: true, shift: true, handler: () => {
    if (activeTab.value) toggleBroadcast(activeTab.value)
  } },
  // Ctrl+N: 新建连接
  { key: 'n', ctrl: true, handler: () => { router.push('/resource-new') } },
  // Alt+6-9: 跳转到第 6-9 个标签
  { key: '6', alt: true, handler: () => { if (tabs.value[5]) { activeTab.value = tabs.value[5].id; setPaneTab(activePaneId.value, tabs.value[5].id) } } },
  { key: '7', alt: true, handler: () => { if (tabs.value[6]) { activeTab.value = tabs.value[6].id; setPaneTab(activePaneId.value, tabs.value[6].id) } } },
  { key: '8', alt: true, handler: () => { if (tabs.value[7]) { activeTab.value = tabs.value[7].id; setPaneTab(activePaneId.value, tabs.value[7].id) } } },
  { key: '9', alt: true, handler: () => { if (tabs.value[8]) { activeTab.value = tabs.value[8].id; setPaneTab(activePaneId.value, tabs.value[8].id) } } },
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
        :class="{ 'ws-tab--active': activeTab === tab.id, 'ws-tab--dragging': dragTabId === tab.id }"
        draggable="true"
        :title="t('workspace.splitHint')"
        @click="activeTab = tab.id; setPaneTab(activePaneId, tab.id)"
        @dblclick="onTabDoubleClick(tab.id)"
        @contextmenu="onTabContextMenu($event, tab.id)"
        @dragstart="onTabDragStart($event, tab.id)"
        @dragover="onTabDragOver($event, tab.id)"
        @drop="onTabDrop($event, tab.id)"
        @dragend="onTabDragEnd"
      >
        <span
          class="ws-tab-pico"
          :style="{ background: PROTOCOL_COLORS[tab.protocol] || 'var(--text-muted)' }"
        >{{ PROTOCOL_ICONS[tab.protocol] || '?' }}</span>
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
        <StatusDot :status="statusColor(tab.status)" style="margin-left: auto" />
        <button class="ws-tab-close" @click.stop="closeTab(tab.id)">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6 6 18M6 6l12 12"/></svg>
        </button>
      </div>
    </div>

    <!-- Tab context menu -->
    <ContextMenu
      v-model="tabContextMenu.show"
      :x="tabContextMenu.x"
      :y="tabContextMenu.y"
      @select="(action: string) => localHandleTabCtxAction(action)"
    >
      <template #default="{ choose }">
        <div class="tab-ctx-item" @click="choose('new')">➕ {{ t('workspace.newConnection') }}</div>
        <div class="tab-ctx-separator" />
        <div class="tab-ctx-item" @click="choose('rename')">✏️ {{ t('workspace.rename') }}</div>
        <div class="tab-ctx-item" @click="choose('duplicate')">📋 {{ t('workspace.duplicate') }}</div>
        <div class="tab-ctx-item" @click="choose('broadcast')">
          {{ findTab(tabContextMenu.tabId)?.broadcast ? '📡 ' + t('workspace.stopBroadcast') : '📡 ' + t('workspace.broadcastInput') }}
        </div>
        <div class="tab-ctx-separator" />
        <div class="tab-ctx-item" @click="choose('close')">{{ t('workspace.close') }}</div>
        <div class="tab-ctx-item" @click="choose('closeOthers')">{{ t('workspace.closeOthers') }}</div>
        <div class="tab-ctx-item" @click="choose('closeLeft')">{{ t('workspace.closeLeft') }}</div>
        <div class="tab-ctx-item" @click="choose('closeRight')">{{ t('workspace.closeRight') }}</div>
        <div class="tab-ctx-item" @click="choose('closeAll')">{{ t('workspace.closeAll') }}</div>
        <div class="tab-ctx-separator" />
        <div class="tab-ctx-item" @click="choose('props')">⚙ {{ t('workspace.properties') }}</div>
        <div class="tab-ctx-item tab-ctx-item--danger" @click="choose('disconnect')">🔌 {{ t('workspace.disconnect') }}</div>
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
      </template>
    </ContextMenu>

    <!-- Pane context menu (right-click on a pane body) -->
    <ContextMenu
      v-model="paneContextMenu.show"
      :x="paneContextMenu.x"
      :y="paneContextMenu.y"
      @select="(action: string) => handlePaneCtxAction(action)"
    >
      <template #default="{ choose }">
        <div class="tab-ctx-item" @click="choose('splitRight')">⤵ {{ t('workspace.splitH') }}</div>
        <div class="tab-ctx-item" @click="choose('splitDown')">⤵ {{ t('workspace.splitV') }}</div>
        <div class="tab-ctx-item tab-ctx-item--danger" @click="choose('close')">{{ t('workspace.closePane') }}</div>
      </template>
    </ContextMenu>
    <div class="ws-main-area">
      <!-- 递归分栏渲染：每个容器节点用自身 direction 决定分栏方向，支持上下/左右混合嵌套 -->
      <div class="ws-body">
        <PaneNode :node="paneLayoutRoot" />
      </div>
    </div>

    <!-- Status bar -->
    <div class="ws-statusbar mono">
      <span class="ws-seg ws-seg--brand">
        <span class="ws-seg-dot" />
        workspace
      </span>
      <span class="ws-seg">{{ tabs.length }} resource{{ tabs.length === 1 ? '' : 's' }} open</span>
      <span v-if="activeTabInfo?.protocol === 'ssh' && terminalSize" class="ws-seg">{{ terminalSize.cols }}×{{ terminalSize.rows }}</span>
      <span v-if="activeTabInfo?.protocol === 'ssh'" class="ws-seg">{{ activeTabInfo.encoding || 'UTF-8' }}</span>
      <span v-if="activeTabInfo?.broadcast" class="ws-seg ws-broadcast-indicator">📡 {{ t('workspace.broadcastIndicator') }}</span>
      <span class="ws-seg ws-seg--spacer" />
      <span class="ws-seg ws-seg--agent">⟡ agent: edge-gw · 12 ms</span>
      <span class="ws-seg ws-seg--actions">
        <button class="ws-action-btn" title="Split horizontal" @click="() => splitHorizontal()">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="7" height="16" rx="1"/><rect x="14" y="4" width="7" height="16" rx="1"/></svg>
        </button>
        <button class="ws-action-btn" title="Split vertical" @click="() => splitVertical()">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="4" y="3" width="16" height="7" rx="1"/><rect x="4" y="14" width="16" height="7" rx="1"/></svg>
        </button>
      </span>
      <span class="ws-seg ws-seg--actions">
        <button class="ws-action-btn" title="Fullscreen" @click="toggleFullscreen">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M7 10 4 13l3 3M4 13h11M17 14l3-3-3-3M20 11H9"/></svg>
        </button>
      </span>
      <span class="ws-seg ws-seg--help" title="F1 help" @click="showShortcuts = !showShortcuts">F1 help</span>
      <span class="ws-seg ws-seg--help" title="Command palette (Ctrl+K)" @click="showCommandPalette = !showCommandPalette">⌘ 命令面板</span>
    </div>

    <!-- Shortcut panel -->
    <ShortcutPanel :show="showShortcuts" @close="showShortcuts = false" />

    <!-- Bottom-right shortcut guide toggle -->
    <button
      class="ws-shortcut-fab"
      :title="t('shortcuts.title')"
      :aria-label="t('shortcuts.title')"
      @click="showShortcuts = !showShortcuts"
    >
      ⌨
    </button>

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
  background: var(--bg-elevated);
  border-bottom: 1px solid var(--border);
  overflow-x: auto;
  scrollbar-width: none;
}
.ws-tabs::-webkit-scrollbar { display: none; }
.ws-tab {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: 0 var(--space-3) 0 var(--space-2);
  font-size: 12.5px;
  color: var(--text-muted);
  border-right: 1px solid var(--border);
  cursor: pointer;
  white-space: nowrap;
  border-top: 2px solid transparent;
  transition: color var(--transition), background var(--transition);
}
.ws-tab:hover {
  background: var(--bg-hover);
  color: var(--text);
}
.ws-tab--active {
  color: var(--text-primary);
  background: var(--bg-surface);
  border-top-color: var(--accent);
}
.ws-tab-pico {
  width: 16px;
  height: 16px;
  border-radius: 4px;
  display: grid;
  place-items: center;
  font-family: var(--font-mono);
  font-size: 9px;
  font-weight: 700;
  color: var(--on-ink);
  flex-shrink: 0;
  line-height: 1;
}
.ws-tab-label {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
  color: var(--text-dim, var(--text-muted));
  cursor: pointer;
  padding: 2px;
  line-height: 1;
  border-radius: var(--radius-sm);
  display: grid;
  place-items: center;
  width: 18px;
  height: 18px;
  opacity: 0.5;
  transition: color var(--transition), background var(--transition), opacity var(--transition);
}
.ws-tab-close:hover {
  color: var(--text);
  background: var(--bg-surface);
  opacity: 1;
}
.ws-tab-broadcast {
  font-size: 10px;
  margin-left: 2px;
}


/* Main area */
.ws-main-area {
  flex: 1;
  min-height: 0;
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

.ws-ssh-area {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
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

/* Status bar */
.ws-statusbar {
  height: var(--statusbar-height);
  display: flex;
  align-items: center;
  gap: 0;
  background: var(--bg-elevated);
  border-top: 1px solid var(--border);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-muted);
}
.ws-seg {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  height: 100%;
  border-right: 1px solid var(--border);
  white-space: nowrap;
}
.ws-seg:last-child {
  border-right: none;
}
.ws-seg--brand {
  background: var(--accent-soft);
  color: var(--accent);
}
.ws-seg--spacer {
  flex: 1;
  border-right: 0;
}
.ws-seg-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--success);
  flex-shrink: 0;
}
.ws-seg--agent {
  color: var(--text-muted);
}
.ws-seg--help {
  cursor: pointer;
}
.ws-seg--help:hover {
  background: var(--bg-hover);
  color: var(--text);
}
.ws-seg--actions {
  padding: 0 4px;
}
.ws-action-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: var(--radius-sm);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: color var(--transition), background var(--transition);
}
.ws-action-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}
.ws-broadcast-indicator {
  color: var(--accent);
  font-weight: 600;
}

/* 手机端适配 */
@media (max-width: 768px) {
  .ws-seg:nth-child(n+4) { display: none; }
  .ws-seg--actions { display: inline-flex !important; }
}

/* Tab 右键菜单 */
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
.tab-ctx-item--has-sub {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.tab-ctx-arrow {
  font-size: var(--text-xs);
  color: var(--text-muted);
  margin-left: var(--space-2);
}
.tab-ctx-item--danger {
  color: var(--danger);
}
.tab-ctx-item--danger:hover {
  background: rgba(248, 81, 73, 0.15);
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
/* Tab dragging feedback */
.ws-tab--dragging {
  opacity: 0.5;
}

/* Pane drag-over highlight */
.ws-pane--drag-over {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

/* Bottom-right shortcut guide toggle */
.ws-shortcut-fab {
  position: fixed;
  right: var(--space-3);
  bottom: var(--space-3);
  z-index: 1000;
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  line-height: 1;
  color: var(--text-secondary);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 50%;
  box-shadow: var(--shadow-md);
  cursor: pointer;
  transition: color var(--transition), background var(--transition), border-color var(--transition);
}
.ws-shortcut-fab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
  border-color: var(--accent);
}

/* 移动端：底栏（z-997）与右下浮动按钮簇会盖住/碰撞 FAB，移到左下并抬高避开 */
@media (max-width: 768px) {
  .ws-shortcut-fab {
    right: auto;
    left: var(--space-3);
    bottom: calc(56px + var(--space-3));
  }
}

</style>
