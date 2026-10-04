<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch, defineOptions, provide } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useWorkspacePersistence } from '@/composables/useWorkspacePersistence'
import { usePaneLayout } from '@/composables/usePaneLayout'
import { useTabs, nextTabId, type Tab } from '@/composables/useTabs'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import ContextMenu from '@/components/ui/ContextMenu.vue'
import { useKeyboardShortcuts } from '@/composables/useKeyboardShortcuts'
import { useFullscreen } from '@/composables/useFullscreen'
import { isTypingTarget } from '@/utils/isTypingTarget'
import { useSftpDrawer } from '@/composables/useSftpDrawer'
import ResourceProperties from '@/features/workspace/ResourceProperties.vue'
import PaneNode from '@/features/workspace/PaneNode.vue'
import { PROTOCOL_COLORS, PROTOCOL_ICONS } from '@/features/resource/protocols'
import ToolbarSettings from '@/features/workspace/ToolbarSettings.vue'
import { useNotificationStore } from '@/stores/notification'
import { PANE_CTX, type PaneCtx } from '@/features/workspace/paneContext'
import WelcomePage from '@/features/workspace/WelcomePage.vue'
import { useWorkspaceStore } from '@/stores/workspace'
import { useShortcutsStore } from '@/stores/shortcuts'

defineOptions({ name: 'WorkspacePage' })

const { t } = useI18n()
const router = useRouter()
const wsStore = useWorkspaceStore()
const shortcutsStore = useShortcutsStore()
const notify = useNotificationStore()
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
  activateTab,
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
  // history
  pushHistory,
  goBack,
  goForward,
  reopenClosedTab,
    // workspace export / import
    exportWorkspace,
    importWorkspace,
  } = useTabs({ activePaneId, setPaneTab, allLeaves, focusPane })

watch(() => wsStore.pendingResource, (resource) => {
  if (!resource) return
  openResource(resource)
  wsStore.consumePending()
}, { immediate: true })

// Track tab switches for history navigation
watch(activeTab, (newTabId) => {
  if (newTabId) pushHistory(newTabId)
})

// Close confirmation for unsaved changes
const showConfirmClose = ref(false)
const pendingCloseTabId = ref('')

function requestCloseTab(tabId: string) {
  const tab = findTab(tabId)
  if (tab?.dirty) {
    pendingCloseTabId.value = tabId
    showConfirmClose.value = true
  } else {
    closeTab(tabId)
  }
}

function confirmCloseTab() {
  const id = pendingCloseTabId.value
  showConfirmClose.value = false
  pendingCloseTabId.value = ''
  if (id) closeTab(id)
}

function cancelCloseTab() {
  showConfirmClose.value = false
  pendingCloseTabId.value = ''
}

