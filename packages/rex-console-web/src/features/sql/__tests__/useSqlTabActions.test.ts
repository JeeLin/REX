import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useSqlTabActions } from '../useSqlTabActions'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (key === 'sql.tabTitle') return `Query ${params?.n}`
      if (key === 'sql.executeFailed') return 'Execute failed'
      return key
    },
  }),
}))

vi.mock('@/api/sql', () => ({
  executeSql: vi.fn(),
}))

describe('useSqlTabActions', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('initializes with empty tabs', () => {
    const { tabs, activeTabId } = useSqlTabActions('res-1')
    expect(tabs.value).toHaveLength(0)
    expect(activeTabId.value).toBe('')
  })

  it('adds a new tab', () => {
    const { tabs, activeTabId, addTab } = useSqlTabActions('res-1')
    addTab()
    expect(tabs.value).toHaveLength(1)
    expect(activeTabId.value).toBe(tabs.value[0]!.id)
    expect(tabs.value[0]!.sql).toBe('')
    expect(tabs.value[0]!.queryId).toBeNull()
  })

  it('adds multiple tabs', () => {
    const { tabs, addTab } = useSqlTabActions('res-1')
    addTab()
    addTab()
    expect(tabs.value).toHaveLength(2)
  })

  it('closes a tab and selects adjacent', () => {
    const { tabs, activeTabId, addTab, closeTab } = useSqlTabActions('res-1')
    addTab()
    addTab()
    const firstId = tabs.value[0]!.id
    closeTab(firstId)
    expect(tabs.value).toHaveLength(1)
    expect(activeTabId.value).not.toBe(firstId)
  })

  it('adds a new tab when closing the last tab', () => {
    const { tabs, addTab, closeTab } = useSqlTabActions('res-1')
    addTab()
    const id = tabs.value[0]!.id
    closeTab(id)
    expect(tabs.value).toHaveLength(1)
    expect(tabs.value[0]!.sql).toBe('')
  })

  it('closes other tabs', () => {
    const { tabs, addTab, closeOthers } = useSqlTabActions('res-1')
    addTab()
    addTab()
    addTab()
    const firstId = tabs.value[0]!.id
    closeOthers(firstId)
    expect(tabs.value).toHaveLength(1)
    expect(tabs.value[0]!.id).toBe(firstId)
  })

  it('closes all tabs and creates a new one', () => {
    const { tabs, addTab, closeAll } = useSqlTabActions('res-1')
    addTab()
    addTab()
    closeAll()
    expect(tabs.value).toHaveLength(1)
    expect(tabs.value[0]!.sql).toBe('')
  })

  it('closes only saved tabs (queryId not null)', () => {
    const { tabs, addTab, closeSaved, markSaved } = useSqlTabActions('res-1')
    addTab()
    addTab()
    addTab()
    markSaved(tabs.value[1]!.id, 'q-1')
    markSaved(tabs.value[2]!.id, 'q-2')
    closeSaved()
    expect(tabs.value).toHaveLength(1)
    expect(tabs.value[0]!.queryId).toBeNull()
  })

  it('renames a tab', () => {
    const { tabs, addTab, renameTab } = useSqlTabActions('res-1')
    addTab()
    const id = tabs.value[0]!.id
    renameTab(id, 'My Query')
    expect(tabs.value[0]!.title).toBe('My Query')
  })

  it('gets tab SQL', () => {
    const { tabs, addTab, getTabSql } = useSqlTabActions('res-1')
    addTab()
    const id = tabs.value[0]!.id
    tabs.value[0]!.sql = 'SELECT 1'
    expect(getTabSql(id)).toBe('SELECT 1')
  })

  it('clears editor for active tab', () => {
    const { tabs, addTab, clearEditor } = useSqlTabActions('res-1')
    addTab()
    tabs.value[0]!.sql = 'SELECT 1'
    clearEditor()
    expect(tabs.value[0]!.sql).toBe('')
  })

  it('opens a query file in a new tab', () => {
    const { tabs, activeTabId, openQueryFile } = useSqlTabActions('res-1')
    openQueryFile('q-1', 'My Query', 'SELECT * FROM users')
    expect(tabs.value).toHaveLength(1)
    expect(tabs.value[0]!.queryId).toBe('q-1')
    expect(tabs.value[0]!.sql).toBe('SELECT * FROM users')
    expect(activeTabId.value).toBe(tabs.value[0]!.id)
  })

  it('does not open duplicate query file tab', () => {
    const { tabs, openQueryFile } = useSqlTabActions('res-1')
    openQueryFile('q-1', 'My Query', 'SELECT 1')
    openQueryFile('q-1', 'My Query', 'SELECT 1')
    expect(tabs.value).toHaveLength(1)
  })

  it('opens SQL tab with content', () => {
    const { tabs, activeTabId, openSqlTab } = useSqlTabActions('res-1')
    openSqlTab('History Item', 'DELETE FROM t')
    expect(tabs.value).toHaveLength(1)
    expect(tabs.value[0]!.title).toBe('History Item')
    expect(tabs.value[0]!.queryId).toBeNull()
    expect(activeTabId.value).toBe(tabs.value[0]!.id)
  })

  it('marks tab as saved with queryId', () => {
    const { tabs, addTab, markSaved, getQueryId } = useSqlTabActions('res-1')
    addTab()
    const id = tabs.value[0]!.id
    markSaved(id, 'q-42')
    expect(getQueryId(id)).toBe('q-42')
  })

  it('generates tabList with subtitles', () => {
    const { tabs, addTab, tabList } = useSqlTabActions('res-1')
    addTab()
    tabs.value[0]!.sql = '  SELECT * FROM users\nWHERE id = 1'
    expect(tabList.value[0]!.subtitle).toBe('SELECT * FROM users')
  })

  it('tabList subtitle is undefined for empty SQL', () => {
    const { addTab, tabList } = useSqlTabActions('res-1')
    addTab()
    expect(tabList.value[0]!.subtitle).toBeUndefined()
  })

  it('truncates long subtitle to 30 chars', () => {
    const { tabs, addTab, tabList } = useSqlTabActions('res-1')
    addTab()
    tabs.value[0]!.sql = 'SELECT * FROM very_long_table_name WHERE condition'
    expect(tabList.value[0]!.subtitle!.length).toBeLessThanOrEqual(30)
  })

  it('does not execute empty SQL', async () => {
    const { execute } = useSqlTabActions('res-1')
    await execute('   ')
    // Should not throw, just return early
  })
})
