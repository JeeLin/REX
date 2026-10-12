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

      // 先恢复 pane layout —— 叶数必须取自恢复后的布局，而不是默认单格。
      // 顺序反了会先按 leafCount=1 把 tabs 截断到只剩第一个（v0.95.0 子任务 4）。
      if (state.paneLayout) {
        paneLayoutDeserialize(state.paneLayout)
      }

      // 再恢复 tabs，按 tab id 去重（而非 (resourceId, protocol)）：
      // 同资源的多个实例（分栏各开一个）是合法状态，按资源去重会把它们折叠成一个。
      //
      // 别名回写：若本次会话已为某资源开了一个「pending」tab（openResource 先于
      // restore 执行，id 是新的），而快照里存的是同一资源的旧 id，则旧 tab 不再 push ——
      // 否则重启后同一资源会出现「新+旧」两个。但只有 1:1 对应时这么做：快照里同一
      // 资源有多个实例（合法的分栏多开）时，不能拿其中一个去顶掉本次会话的 tab。
      const leafCount = allLeaves.value.length || 1
      const existingIds = new Set(tabs.value.map((x) => x.id))
      const restoredIds: string[] = []

      // 本次会话已开的 tab，按 (resourceId, protocol) 索引，用于别名回写匹配。
      const liveByKey = new Map<string, string[]>()
      for (const x of tabs.value) {
        if (!x.resourceId) continue
        const key = `${x.resourceId}::${x.protocol}`
        const arr = liveByKey.get(key)
        if (arr) arr.push(x.id)
        else liveByKey.set(key, [x.id])
      }
      // 快照里每个资源出现几次：>1 说明该资源本来就是多实例，不能做 1:1 别名回写。
      const persistedCountByKey = new Map<string, number>()
      for (const t of state.tabs) {
        if (!t.resourceId) continue
        const key = `${t.resourceId}::${t.protocol}`
        persistedCountByKey.set(key, (persistedCountByKey.get(key) ?? 0) + 1)
      }

      for (const t of state.tabs.slice(0, leafCount)) {
        if (existingIds.has(t.id)) {
          // 同 id 已在（本次会话刚开的 tab）：不重复 push，仅记录为已绑定来源。
          restoredIds.push(t.id)
          continue
        }

        // 别名回写判定：快照中该资源恰好只有 1 个实例，且本次会话已有同一资源的
        // pending tab → 认作同一个 tab 的旧 id，跳过 push（由 pending tab 吸收）。
        if (t.resourceId) {
          const key = `${t.resourceId}::${t.protocol}`
          const live = liveByKey.get(key) ?? []
          if (live.length === 1 && persistedCountByKey.get(key) === 1) {
            restoredIds.push(live[0]!) // 绑定时用本次会话的活 tab id
            continue
          }
        }

        tabs.value.push({
          id: t.id,
          label: t.label,
          protocol: t.protocol as 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3' | 'sql',
          resourceId: t.resourceId,
          environmentId: t.environmentId,
          status: 'disconnected',
        })
        existingIds.add(t.id)
        restoredIds.push(t.id)
      }

      // Restore active tab
      if (state.activeTabId && tabs.value.some(t => t.id === state.activeTabId)) {
        activeTab.value = state.activeTabId
      } else if (tabs.value.length > 0) {
        activeTab.value = tabs.value[0]!.id
      }

      // 恢复 tab→pane 绑定：按 tab 在快照中的顺序依次占用空闲叶，
      // 保持「一 tab 一叶」；绑定前先清掉指向不存在 tab 的悬空绑定，
      // 否则 PaneLeaf 会渲染一个查无此 tab 的空白占位。
      const leaves = allLeaves.value
      const validIds = new Set(tabs.value.map((x) => x.id))
      for (const leaf of leaves) {
        if (leaf.tabId && !validIds.has(leaf.tabId)) setPaneTab(leaf.id, null)
      }
      // restoredIds 里存的已是「实际 tab id」（别名回写时是本次会话的活 id，
      // 不是快照里的旧 id），所以这里直接用 restoredIds 的顺序依次占空闲叶。
      for (const tabId of restoredIds) {
        for (const leaf of leaves) {
          if (!leaf.tabId) {
            setPaneTab(leaf.id, tabId)
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
  // allLeaves 也要 watch：split / closePane 只改布局、不动 tabs 与 activeTab，
  // 不把它纳入依赖的话，纯布局操作不会落盘，刷新后布局回退成单格（v0.95.0 子任务 4）。
  watch([tabs, activeTab, allLeaves], save, { deep: true })

  return { restore, save }
}
