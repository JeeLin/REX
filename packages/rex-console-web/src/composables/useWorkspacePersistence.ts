import { watch, type Ref } from 'vue'
import type { Tab } from '@/features/workspace/useTabs'
import { useTabs } from '@/features/workspace/useTabs'

const STORAGE_KEY = 'rex-workspace-state'
const EXPIRY_MS = 24 * 60 * 60 * 1000 // 24 hours

interface WorkspaceState {
  version: number
  tabs: SerializedTab[]
  activeTabId: string | null
  layout: string
  timestamp: number
}

interface SerializedTab {
  name: string
  proto: string
  resourceId: string
  panelIndex: number
}

/**
 * Auto-persist workspace state to localStorage and restore on init.
 * Call once in Workspace.vue setup.
 */
export function useWorkspacePersistence(currentLayout: Ref<string>) {
  const { tabs, activeTabId, addTab } = useTabs()

  function restore(): boolean {
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      if (!raw) return false

      const state: WorkspaceState = JSON.parse(raw)
      if (!state.tabs || !Array.isArray(state.tabs)) return false

      // Check expiry
      if (Date.now() - state.timestamp > EXPIRY_MS) {
        localStorage.removeItem(STORAGE_KEY)
        return false
      }

      // Restore tabs
      for (const t of state.tabs) {
        addTab(t.name, t.proto as Tab['proto'], t.resourceId, false)
      }

      // Restore active tab
      if (state.activeTabId && tabs.value.some((t) => t.id === state.activeTabId)) {
        activeTabId.value = state.activeTabId
      } else if (tabs.value.length > 0) {
        activeTabId.value = tabs.value[0]!.id
      }

      // Restore panel positions
      for (let i = 0; i < state.tabs.length; i++) {
        const saved = state.tabs[i]
        const tab = tabs.value[i]
        if (saved && tab && saved.panelIndex !== undefined) {
          tab.panelIndex = saved.panelIndex
        }
      }

      // Restore layout
      if (state.layout) {
        currentLayout.value = state.layout
      }

      return tabs.value.length > 0
    } catch {
      return false
    }
  }

  function save() {
    try {
      const state: WorkspaceState = {
        version: 1,
        tabs: tabs.value.map((t) => ({
          name: t.name,
          proto: t.proto,
          resourceId: t.resourceId,
          panelIndex: t.panelIndex,
        })),
        activeTabId: activeTabId.value,
        layout: currentLayout.value,
        timestamp: Date.now(),
      }
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state))
    } catch {
      // Ignore storage errors
    }
  }

  // Auto-save on state changes
  watch([tabs, activeTabId, currentLayout], save, { deep: true })

  return { restore, save }
}
