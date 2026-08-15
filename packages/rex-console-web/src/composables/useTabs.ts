import { ref, computed, type Ref } from 'vue'

export type TabProtocol = 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3'

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
  // Terminal settings
  theme?: string
  fontSize?: number
  opacity?: number
  cursorStyle?: string
  cursorBlink?: boolean
  backgroundImage?: string
  encoding?: string
}

export interface ResourceNode {
  id: string
  name: string
  protocol?: string
  environmentId?: string
}

export interface UseTabsDeps {
  activePaneId: Ref<string>
  setPaneTab: (paneId: string, tabId: string | null) => void
}

const TAB_COLORS = ['#f85149', '#3fb950', '#58a6ff', '#d29922', '#8b5cf6', '#e8912d', '#f0883e', '#a371f7']

// 单调递增序号 + 时间戳，保证同一毫秒内的多次开/复制也得到唯一 tab id
// （避免 tab-${Date.now()} 在并发调用时碰撞，产生重复 id）
let tabSeq = 0
function nextTabId(): string {
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
      status: 'connecting',
    })
    activeTab.value = id
    setPaneTab(activePaneId.value, id)
  }

  // ===== 关闭 =====
  function closeTab(id: string) {
    const idx = tabs.value.findIndex((t) => t.id === id)
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
    tabs.value = []
    activeTab.value = ''
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

  // ===== 右键菜单 =====
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
  }
}
