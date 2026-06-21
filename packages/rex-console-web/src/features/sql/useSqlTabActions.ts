import { ref, computed } from 'vue'
import { executeSql } from '@/api/sql'
import type { SqlResult } from '@/api/sql'

export interface QueryTab {
  id: string
  title: string
  sql: string
  result: SqlResult | null
}

export function useSqlTabActions(resourceId: string, onError?: (msg: string) => void) {
  const tabs = ref<QueryTab[]>([])
  const activeTabId = ref('')
  const executing = ref(false)
  let tabCounter = 0

  const tabList = computed(() =>
    tabs.value.map((t) => ({ id: t.id, title: t.title })),
  )

  const activeTab = computed(() => {
    const tab = tabs.value.find((t) => t.id === activeTabId.value)
    return tab ?? tabs.value[0] ?? { id: '', title: '', sql: '', result: null }
  })

  function addTab() {
    tabCounter++
    const id = `sql-tab-${Date.now()}-${tabCounter}`
    tabs.value.push({ id, title: `查询 ${tabCounter}`, sql: '', result: null })
    activeTabId.value = id
  }

  function closeTab(id: string) {
    const idx = tabs.value.findIndex((t) => t.id === id)
    if (idx < 0) return
    tabs.value.splice(idx, 1)
    if (tabs.value.length === 0) addTab()
    if (activeTabId.value === id) {
      activeTabId.value = tabs.value[Math.min(idx, tabs.value.length - 1)].id
    }
  }

  function closeOthers(id: string) {
    const tab = tabs.value.find((t) => t.id === id)
    if (!tab) return
    tabs.value = [tab]
    activeTabId.value = id
  }

  function renameTab(id: string, newTitle: string) {
    const tab = tabs.value.find((t) => t.id === id)
    if (tab) tab.title = newTitle
  }

  function getTabSql(id: string): string {
    return tabs.value.find((t) => t.id === id)?.sql ?? ''
  }

  function clearEditor() {
    const tab = tabs.value.find((t) => t.id === activeTabId.value)
    if (tab) tab.sql = ''
  }

  async function execute(sql: string) {
    if (!sql.trim() || executing.value) return
    executing.value = true
    try {
      activeTab.value.result = await executeSql(resourceId, sql)
    } catch (e: any) {
      activeTab.value.result = { columns: [], rows: [], affected_rows: 0, elapsed_ms: 0 }
      const msg = e.response?.data?.error?.message || e.message || '执行失败'
      onError?.(msg)
    } finally {
      executing.value = false
    }
  }

  function handleSort(column: string, direction: 'asc' | 'desc') {
    if (!activeTab.value.result) return
    const colIdx = activeTab.value.result.columns.findIndex((c) => c.name === column)
    if (colIdx < 0) return
    const sorted = [...activeTab.value.result.rows].sort((a, b) => {
      const va = a[colIdx]
      const vb = b[colIdx]
      if (va === null) return 1
      if (vb === null) return -1
      if (va === vb) return 0
      return direction === 'asc' ? (va < vb ? -1 : 1) : (va > vb ? -1 : 1)
    })
    activeTab.value.result = { ...activeTab.value.result, rows: sorted }
  }

  function handleGenerateSql(sql: string) {
    activeTab.value.sql = sql
  }

  return {
    tabs,
    activeTabId,
    executing,
    tabList,
    activeTab,
    addTab,
    closeTab,
    closeOthers,
    renameTab,
    getTabSql,
    clearEditor,
    execute,
    handleSort,
    handleGenerateSql,
  }
}
