import { ref, computed } from 'vue'
import type { Protocol } from '@/composables/useProtocol'

export type PanelComponent = 'terminal' | 'sql' | 'files' | 'redis' | 'unsupported'

const PROTOCOL_COMPONENT: Record<string, PanelComponent> = {
  ssh: 'terminal',
  mysql: 'sql',
  postgresql: 'sql',
  sqlite: 'sql',
  sftp: 'files',
  redis: 'redis',
}

export interface Tab {
  id: string
  name: string
  proto: Protocol
  resourceId: string
  panelIndex: number
  status: 'online' | 'offline' | 'connecting'
  component: PanelComponent
}

let tabCounter = 0

const tabs = ref<Tab[]>([])
const activeTabId = ref<string | null>(null)

export function useTabs() {
  const activeTab = computed(() => tabs.value.find((t) => t.id === activeTabId.value) ?? null)

  function addTab(name: string, proto: Protocol, resourceId: string): string | null {
    // Dedup: same resourceId → activate existing
    const existing = tabs.value.find((t) => t.resourceId === resourceId)
    if (existing) {
      activeTabId.value = existing.id
      return existing.id
    }

    tabCounter++
    const id = `tab-${tabCounter}`
    const tab: Tab = {
      id,
      name,
      proto,
      resourceId,
      panelIndex: 0,
      status: 'connecting',
      component: PROTOCOL_COMPONENT[proto] || 'unsupported',
    }
    tabs.value.push(tab)
    activeTabId.value = id

    // Simulate connection after a short delay
    setTimeout(() => {
      const t = tabs.value.find((x) => x.id === id)
      if (t) t.status = 'online'
    }, 800)

    return id
  }

  function closeTab(id: string) {
    const idx = tabs.value.findIndex((t) => t.id === id)
    if (idx === -1) return

    tabs.value.splice(idx, 1)

    if (activeTabId.value === id) {
      // Activate neighbor or null
      if (tabs.value.length === 0) {
        activeTabId.value = null
      } else {
        const nextIdx = Math.min(idx, tabs.value.length - 1)
        activeTabId.value = tabs.value[nextIdx].id
      }
    }
  }

  function closeOtherTabs(id: string) {
    tabs.value = tabs.value.filter((t) => t.id === id)
    activeTabId.value = id
  }

  function closeTabsRight(id: string) {
    const idx = tabs.value.findIndex((t) => t.id === id)
    if (idx === -1) return
    tabs.value = tabs.value.slice(0, idx + 1)
    if (!tabs.value.find((t) => t.id === activeTabId.value)) {
      activeTabId.value = id
    }
  }

  function closeTabsLeft(id: string) {
    const idx = tabs.value.findIndex((t) => t.id === id)
    if (idx === -1) return
    tabs.value = tabs.value.slice(idx)
    if (!tabs.value.find((t) => t.id === activeTabId.value)) {
      activeTabId.value = id
    }
  }

  function closeAllTabs() {
    tabs.value = []
    activeTabId.value = null
  }

  function activateTab(id: string) {
    if (tabs.value.find((t) => t.id === id)) {
      activeTabId.value = id
    }
  }

  function duplicateTab(id: string) {
    const tab = tabs.value.find((t) => t.id === id)
    if (tab) {
      addTab(tab.name, tab.proto, tab.resourceId)
    }
  }

  function moveTabToPanel(id: string, panelIndex: number) {
    const tab = tabs.value.find((t) => t.id === id)
    if (tab) {
      tab.panelIndex = panelIndex
    }
  }

  function swapPanels(tabId1: string, tabId2: string) {
    const t1 = tabs.value.find((t) => t.id === tabId1)
    const t2 = tabs.value.find((t) => t.id === tabId2)
    if (!t1 || !t2) return
    const tmp = t1.panelIndex
    t1.panelIndex = t2.panelIndex
    t2.panelIndex = tmp
  }

  function nextTab() {
    if (tabs.value.length <= 1) return
    const idx = tabs.value.findIndex((t) => t.id === activeTabId.value)
    const nextIdx = (idx + 1) % tabs.value.length
    activeTabId.value = tabs.value[nextIdx].id
  }

  function prevTab() {
    if (tabs.value.length <= 1) return
    const idx = tabs.value.findIndex((t) => t.id === activeTabId.value)
    const prevIdx = (idx - 1 + tabs.value.length) % tabs.value.length
    activeTabId.value = tabs.value[prevIdx].id
  }

  function switchTabByIndex(index: number) {
    if (index >= 0 && index < tabs.value.length) {
      activeTabId.value = tabs.value[index].id
    }
  }

  function reorderTab(fromId: string, toId: string) {
    const allTabs = tabs.value
    const srcIdx = allTabs.findIndex((t) => t.id === fromId)
    const dstIdx = allTabs.findIndex((t) => t.id === toId)
    if (srcIdx === -1 || dstIdx === -1 || srcIdx === dstIdx) return
    const [moved] = allTabs.splice(srcIdx, 1)
    allTabs.splice(dstIdx, 0, moved)
  }

  function disconnectAll() {
    for (const tab of tabs.value) {
      tab.status = 'offline'
    }
  }

  const activePanelIndex = computed(() => activeTab.value?.panelIndex ?? 0)

  return {
    tabs,
    activeTabId,
    activeTab,
    activePanelIndex,
    addTab,
    closeTab,
    closeOtherTabs,
    closeTabsRight,
    closeTabsLeft,
    closeAllTabs,
    activateTab,
    duplicateTab,
    moveTabToPanel,
    swapPanels,
    disconnectAll,
    nextTab,
    prevTab,
    switchTabByIndex,
    reorderTab,
  }
}