// Workspace export / import
function handleExportWorkspace() {
  try {
    const json = exportWorkspace()
    const blob = new Blob([json], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `workspace-${new Date().toISOString().slice(0, 10)}.rex-workspace.json`
    a.click()
    URL.revokeObjectURL(url)
    notify.success(t('workspace.exportSuccess'))
  } catch {
    notify.error(t('workspace.exportFailed'))
  }
}

const showImportDialog = ref(false)
const importData = ref('')

function handleImportPick() {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.json'
  input.onchange = async () => {
    const file = input.files?.[0]
    if (!file) return
    try {
      importData.value = await file.text()
      JSON.parse(importData.value) // validate
      showImportDialog.value = true
    } catch {
      notify.error(t('workspace.importInvalid'))
    }
  }
  input.click()
}

function confirmImport() {
  try {
    importWorkspace(importData.value)
    showImportDialog.value = false
    importData.value = ''
    notify.success(t('workspace.importSuccess'))
  } catch {
    notify.error(t('workspace.importFailed'))
  }
}

function cancelImport() {
  showImportDialog.value = false
  importData.value = ''
}

// Multi-window
function openInNewWindow() {
  const tab = findTab(tabContextMenu.value.tabId)
  if (!tab) return
  const params = new URLSearchParams({ resourceId: tab.resourceId || '', protocol: tab.protocol, window: '1' })
  window.open(`/workspace?${params.toString()}`, '_blank')
  tabContextMenu.value.show = false
}

// Toolbar settings
interface ToolbarConfig {
  splitH: boolean
  splitV: boolean
  fullscreen: boolean
  f1Help: boolean
  commandPalette: boolean
}

const toolbarConfig = ref<ToolbarConfig>({ splitH: true, splitV: true, fullscreen: true, f1Help: true, commandPalette: true })
const showToolbarSettings = ref(false)

function handleToolbarSettingsClickAway(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (!target.closest('.toolbar-settings-popover') && !target.closest('.ws-action-btn')) {
    showToolbarSettings.value = false
  }
}

function handleToolbarConfigUpdate(cfg: ToolbarConfig) {
  toolbarConfig.value = cfg
}


// Workspace toolbar entry delegates to the single global palette owned by AppLayout,
// so the toolbar button and Ctrl+K always open the same panel.
function toggleGlobalCommandPalette() {
  document.dispatchEvent(new CustomEvent('rex:command-palette-toggle'))
}

// 工作区状态保活：切换页面回来时恢复 tab
const { restore } = useWorkspacePersistence({ tabs, activeTab, paneLayoutSerialize: serializeLayout, paneLayoutDeserialize: deserializeLayout, allLeaves, setPaneTab })

onMounted(() => {
  document.addEventListener('keydown', handleSplitKeydown)
  document.addEventListener('click', handleToolbarSettingsClickAway)
  // 从 localStorage 恢复上次的工作区状态
  restore()
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleSplitKeydown)
  document.removeEventListener('click', handleToolbarSettingsClickAway)
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
  createTab: (protocol: string, label: string) => {
    const id = nextTabId()
    tabs.value.push({
      id,
      label,
      protocol: protocol as 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sqlite' | 's3' | 'sftp' | 'sip' | 'sql',
      status: 'connecting',
    })
    activateTab(id)
    return id
  }
})

// Tab 右键菜单相关本地 UI 状态

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
    case 'props': openProperties(id); break
    case 'disconnect': disconnectTab(id); break
    case 'close': requestCloseTab(id); break
    case 'openInNewWindow': openInNewWindow(); break
    default:
      // rename/duplicate/broadcast/closeOthers/closeLeft/closeRight/closeAll
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
    focusPane(targetLeaf.id)
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
}

// Ctrl+Shift+\ vertical split. With shift held the event key becomes '|' on most
// layouts, so the character-based matcher never fires: match the physical key code.
function handleSplitKeydown(e: KeyboardEvent) {
  if (isTypingTarget(e.target)) return
  if (!(e.code === 'Backslash' && (e.ctrlKey || e.metaKey) && e.shiftKey)) return
  if (isOverlayOpen()) return
  e.preventDefault()
  splitVertical()
}

// Modal/overlay surfaces that must swallow the split chord while open: the
// local dialogs and context menus of this page, plus the AppLayout-owned
// command palette / shortcut panel whose open state lives in the shortcuts store.
function isOverlayOpen(): boolean {
  if (showConfirmClose.value || showImportDialog.value || showProps.value) return true
  if (tabContextMenu.value.show || paneContextMenu.value.show) return true
  return shortcutsStore.show || shortcutsStore.paletteVisible
}

// Single fullscreen implementation shared with AppLayout: unsupported/rejected
// requests fall back to a toggleable UI flag instead of throwing.
const { isFullscreen, toggle: toggleFullscreen } = useFullscreen()

// 快捷键面板状态由 shortcuts store 提供（顶栏按钮 + F1 / 状态栏共用）

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

