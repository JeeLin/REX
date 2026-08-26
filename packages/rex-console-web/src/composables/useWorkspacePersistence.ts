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

      // Restore tabs — 持久化的 tabs 数量可能超过当前布局 leaves 数量。
      // 只恢复前 N 个（N = leaves 数量），多余的 tab 无 pane 可绑定，直接丢弃，
      // 避免 tabs>leaves 不对齐导致多余 tab 无法打开。
      // 去重：按 (resourceId, protocol) 判重，与 openResource 的判重键一致，
      // 防止「restore 与打开资源的即时动作叠加」产生同一资源的重复 tab（重启后出现旧+新两个）。
      const leafCount = allLeaves.value.length || 1
      const tabsToRestore = state.tabs.slice(0, leafCount)
      const restoredIds: string[] = []
      for (const t of tabsToRestore) {
        const already = tabs.value.some(
          x => x.resourceId === t.resourceId && x.protocol === t.protocol,
        )
        if (already) continue
        tabs.value.push({
          id: t.id,
          label: t.label,
          protocol: t.protocol as 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3' | 'sql',
          resourceId: t.resourceId,
          environmentId: t.environmentId,
          status: 'disconnected',
        })
        restoredIds.push(t.id)
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

      // Restore tab-to-pane bindings（仅绑定实际恢复的 tab，忽略被去重的重复项）
      for (const t of state.tabs) {
        if (!restoredIds.includes(t.id)) continue
        const leaves = allLeaves.value
        // Find first unbound leaf and bind it
        for (const leaf of leaves) {
          if (!leaf.tabId) {
            setPaneTab(leaf.id, t.id)
            break
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
