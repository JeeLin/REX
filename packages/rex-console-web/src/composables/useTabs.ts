import { ref, computed, type Ref } from 'vue'

export type TabProtocol = 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3' | 'sip' | 'sql'

export interface Tab {
  id: string
  label: string
  protocol: TabProtocol
  resourceId?: string
  environmentId?: string
  connectionMode?: string
  agentId?: string
  status: 'connecting' | 'connected' | 'disconnected' | 'error'
  color?: string
  renaming?: boolean
  broadcast?: boolean
  pinned?: boolean
  dirty?: boolean
  // v0.70.7：SQL 资源的子类（dialect，mysql/postgresql/sqlite）。连接时回写，
  // 之后经此字段直接路由 SQL 控制台，无需再次探测。
  subtype?: string
  // Terminal settings
  theme?: string
  fontSize?: number
  opacity?: number
  cursorStyle?: string
  cursorBlink?: boolean
  backgroundImage?: string
  encoding?: string
  // Snapshot of saved content for dirty detection (e.g. SQL query text)
  savedContent?: string
}

export interface ResourceNode {
  id: string
  name: string
  protocol?: string
  environmentId?: string
  subtype?: string
}

export interface UseTabsDeps {
  activePaneId: Ref<string>
  setPaneTab: (paneId: string, tabId: string | null) => void
}

const TAB_COLORS = ['#f85149', '#3fb950', '#58a6ff', '#d29922', '#8b5cf6', '#e8912d', '#f0883e', '#a371f7']

// 单调递增序号 + 时间戳，保证同一毫秒内的多次开/复制也得到唯一 tab id
// （避免 tab-${Date.now()} 在并发调用时碰撞，产生重复 id）
let tabSeq = 0
export function nextTabId(): string {
  tabSeq += 1
  return `tab-${Date.now()}-${tabSeq}`
}

