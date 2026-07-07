import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { useTabs } from '../useTabs'

describe('useTabs', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    const { closeAllTabs } = useTabs()
    closeAllTabs()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('starts with no tabs', () => {
    const { tabs, activeTabId } = useTabs()
    expect(tabs.value).toHaveLength(0)
    expect(activeTabId.value).toBeNull()
  })

  it('adds a tab and maps protocol to component', () => {
    const { tabs, activeTabId, addTab } = useTabs()
    const id = addTab('My Server', 'ssh', 'res-1')
    expect(id).toBeTruthy()
    expect(tabs.value).toHaveLength(1)
    expect(tabs.value[0]!.name).toBe('My Server')
    expect(tabs.value[0]!.component).toBe('terminal')
    expect(activeTabId.value).toBe(id)
  })

  it('deduplicates tabs with same resourceId', () => {
    const { tabs, addTab } = useTabs()
    const id1 = addTab('Server', 'ssh', 'res-dedup')
    const id2 = addTab('Server', 'ssh', 'res-dedup')
    expect(id1).toBe(id2)
    expect(tabs.value).toHaveLength(1)
  })

  it('sets status to online after delay', () => {
    const { tabs, addTab } = useTabs()
    addTab('Server', 'ssh', `res-${Date.now()}`)
    expect(tabs.value[0]!.status).toBe('connecting')
    vi.advanceTimersByTime(900)
    expect(tabs.value[0]!.status).toBe('online')
  })

  it('closes a tab and activates neighbor', () => {
    const { tabs, activeTabId, addTab, closeTab } = useTabs()
    addTab('S1', 'ssh', `r-${Date.now()}-a`)
    addTab('S2', 'mysql', `r-${Date.now()}-b`)
    const firstId = tabs.value[0]!.id
    closeTab(firstId)
    expect(tabs.value).toHaveLength(1)
    expect(activeTabId.value).toBe(tabs.value[0]!.id)
  })

  it('closes all tabs', () => {
    const { tabs, addTab, closeAllTabs } = useTabs()
    addTab('S1', 'ssh', `r-${Date.now()}-x`)
    addTab('S2', 'mysql', `r-${Date.now()}-y`)
    closeAllTabs()
    expect(tabs.value).toHaveLength(0)
  })

  it('closeOtherTabs keeps only the specified tab', () => {
    const { tabs, addTab, closeOtherTabs } = useTabs()
    addTab('S1', 'ssh', `r-${Date.now()}-o1`)
    addTab('S2', 'mysql', `r-${Date.now()}-o2`)
    addTab('S3', 'redis', `r-${Date.now()}-o3`)
    closeOtherTabs(tabs.value[1]!.id)
    expect(tabs.value).toHaveLength(1)
    expect(tabs.value[0]!.name).toBe('S2')
  })

  it('closeTabsRight removes tabs after the given tab', () => {
    const { tabs, addTab, closeTabsRight } = useTabs()
    addTab('S1', 'ssh', `r-${Date.now()}-r1`)
    addTab('S2', 'mysql', `r-${Date.now()}-r2`)
    addTab('S3', 'redis', `r-${Date.now()}-r3`)
    closeTabsRight(tabs.value[0]!.id)
    expect(tabs.value).toHaveLength(1)
  })

  it('closeTabsLeft removes tabs before the given tab', () => {
    const { tabs, addTab, closeTabsLeft } = useTabs()
    addTab('S1', 'ssh', `r-${Date.now()}-l1`)
    addTab('S2', 'mysql', `r-${Date.now()}-l2`)
    addTab('S3', 'redis', `r-${Date.now()}-l3`)
    closeTabsLeft(tabs.value[2]!.id)
    expect(tabs.value).toHaveLength(1)
    expect(tabs.value[0]!.name).toBe('S3')
  })

  it('activateTab changes active tab', () => {
    const { tabs, activeTabId, addTab, activateTab } = useTabs()
    addTab('S1', 'ssh', `r-${Date.now()}-a1`)
    addTab('S2', 'mysql', `r-${Date.now()}-a2`)
    activateTab(tabs.value[0]!.id)
    expect(activeTabId.value).toBe(tabs.value[0]!.id)
  })

  it('duplicateTab creates a copy', () => {
    const { tabs, addTab, duplicateTab } = useTabs()
    const id = addTab('S1', 'ssh', 'res-unique-dup')
    // duplicateTab now creates a new tab even with the same resourceId
    if (id) duplicateTab(id)
    expect(tabs.value).toHaveLength(2)
  })

  it('reorderTab changes tab order', () => {
    const { tabs, addTab, reorderTab } = useTabs()
    addTab('S1', 'ssh', `r-${Date.now()}-re1`)
    addTab('S2', 'mysql', `r-${Date.now()}-re2`)
    addTab('S3', 'redis', `r-${Date.now()}-re3`)
    reorderTab(tabs.value[0]!.id, tabs.value[2]!.id)
    expect(tabs.value[0]!.name).toBe('S2')
    expect(tabs.value[2]!.name).toBe('S1')
  })

  it('nextTab/prevTab cycle through tabs', () => {
    const { tabs, activeTabId, addTab, activateTab, nextTab, prevTab } = useTabs()
    addTab('S1', 'ssh', `r-${Date.now()}-n1`)
    addTab('S2', 'mysql', `r-${Date.now()}-n2`)
    addTab('S3', 'redis', `r-${Date.now()}-n3`)
    activateTab(tabs.value[0]!.id)

    nextTab()
    expect(activeTabId.value).toBe(tabs.value[1]!.id)
    nextTab()
    expect(activeTabId.value).toBe(tabs.value[2]!.id)
    nextTab() // wraps
    expect(activeTabId.value).toBe(tabs.value[0]!.id)
    prevTab()
    expect(activeTabId.value).toBe(tabs.value[2]!.id)
  })

  it('switchTabByIndex activates the correct tab', () => {
    const { tabs, activeTabId, addTab, switchTabByIndex } = useTabs()
    addTab('S1', 'ssh', `r-${Date.now()}-si1`)
    addTab('S2', 'mysql', `r-${Date.now()}-si2`)
    switchTabByIndex(1)
    expect(activeTabId.value).toBe(tabs.value[1]!.id)
    switchTabByIndex(5) // out of range, no-op
    expect(activeTabId.value).toBe(tabs.value[1]!.id)
  })

  it('disconnectAll sets all tabs offline', () => {
    const { tabs, addTab, disconnectAll } = useTabs()
    addTab('S1', 'ssh', `r-${Date.now()}-d1`)
    addTab('S2', 'mysql', `r-${Date.now()}-d2`)
    disconnectAll()
    expect(tabs.value.every(t => t.status === 'offline')).toBe(true)
  })
})
