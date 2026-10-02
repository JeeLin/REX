import { describe, it, expect, vi } from 'vitest'
import { ref } from 'vue'
import { useTabs } from '../useTabs'

/**
 * Single pane by default. Pass `allLeaves` for a real pane tree whose setPaneTab
 * mutates the leaves; focusPane is a spy standing in for the layout's single
 * focus write point.
 */
function createTabs(opts?: { allLeaves?: Array<{ id: string; tabId: string | null }> }) {
  const allLeaves = ref(opts?.allLeaves ?? [{ id: 'pane-1', tabId: null as string | null }])
  const activePaneId = ref(allLeaves.value[0]!.id)
  const setPaneTab = vi.fn((paneId: string, tabId: string | null) => {
    const leaf = allLeaves.value.find((l) => l.id === paneId)
    if (leaf) leaf.tabId = tabId
  })
  // Mirrors usePaneLayout.focusPane: the one write point that must also update
  // lastFocusedPaneId, so activateTab's focus path is asserted against it.
  const focusPane = vi.fn((paneId: string) => {
    if (allLeaves.value.find((l) => l.id === paneId)) activePaneId.value = paneId
  })
  const tabs = useTabs({ activePaneId, setPaneTab, allLeaves, focusPane })
  return { ...tabs, activePaneId, allLeaves, setPaneTab, focusPane }
}

/** Two-leaf split workspace sharing the scaffolding of createTabs. */
function createSplitTabs() {
  return createTabs({
    allLeaves: [
      { id: 'pane-1', tabId: null },
      { id: 'pane-2', tabId: null },
    ],
  })
}