// Jump to the tab at the given index (1-based keys Alt+1~9).
function jumpToTab(index: number) {
  const tab = tabs.value[index]
  if (!tab) return
  // activateTab handles the pane focus/binding itself; see useTabs.activateTab.
  activateTab(tab.id)
}

// Cycle active tab by step, wrapping around the tab list.
function cycleTab(step: number) {
  if (tabs.value.length === 0) return
  const idx = tabs.value.findIndex(t => t.id === activeTab.value)
  const nextId = tabs.value[(idx + step + tabs.value.length) % tabs.value.length]!.id
  activateTab(nextId)
}

// 快捷键
useKeyboardShortcuts([
  // Alt+T: new SSH tab (replaces browser-reserved Ctrl+T)
  { key: 't', alt: true, handler: () => {
    const id = nextTabId()
    tabs.value.push({ id, label: 'New Tab', protocol: 'ssh', status: 'connecting' })
    activateTab(id)
  } },
  // Alt+W: close current tab (replaces browser-reserved Ctrl+W).
  // Routes through closeTab() so the tab lands in closedTabs and Alt+Shift+T
  // can restore it — a bare splice made "reopen closed tab" dead in practice.
  { key: 'w', alt: true, handler: () => {
    if (activeTab.value) requestCloseTab(activeTab.value)
  } },
  { key: '\\', ctrl: true, handler: splitHorizontal },
  // Ctrl+Alt+1-5: layout presets
  ...(['single', 'left-right', 'top-bottom', 'grid-four', 'main-side'] as const).map(
    (preset, i) => ({ key: String(i + 1), ctrl: true, alt: true, handler: () => applyLayout(preset) }),
  ),
  // 移动端隐藏桌面风格快捷键面板（触屏无键盘快捷键，改触屏友好交互）
  { key: 'F1', handler: () => { if (window.innerWidth >= 768) shortcutsStore.toggle() } },
  { key: 'b', ctrl: true, handler: () => {
    if (activeTabInfo.value?.protocol === 'ssh') toggleSftpDrawer()
  } },
  { key: 'B', ctrl: true, shift: true, handler: () => {
    if (activeTab.value) toggleBroadcast(activeTab.value)
  } },
  // Ctrl+Shift+N: 新建连接（保留原绑定；右键菜单 no-op 项已删除，
  // 真实新连接入口是 ResourcePanel 侧栏 wizard）
  { key: 'n', ctrl: true, shift: true, handler: () => { router.push('/workspace') } },
  // Alt+1-9: jump to the 1st-9th tab
  ...Array.from({ length: 9 }, (_, i) => ({ key: String(i + 1), alt: true, handler: () => jumpToTab(i) })),
  // Cmd/Ctrl+← : go back in tab history
  { key: 'ArrowLeft', ctrl: true, handler: goBack },
  // Cmd/Ctrl+→ : go forward in tab history
  { key: 'ArrowRight', ctrl: true, handler: goForward },
  // Ctrl+Shift+→ : next tab (replaces browser-reserved Ctrl+Tab)
  { key: 'ArrowRight', ctrl: true, shift: true, handler: () => cycleTab(1) },
  // Ctrl+Shift+← : previous tab (replaces browser-reserved Ctrl+Shift+Tab)
  { key: 'ArrowLeft', ctrl: true, shift: true, handler: () => cycleTab(-1) },
  // Alt+Shift+T : reopen closed tab
  { key: 't', alt: true, shift: true, handler: reopenClosedTab },
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
        @click="activateTab(tab.id)"
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
        <span v-if="tab.pinned" class="ws-tab-pin" title="Pinned">📌</span>
        <span v-if="tab.broadcast" class="ws-tab-broadcast" title="Broadcast mode active">📡</span>
        <StatusDot :status="statusColor(tab.status)" style="margin-left: auto" />
        <button class="ws-tab-close" @click.stop="requestCloseTab(tab.id)">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6 6 18M6 6l12 12" /></svg>
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
        <div class="tab-ctx-item" @click="choose('rename')">✏️ {{ t('workspace.rename') }}</div>
        <div class="tab-ctx-item" @click="choose('duplicate')">📋 {{ t('workspace.duplicate') }}</div>
        <div class="tab-ctx-item" @click="choose('broadcast')">
          {{ findTab(tabContextMenu.tabId)?.broadcast ? '📡 ' + t('workspace.stopBroadcast') : '📡 ' + t('workspace.broadcastInput') }}
        </div>
        <div class="tab-ctx-separator" />
        <div class="tab-ctx-item" @click="choose('pin')">
          {{ findTab(tabContextMenu.tabId)?.pinned ? '📌 Unpin' : '📌 Pin Tab' }}
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
        <div class="tab-ctx-item" @click="choose('openInNewWindow')">🪟 {{ t('workspace.openInNewWindow') }}</div>
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
        {{ t('workspace.statusbar.workspace', 'workspace') }}
      </span>
      <span class="ws-seg">{{ tabs.length }} {{ t('workspace.statusbar.resourcesOpen', 'resource(s) open') }}</span>
      <span v-if="activeTabInfo?.protocol === 'ssh' && terminalSize" class="ws-seg">{{ terminalSize.cols }}×{{ terminalSize.rows }}</span>
      <span v-if="activeTabInfo?.protocol === 'ssh'" class="ws-seg">{{ activeTabInfo.encoding || 'UTF-8' }}</span>
      <span v-if="activeTabInfo?.broadcast" class="ws-seg ws-broadcast-indicator">📡 {{ t('workspace.broadcastIndicator') }}</span>
      <span class="ws-seg ws-seg--spacer" />
      <span class="ws-seg ws-seg--actions">
        <button v-if="toolbarConfig.splitH" class="ws-action-btn" :title="t('workspace.toolbarSettings.splitH')" @click="() => splitHorizontal()">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="7" height="16" rx="1" /><rect x="14" y="4" width="7" height="16" rx="1" /></svg>
        </button>
        <button v-if="toolbarConfig.splitV" class="ws-action-btn" :title="t('workspace.toolbarSettings.splitV')" @click="() => splitVertical()">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="4" y="3" width="16" height="7" rx="1" /><rect x="4" y="14" width="16" height="7" rx="1" /></svg>
        </button>
      </span>
      <span class="ws-seg ws-seg--actions">
        <button
          v-if="toolbarConfig.fullscreen"
          class="ws-action-btn"
          :title="isFullscreen ? t('common.exitFullscreen', 'Exit fullscreen') : t('common.fullscreen')"
          :aria-label="isFullscreen ? t('common.exitFullscreen', 'Exit fullscreen') : t('common.fullscreen')"
          @click="toggleFullscreen"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M7 10 4 13l3 3M4 13h11M17 14l3-3-3-3M20 11H9" /></svg>
        </button>
      </span>
      <span v-if="toolbarConfig.f1Help" class="ws-seg ws-seg--help" :title="t('workspace.statusbar.f1Help', 'F1 help')" @click="shortcutsStore.toggle()">{{ t('workspace.statusbar.f1Help', 'F1 help') }}</span>
      <span v-if="toolbarConfig.commandPalette" class="ws-seg ws-seg--help" title="Command palette (Ctrl+K)" @click="toggleGlobalCommandPalette">⌘ {{ t('workspace.commandPalette', 'Command palette') }}</span>
      <span class="ws-seg ws-seg--actions">
        <button class="ws-action-btn" :title="t('workspace.exportWorkspace')" @click="handleExportWorkspace">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" /></svg>
        </button>
        <button class="ws-action-btn" :title="t('workspace.importWorkspace')" @click="handleImportPick">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" /></svg>
        </button>
      </span>
      <span class="ws-seg ws-seg--help" style="position: relative">
        <button class="ws-action-btn" :title="t('workspace.toolbarSettings.title')" @click="showToolbarSettings = !showToolbarSettings">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3" /><path d="M12 1v4M12 19v4M4.22 4.22l2.83 2.83M16.95 16.95l2.83 2.83M1 12h4M19 12h4M4.22 19.78l2.83-2.83M16.95 7.05l2.83-2.83" /></svg>
        </button>
        <Teleport to="body">
          <Transition name="menu">
            <div v-if="showToolbarSettings" class="toolbar-settings-popover" @click.stop>
              <ToolbarSettings @update:config="handleToolbarConfigUpdate" />
            </div>
          </Transition>
        </Teleport>
      </span>
    </div>

    <!-- Shortcut panel is rendered by AppLayout (shared shortcuts store) -->

    <!-- Resource properties dialog -->
    <ResourceProperties
      v-model:show="showProps"
      :resource="propsResource"
      @save="onPropsSave"
    />

    <!-- Close confirmation dialog -->
    <Teleport to="body">
      <Transition name="overlay">
        <div v-if="showConfirmClose" class="confirm-overlay" @click="cancelCloseTab" />
      </Transition>
      <Transition name="panel">
        <div v-if="showConfirmClose" class="confirm-dialog">
          <h3 class="confirm-title">Unsaved Changes</h3>
          <p class="confirm-msg">This tab has unsaved changes. Are you sure you want to close it?</p>
          <div class="confirm-actions">
            <button class="confirm-btn confirm-btn--cancel" @click="cancelCloseTab">Cancel</button>
            <button class="confirm-btn confirm-btn--danger" @click="confirmCloseTab">Close Tab</button>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Import confirmation dialog -->
    <Teleport to="body">
      <Transition name="overlay">
        <div v-if="showImportDialog" class="confirm-overlay" @click="cancelImport" />
      </Transition>
      <Transition name="panel">
        <div v-if="showImportDialog" class="confirm-dialog">
          <h3 class="confirm-title">{{ t('workspace.importConfirmTitle') }}</h3>
          <p class="confirm-msg">{{ t('workspace.importConfirm') }}</p>
          <div class="confirm-actions">
            <button class="confirm-btn confirm-btn--cancel" @click="cancelImport">{{ t('common.cancel') }}</button>
            <button class="confirm-btn confirm-btn--danger" @click="confirmImport">{{ t('common.confirm') }}</button>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Welcome page (shown when no tabs are open) -->
    <WelcomePage v-if="tabs.length === 0" />
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
  width: 6px;
  min-height: 6px;
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
  height: 34px;
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

/* Pin icon */
.ws-tab-pin {
  font-size: 10px;
  margin-left: 2px;
  opacity: 0.7;
}
.ws-tab--active .ws-tab-pin {
  opacity: 1;
}

/* Close confirmation dialog */
.confirm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(2px);
  z-index: 80;
}
.confirm-dialog {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 360px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  z-index: 90;
  padding: var(--space-4);
}
.confirm-title {
  font-size: var(--text-md);
  font-weight: 600;
  margin: 0 0 var(--space-2);
}
.confirm-msg {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  margin: 0 0 var(--space-4);
  line-height: 1.5;
}
.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
}
.confirm-btn {
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: background var(--transition), color var(--transition);
}
.confirm-btn:hover {
  background: var(--bg-hover);
}
.confirm-btn--danger {
  background: var(--danger);
  color: #fff;
  border-color: var(--danger);
}
.confirm-btn--danger:hover {
  opacity: 0.9;
}
.confirm-btn--danger:hover {
  opacity: 0.9;
}

/* Toolbar settings popover */
.toolbar-settings-popover {
  position: fixed;
  bottom: calc(var(--statusbar-height) + 4px);
  right: var(--space-4);
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: var(--shadow-lg);
  z-index: 80;
}
.menu-enter-active,
.menu-leave-active {
  transition: opacity var(--transition);
}
.menu-enter-from,
.menu-leave-to {
  opacity: 0;
}
</style>