export function useTabs(deps: UseTabsDeps) {
  const { activePaneId, setPaneTab } = deps

  const tabs = ref<Tab[]>([])
  const activeTab = ref<string>('')

  // 右键菜单状态 + 拖拽状态
  const tabContextMenu = ref<{ show: boolean; x: number; y: number; tabId: string }>({
    show: false,
    x: 0,
    y: 0,
    tabId: '',
  })
  const dragTabId = ref('')

  const activeTabInfo = computed(() => tabs.value.find((t) => t.id === activeTab.value) ?? null)

  function findTab(id: string): Tab | undefined {
    return tabs.value.find((t) => t.id === id)
  }

  function formatConnection(tab: Tab): string {
    return tab.protocol.toUpperCase()
  }

  // ===== 打开资源 =====
  function openResource(node: ResourceNode) {
    const resourceId = node.id
    const protocol = (node.protocol || 'ssh') as Tab['protocol']
    const existing = tabs.value.find((t) => t.resourceId === resourceId && t.protocol === protocol)
    if (existing) {
      activeTab.value = existing.id
      setPaneTab(activePaneId.value, existing.id)
      return
    }

    const id = nextTabId()
    tabs.value.push({
      id,
      label: node.name,
      protocol,
      resourceId,
      environmentId: node.environmentId,
      subtype: node.subtype,
      status: 'connecting',
    })
    activeTab.value = id
    setPaneTab(activePaneId.value, id)
  }

  function closeTab(id: string) {
    const idx = tabs.value.findIndex((t) => t.id === id)
    if (idx < 0) return
    const tab = tabs.value[idx]!
    trackClosedTab(tab)
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
    tabs.value = tabs.value.filter((t) => t.id === id)
    activeTab.value = id
  }

  function closeTabsRight(id: string) {
    const idx = tabs.value.findIndex((t) => t.id === id)
    if (idx >= 0) tabs.value.splice(idx + 1)
    if (!tabs.value.find((t) => t.id === activeTab.value)) {
      activeTab.value = tabs.value[tabs.value.length - 1]!.id
    }
  }

  function closeTabsLeft(id: string) {
    const idx = tabs.value.findIndex((t) => t.id === id)
    if (idx > 0) tabs.value.splice(0, idx)
    if (!tabs.value.find((t) => t.id === activeTab.value)) {
      activeTab.value = tabs.value[0]!.id
    }
  }

  function closeAllTabs() {
    tabs.value = tabs.value.filter(t => t.pinned)
    activeTab.value = tabs.value.length > 0 ? tabs.value[0]!.id : ''
  }

  // ===== Pin =====
  function togglePinTab(tabId: string) {
    const tab = findTab(tabId)
    if (tab) tab.pinned = !tab.pinned
    tabContextMenu.value.show = false
  }

  function duplicateTab(id: string) {
    const tab = findTab(id)
    if (!tab) return
    const newId = nextTabId()
    tabs.value.push({ ...tab, id: newId, status: 'connecting' })
    activeTab.value = newId
    tabContextMenu.value.show = false
  }

  // ===== 广播模式 =====
  function toggleBroadcast(tabId: string) {
    const tab = findTab(tabId)
    if (tab) tab.broadcast = !tab.broadcast
    tabContextMenu.value.show = false
  }

  // ===== 重命名 =====
  function startRename(id: string) {
    const tab = findTab(id)
    if (tab) tab.renaming = true
    tabContextMenu.value.show = false
  }

  function finishRename(id: string, newLabel: string) {
    const tab = findTab(id)
    if (tab) {
      tab.label = newLabel || tab.label
      tab.renaming = false
    }
  }

  // ===== 设色 =====
  function setTabColor(color: string) {
    const tab = findTab(tabContextMenu.value.tabId)
    if (tab) tab.color = color
    tabContextMenu.value.show = false
  }

  // ===== 状态更新 =====
  function onTabStatusChange(tabId: string, status: Tab['status']) {
    const tab = findTab(tabId)
    if (tab) tab.status = status
  }

  // ===== Tab history =====
  const tabHistory = ref<string[]>([])
  const tabHistoryIndex = ref(-1)
  const MAX_HISTORY = 50

  function pushHistory(tabId: string) {
    // Trim forward history when navigating to a new position
    if (tabHistoryIndex.value < tabHistory.value.length - 1) {
      tabHistory.value = tabHistory.value.slice(0, tabHistoryIndex.value + 1)
    }
    // Avoid duplicate consecutive entries
    if (tabHistory.value.length > 0 && tabHistory.value[tabHistory.value.length - 1] === tabId) return
    tabHistory.value.push(tabId)
    if (tabHistory.value.length > MAX_HISTORY) {
      tabHistory.value = tabHistory.value.slice(tabHistory.value.length - MAX_HISTORY)
    }
    tabHistoryIndex.value = tabHistory.value.length - 1
  }

  // ===== Closed tabs =====
  const closedTabs = ref<Tab[]>([])
  const MAX_CLOSED = 10

  function trackClosedTab(tab: Tab) {
    closedTabs.value.push({ ...tab })
    if (closedTabs.value.length > MAX_CLOSED) {
      closedTabs.value = closedTabs.value.slice(closedTabs.value.length - MAX_CLOSED)
    }
  }

  function goBack() {
    if (tabHistoryIndex.value <= 0) return
    tabHistoryIndex.value--
    const id = tabHistory.value[tabHistoryIndex.value]
    if (id && tabs.value.find(t => t.id === id)) {
      activeTab.value = id
    }
  }

  function goForward() {
    if (tabHistoryIndex.value >= tabHistory.value.length - 1) return
    tabHistoryIndex.value++
    const id = tabHistory.value[tabHistoryIndex.value]
    if (id && tabs.value.find(t => t.id === id)) {
      activeTab.value = id
    }
  }

  function reopenClosedTab() {
    if (closedTabs.value.length === 0) return
    const tab = closedTabs.value.pop()!
    tabs.value.push({ ...tab, status: 'connecting' })
    activeTab.value = tab.id
  }

  // ===== Workspace Export / Import =====
  function exportWorkspace(): string {
    const data = {
      version: 1,
      exportedAt: new Date().toISOString(),
      tabs: tabs.value.map(t => ({
        id: t.id,
        label: t.label,
        protocol: t.protocol,
        resourceId: t.resourceId,
        environmentId: t.environmentId,
        subtype: t.subtype,
        theme: t.theme,
        fontSize: t.fontSize,
        opacity: t.opacity,
        cursorStyle: t.cursorStyle,
        cursorBlink: t.cursorBlink,
        backgroundImage: t.backgroundImage,
        encoding: t.encoding,
        color: t.color,
        pinned: t.pinned,
      })),
      activeTab: activeTab.value,
      activePaneId: activePaneId.value,
    }
    return JSON.stringify(data, null, 2)
  }

  interface WorkspaceSnapshot {
    version: number
    exportedAt: string
    tabs: Array<{
      id: string
      label: string
      protocol: Tab['protocol']
      resourceId?: string
      environmentId?: string
      subtype?: string
      theme?: string
      fontSize?: number
      opacity?: number
      cursorStyle?: string
      cursorBlink?: boolean
      backgroundImage?: string
      encoding?: string
      color?: string
      pinned?: boolean
    }>
    activeTab?: string
    activePaneId?: string
  }

  function importWorkspace(data: string): void {
    const parsed = JSON.parse(data) as WorkspaceSnapshot
    if (!parsed.tabs || !Array.isArray(parsed.tabs)) {
      throw new Error('Invalid workspace file: missing tabs array')
    }

    // Clear existing tabs
    tabs.value = []

    // Recreate each tab by calling openResource for tabs with resourceId
    for (const saved of parsed.tabs) {
      if (saved.resourceId) {
        openResource({
          id: saved.resourceId,
          name: saved.label,
          protocol: saved.protocol,
          environmentId: saved.environmentId,
          subtype: saved.subtype,
        })
      } else {
        // Manually recreate tabs without a resource (e.g. new tabs)
        const id = nextTabId()
        tabs.value.push({
          id,
          label: saved.label,
          protocol: saved.protocol,
          status: 'connecting',
          theme: saved.theme,
          fontSize: saved.fontSize,
          opacity: saved.opacity,
          cursorStyle: saved.cursorStyle,
          cursorBlink: saved.cursorBlink,
          backgroundImage: saved.backgroundImage,
          encoding: saved.encoding,
          color: saved.color,
          pinned: saved.pinned,
        })
      }
    }

    // Restore active tab
    if (parsed.activeTab && tabs.value.find(t => t.id === parsed.activeTab)) {
      activeTab.value = parsed.activeTab
    } else if (tabs.value.length > 0) {
      activeTab.value = tabs.value[0]!.id
    }
  }

  // ===== Tab context menu =====
  function onTabContextMenu(e: MouseEvent, tabId: string) {
    e.preventDefault()
    tabContextMenu.value = { show: true, x: e.clientX, y: e.clientY, tabId }
  }

  function handleTabCtxAction(action: string) {
    const id = tabContextMenu.value.tabId
    if (!id) return
    switch (action) {
      case 'rename': startRename(id); break
      case 'duplicate': duplicateTab(id); break
      case 'broadcast': toggleBroadcast(id); break
      case 'close': closeTab(id); break
      case 'closeOthers': closeOtherTabs(id); break
      case 'closeLeft': closeTabsLeft(id); break
      case 'closeRight': closeTabsRight(id); break
      case 'closeAll': closeAllTabs(); break
      case 'pin': togglePinTab(id); break
    }
    tabContextMenu.value.show = false
  }

  // ===== 拖拽排序 =====
  function onTabDragStart(e: DragEvent, tabId: string) {
    dragTabId.value = tabId
    e.dataTransfer!.effectAllowed = 'move'
    e.dataTransfer!.setData('text/tab-id', tabId)
  }

  function onTabDragOver(e: DragEvent, _targetId: string) {
    e.preventDefault()
    e.dataTransfer!.dropEffect = 'move'
  }

  function onTabDrop(e: DragEvent, targetId: string) {
    e.preventDefault()
    if (!dragTabId.value || dragTabId.value === targetId) return
    const fromIdx = tabs.value.findIndex((t) => t.id === dragTabId.value)
    const toIdx = tabs.value.findIndex((t) => t.id === targetId)
    if (fromIdx < 0 || toIdx < 0) return
    const moved = tabs.value.splice(fromIdx, 1)[0]
    if (moved) tabs.value.splice(toIdx, 0, moved)
    dragTabId.value = ''
  }

  function onTabDragEnd() {
    dragTabId.value = ''
  }

  return {
    // state
    tabs,
    activeTab,
    activeTabInfo,
    tabContextMenu,
    dragTabId,
    tabColors: TAB_COLORS,
    // helpers
    findTab,
    formatConnection,
    // open / close / manage
    openResource,
    closeTab,
    closeOtherTabs,
    closeTabsRight,
    closeTabsLeft,
    closeAllTabs,
    duplicateTab,
    toggleBroadcast,
    startRename,
    finishRename,
    setTabColor,
    onTabStatusChange,
    // context menu
    onTabContextMenu,
    handleTabCtxAction,
    // drag
    onTabDragStart,
    onTabDragOver,
    onTabDrop,
    onTabDragEnd,
    // history & pin
    tabHistory,
    tabHistoryIndex,
    closedTabs,
    pushHistory,
    goBack,
    goForward,
    reopenClosedTab,
    togglePinTab,
    // workspace export / import
    exportWorkspace,
    importWorkspace,
  }
}