describe('useTabs', () => {
  it('opens a resource and binds it to the active pane', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'Server A', protocol: 'ssh' })
    expect(t.tabs.value).toHaveLength(1)
    expect(t.activeTab.value).toBe(t.tabs.value[0]!.id)
    expect(t.setPaneTab).toHaveBeenCalledWith('pane-1', t.tabs.value[0]!.id)
    expect(t.tabs.value[0]!.status).toBe('connecting')
  })

  it('does not duplicate the same resource+protocol', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'Server A', protocol: 'ssh' })
    t.openResource({ id: 'r1', name: 'Server A', protocol: 'ssh' })
    expect(t.tabs.value).toHaveLength(1)
  })

  it('closes a tab and activates a neighbor', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    const [first, second] = t.tabs.value
    t.closeTab(first!.id)
    expect(t.tabs.value).toHaveLength(1)
    expect(t.activeTab.value).toBe(second!.id)
  })

  it('closes all tabs and clears active', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    t.closeAllTabs()
    expect(t.tabs.value).toHaveLength(0)
    expect(t.activeTab.value).toBe('')
  })

  it('closes tabs to the right of a target', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    t.openResource({ id: 'r3', name: 'C', protocol: 'ssh' })
    const second = t.tabs.value[1]!
    t.closeTabsRight(second.id)
    expect(t.tabs.value.map((x) => x.id)).toEqual([t.tabs.value[0]!.id, second.id])
  })

  it('closes tabs to the left of a target', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    t.openResource({ id: 'r3', name: 'C', protocol: 'ssh' })
    const second = t.tabs.value[1]!
    const third = t.tabs.value[2]!
    t.closeTabsLeft(second.id)
    expect(t.tabs.value.map((x) => x.id)).toEqual([second.id, third.id])
  })

  it('duplicates a tab with connecting status', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    const original = t.tabs.value[0]!
    t.duplicateTab(original.id)
    expect(t.tabs.value).toHaveLength(2)
    expect(t.tabs.value[1]!.label).toBe(original.label)
    expect(t.tabs.value[1]!.status).toBe('connecting')
  })

  it('renames a tab', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    const id = t.tabs.value[0]!.id
    t.startRename(id)
    expect(t.findTab(id)?.renaming).toBe(true)
    t.finishRename(id, 'Renamed')
    expect(t.findTab(id)?.label).toBe('Renamed')
    expect(t.findTab(id)?.renaming).toBe(false)
  })

  it('toggles broadcast mode', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    const id = t.tabs.value[0]!.id
    t.toggleBroadcast(id)
    expect(t.findTab(id)?.broadcast).toBe(true)
    t.toggleBroadcast(id)
    expect(t.findTab(id)?.broadcast).toBe(false)
  })

  it('handleTabCtxAction dispatches close/closeOthers etc.', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    const id = t.tabs.value[0]!.id
    // 设置 tabContextMenu.tabId 模拟右键
    t.tabContextMenu.value = { show: true, x: 0, y: 0, tabId: id }
    t.handleTabCtxAction('close')
    expect(t.tabs.value).toHaveLength(1)
  })

  it('reorders tabs via drag-drop', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    const [a, b] = t.tabs.value
    const dt = { effectAllowed: '', dropEffect: '', setData: () => {}, getData: () => '' }
    const dragEvt = new Event('dragstart') as DragEvent
    Object.defineProperty(dragEvt, 'dataTransfer', { value: dt })
    const dropEvt = new Event('drop') as DragEvent
    Object.defineProperty(dropEvt, 'dataTransfer', { value: { getData: () => a!.id } })
    t.onTabDragStart(dragEvt, a!.id)
    t.onTabDrop(dropEvt, b!.id)
    expect(t.tabs.value.map((x) => x.id)).toEqual([b!.id, a!.id])
  })

  it('closeTab keeps last tab active when closing the last one', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    const id = t.tabs.value[0]!.id
    t.closeTab(id)
    expect(t.tabs.value).toHaveLength(0)
    expect(t.activeTab.value).toBe('')
  })

  it('closeTab ignores unknown id', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.closeTab('nope')
    expect(t.tabs.value).toHaveLength(1)
  })

  it('closeOtherTabs keeps only the target', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    t.openResource({ id: 'r3', name: 'C', protocol: 'ssh' })
    const second = t.tabs.value[1]!.id
    t.closeOtherTabs(second)
    expect(t.tabs.value.map((x) => x.id)).toEqual([second])
    expect(t.activeTab.value).toBe(second)
  })

  it('closeTabsRight is a no-op when target not found', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.closeTabsRight('nope')
    expect(t.tabs.value).toHaveLength(1)
  })

  it('closeTabsRight activates last tab when active was removed', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    t.openResource({ id: 'r3', name: 'C', protocol: 'ssh' })
    const first = t.tabs.value[0]!.id
    t.closeTabsRight(first)
    expect(t.tabs.value).toHaveLength(1)
    expect(t.activeTab.value).toBe(first)
  })

  it('closeTabsLeft is a no-op when target is first', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    const first = t.tabs.value[0]!.id
    t.closeTabsLeft(first)
    expect(t.tabs.value).toHaveLength(2)
  })

  it('closeTabsLeft activates first tab when active was removed', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    t.openResource({ id: 'r3', name: 'C', protocol: 'ssh' })
    const third = t.tabs.value[2]!.id
    t.closeTabsLeft(third)
    expect(t.tabs.value).toHaveLength(1)
    expect(t.activeTab.value).toBe(third)
  })

  it('handleTabCtxAction dispatches all actions', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    t.openResource({ id: 'r3', name: 'C', protocol: 'ssh' })
    const id = t.tabs.value[0]!.id
    const dispatch = (a: string) => {
      t.tabContextMenu.value = { show: true, x: 0, y: 0, tabId: id }
      t.handleTabCtxAction(a)
    }
    // 打开右键菜单
    const e = new Event('contextmenu') as MouseEvent
    t.onTabContextMenu(e, id)
    expect(t.tabContextMenu.value.show).toBe(true)
    expect(t.tabContextMenu.value.tabId).toBe(id)

    t.startRename(id); t.finishRename(id, 'Renamed'); dispatch('duplicate')
    dispatch('closeLeft'); dispatch('closeRight')
    dispatch('closeOthers'); dispatch('closeAll')
    expect(t.tabs.value).toHaveLength(0)

    // 空 tabId 时不动作
    t.tabContextMenu.value = { show: true, x: 0, y: 0, tabId: '' }
    t.handleTabCtxAction('close')
    expect(t.tabs.value).toHaveLength(0)
  })

  it('handleTabCtxAction toggles pin on and off', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    const second = t.tabs.value[1]!.id
    const dispatchPin = (tabId: string) => {
      t.tabContextMenu.value = { show: true, x: 0, y: 0, tabId }
      t.handleTabCtxAction('pin')
    }

    dispatchPin(second)
    expect(t.findTab(second)?.pinned).toBe(true)
    expect(t.tabContextMenu.value.show).toBe(false)

    // 二次 pin 取消置顶，且不动其它 tab
    dispatchPin(second)
    expect(t.findTab(second)?.pinned).toBe(false)
    expect(t.tabs.value[0]!.pinned).toBeUndefined()
  })

  it('duplicateTab opens a connected copy', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    const id = t.tabs.value[0]!.id
    t.duplicateTab(id)
    expect(t.tabs.value).toHaveLength(2)
    const copy = t.tabs.value[1]!
    expect(copy.id).not.toBe(id)
    expect(copy.status).toBe('connecting')
    expect(t.tabContextMenu.value.show).toBe(false)
  })

  it('duplicateTab is a no-op for unknown id', () => {
    const t = createTabs()
    t.duplicateTab('nope')
    expect(t.tabs.value).toHaveLength(0)
  })

  it('toggleBroadcast and onTabStatusChange update state', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    const id = t.tabs.value[0]!.id
    t.toggleBroadcast(id)
    expect(t.findTab(id)?.broadcast).toBe(true)
    t.onTabStatusChange(id, 'connected')
    expect(t.findTab(id)?.status).toBe('connected')
  })

  it('setTabColor sets color via context menu tabId', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    const id = t.tabs.value[0]!.id
    t.tabContextMenu.value = { show: true, x: 0, y: 0, tabId: id }
    t.setTabColor('#fff')
    expect(t.findTab(id)?.color).toBe('#fff')
    expect(t.tabContextMenu.value.show).toBe(false)
  })

  it('drag handlers set and clear dragTabId', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    const [a, b] = t.tabs.value
    const dt = { effectAllowed: '', dropEffect: '', setData: () => {} }
    const dragEvt = new Event('dragstart') as DragEvent
    Object.defineProperty(dragEvt, 'dataTransfer', { value: dt })
    t.onTabDragStart(dragEvt, a!.id)
    expect(t.dragTabId.value).toBe(a!.id)

    // 拖回自身：不交换（自比较提前返回）
    const sameEvt = new Event('drop') as DragEvent
    Object.defineProperty(sameEvt, 'dataTransfer', { value: { getData: () => a!.id } })
    t.onTabDrop(sameEvt, a!.id)
    expect(t.tabs.value.map((x) => x.id)).toEqual([a!.id, b!.id])
    // 自比较分支不清除 dragTabId
    expect(t.dragTabId.value).toBe(a!.id)

    t.onTabDragEnd()
    expect(t.dragTabId.value).toBe('')
  })

  it('onTabDrop swaps two distinct tabs', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    const [a, b] = t.tabs.value
    const dragEvt = new Event('dragstart') as DragEvent
    Object.defineProperty(dragEvt, 'dataTransfer', { value: { effectAllowed: '', dropEffect: '', setData: () => {} } })
    t.onTabDragStart(dragEvt, a!.id)
    const dropEvt = new Event('drop') as DragEvent
    Object.defineProperty(dropEvt, 'dataTransfer', { value: { getData: () => a!.id } })
    t.onTabDrop(dropEvt, b!.id)
    expect(t.tabs.value.map((x) => x.id)).toEqual([b!.id, a!.id])

    t.onTabDragEnd()
    expect(t.dragTabId.value).toBe('')
  })

  it('onTabDragOver sets dropEffect and prevents default', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    const e = new Event('dragover') as DragEvent
    Object.defineProperty(e, 'dataTransfer', { value: { effectAllowed: '', dropEffect: '', setData: () => {} } })
    const prevent = vi.spyOn(e, 'preventDefault')
    t.onTabDragOver(e, 'x')
    expect(prevent).toHaveBeenCalled()
  })

  it('formatConnection returns uppercased protocol', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'mysql' })
    expect(t.formatConnection(t.tabs.value[0]!)).toBe('MYSQL')
  })

  it('exposes tabColors palette and activeTabInfo', () => {
    const t = createTabs()
    expect(t.tabColors.length).toBeGreaterThan(0)
    expect(t.activeTabInfo.value).toBeNull()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    expect(t.activeTabInfo.value?.label).toBe('A')
  })

  // ===== activateTab ↔ pane focus =====

  it('activateTab focuses the pane already holding the tab instead of moving it', () => {
    const t = createSplitTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    const a = t.tabs.value[0]!.id
    expect(t.allLeaves.value.map((l) => l.tabId)).toEqual([a, null])
    expect(t.activePaneId.value).toBe('pane-1')

    t.activePaneId.value = 'pane-2'
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    const b = t.tabs.value[1]!.id
    expect(t.allLeaves.value.map((l) => l.tabId)).toEqual([a, b])

    // Back to A: focus pane-1 via the unified write point, do not drain A into pane-2.
    t.focusPane.mockClear()
    t.activateTab(a)
    expect(t.activeTab.value).toBe(a)
    expect(t.focusPane).toHaveBeenCalledWith('pane-1')
    expect(t.activePaneId.value).toBe('pane-1')
    expect(t.allLeaves.value.map((l) => l.tabId)).toEqual([a, b])
  })

  it('activateTab binds an unmounted tab to the active pane', () => {
    const t = createSplitTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.activePaneId.value = 'pane-2'
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    const a = t.tabs.value[0]!.id

    t.tabs.value.push({ id: 'c', label: 'C', protocol: 'ssh', status: 'connecting' })
    t.activateTab('c')

    expect(t.activeTab.value).toBe('c')
    expect(t.activePaneId.value).toBe('pane-2')
    expect(t.allLeaves.value[1]!.tabId).toBe('c')
    // One tab hangs on at most one pane.
    expect(t.allLeaves.value.filter((l) => l.tabId === 'c')).toHaveLength(1)
    expect(t.allLeaves.value[0]!.tabId).toBe(a)
  })

  // ===== tab history =====

  it('goBack lands on the nearest earlier tab in a single press after a history entry was closed', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    t.openResource({ id: 'r3', name: 'C', protocol: 'ssh' })
    const [a, b, c] = t.tabs.value.map((x) => x.id)
    t.pushHistory(a!)
    t.pushHistory(b!)
    t.pushHistory(c!)
    expect(t.tabHistory.value).toEqual([a, b, c])

    t.closeTab(b!)
    // The closed id stays in the list; goBack/goForward skip it.
    expect(t.tabHistory.value).toEqual([a, b, c])
    expect(t.tabHistoryIndex.value).toBe(2) // still pointing at the current tab (C)

    t.goBack()
    expect(t.activeTab.value).toBe(a)
    expect(t.tabHistoryIndex.value).toBe(0)

    // The forward branch survives: the stale B entry is skipped, not consumed.
    t.goForward()
    expect(t.activeTab.value).toBe(c)
    expect(t.tabHistoryIndex.value).toBe(2)
  })

  it('goBack/goForward skip history entries whose tab no longer exists', () => {
    const t = createTabs()
    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    const [a] = t.tabs.value.map((x) => x.id)
    t.pushHistory(a!)
    t.pushHistory('ghost') // stale id left behind by a bulk close
    t.pushHistory(t.tabs.value[1]!.id)

    t.goBack()
    expect(t.activeTab.value).toBe(a)
    expect(t.tabHistoryIndex.value).toBe(0)

    t.goForward()
    expect(t.activeTab.value).toBe(t.tabs.value[1]!.id)
    expect(t.tabHistoryIndex.value).toBe(2)
  })

  it('goBack/goForward are safe no-ops on an empty or fully closed history', () => {
    const t = createTabs()
    expect(() => { t.goBack(); t.goForward() }).not.toThrow()
    expect(t.activeTab.value).toBe('')
    expect(t.tabHistoryIndex.value).toBe(-1)

    t.openResource({ id: 'r1', name: 'A', protocol: 'ssh' })
    t.openResource({ id: 'r2', name: 'B', protocol: 'ssh' })
    t.pushHistory(t.tabs.value[0]!.id)
    t.pushHistory(t.tabs.value[1]!.id)
    t.closeTab(t.tabs.value[0]!.id)
    t.closeTab(t.tabs.value[0]!.id)

    expect(t.tabs.value).toHaveLength(0)
    // Stale ids are left in place and skipped on read, not pruned on close.
    expect(t.tabHistory.value).toHaveLength(2)

    expect(() => { t.goBack(); t.goForward() }).not.toThrow()
    expect(t.activeTab.value).toBe('')
  })
})
