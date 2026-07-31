import { watch, type Ref } from 'vue'

const STORAGE_KEY = 'rex-workspace-state'
const EXPIRY_MS = 24 * 60 * 60 * 1000 // 24 hours

interface WorkspaceState {
  version: number
  tabs: SerializedTab[]
  activeTabId: string | null
  splitCount: number
  timestamp: number
}

interface SerializedTab {
  id: string
  label: string
  protocol: string
  resourceId?: string
  environmentId?: string
}

/**
 * Persist workspace tabs to localStorage and restore on page reload.
 * Call once in WorkspacePage setup.
 */
export function useWorkspacePersistence(opts: {
  tabs: Ref<Array<{ id: string; label: string; protocol: string; resourceId?: string; environmentId?: string }>>
  activeTab: Ref<string>
  splitCount: Ref<number>
}) {
  const { tabs, activeTab, splitCount } = opts

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

      // Restore split count
      if (state.splitCount > 0) {
        splitCount.value = state.splitCount
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
        tabs: tabs.value.map(t => ({
          id: t.id,
          label: t.label,
          protocol: t.protocol,
          resourceId: t.resourceId,
          environmentId: t.environmentId,
        })),
        activeTabId: activeTab.value,
        splitCount: splitCount.value,
        timestamp: Date.now(),
      }
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state))
    } catch {
      // Ignore storage errors
    }
  }

  // Auto-save on state changes
  watch([tabs, activeTab, splitCount], save, { deep: true })

  return { restore, save }
}
