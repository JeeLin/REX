import { watch, type Ref } from 'vue'

const STORAGE_KEY = 'rex-workspace-state'
const EXPIRY_MS = 24 * 60 * 60 * 1000 // 24 hours

interface WorkspaceState {
  version: number
  tabs: SerializedTab[]
  activeTabId: string | null
  paneLayout: string  // JSON-serialized pane tree
  timestamp: number
}

interface SerializedTab {
  id: string
  label: string
  protocol: string
  resourceId?: string
  environmentId?: string
  status: string
}

export function useWorkspacePersistence(opts: {
  tabs: Ref<Array<{ id: string; label: string; protocol: string; resourceId?: string; environmentId?: string; status: string }>>
  activeTab: Ref<string>
  paneLayoutSerialize: () => string
  paneLayoutDeserialize: (data: string) => void
  allLeaves: Ref<Array<{ id: string; tabId: string | null }>>
  setPaneTab: (paneId: string, tabId: string | null) => void
}) {
  const { tabs, activeTab, paneLayoutSerialize, paneLayoutDeserialize, allLeaves, setPaneTab } = opts

  function restore(): boolean {
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      if (!raw) return false

      const state: WorkspaceState = JSON.parse(raw)
      if (!state.tabs || !Array.isArray(state.tabs)) return false

      // Check expiry (24h)
      if (Date.now() - state.timestamp > EXPIRY_MS) {
        localStorage.removeItem(STORAGE_KEY)
        return false
      }

      // Restore tabs
      for (const t of state.tabs) {
        tabs.value.push({
          id: t.id,
          label: t.label,
          protocol: t.protocol as 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3',
          resourceId: t.resourceId,
          environmentId: t.environmentId,
          status: 'disconnected',
        })
      }

      // Restore active tab
      if (state.activeTabId && tabs.value.some(t => t.id === state.activeTabId)) {
        activeTab.value = state.activeTabId
      } else if (tabs.value.length > 0) {
        activeTab.value = tabs.value[0]!.id
      }

      // Restore pane layout
      if (state.paneLayout) {
        paneLayoutDeserialize(state.paneLayout)
      }

      // Restore tab-to-pane bindings
      if (state.tabs) {
        for (const t of state.tabs) {
          const leaves = allLeaves.value
          // Find first unbound leaf and bind it
          for (const leaf of leaves) {
            if (!leaf.tabId) {
              setPaneTab(leaf.id, t.id)
              break
            }
          }
        }
      }

      return tabs.value.length > 0
    } catch {
      return false
    }
  }

  function save() {
    try {
      const state: WorkspaceState = {
        version: 2,
        tabs: tabs.value.map(t => ({
          id: t.id,
          label: t.label,
          protocol: t.protocol,
          resourceId: t.resourceId,
          environmentId: t.environmentId,
          status: t.status,
        })),
        activeTabId: activeTab.value,
        paneLayout: paneLayoutSerialize(),
        timestamp: Date.now(),
      }
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state))
    } catch {
      // Ignore storage errors
    }
  }

  // Auto-save on state changes
  watch([tabs, activeTab], save, { deep: true })

  return { restore, save }
}
