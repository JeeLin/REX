import { describe, it, expect, vi } from 'vitest'
import { ref } from 'vue'
import { useTabs } from '../useTabs'

function createTabs() {
  const activePaneId = ref('pane-1')
  const setPaneTab = vi.fn()
  const tabs = useTabs({ activePaneId, setPaneTab })
  return { ...tabs, activePaneId, setPaneTab }
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
})
