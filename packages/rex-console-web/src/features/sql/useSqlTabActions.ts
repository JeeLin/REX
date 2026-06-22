import { ref, computed } from 'vue'
import { executeSql } from '@/api/sql'
import type { SqlResult } from '@/api/sql'
import { getErrorMessage } from '@/utils/error'

export interface QueryTab {
  id: string
  title: string
  sql: string
  result: SqlResult | null
  message: string
  isError: boolean
  /** 已保存的查询文件 ID，null 表示未保存 */
  queryId: string | null
}

export function useSqlTabActions(
  resourceId: string,
  onError?: (msg: string) => void,
  onExecuted?: (sql: string, result: SqlResult) => void,
) {
  const tabs = ref<QueryTab[]>([])
  const activeTabId = ref('')
  const executing = ref(false)
  let tabCounter = 0

  const tabList = computed(() =>
    tabs.value.map((t) => ({ id: t.id, title: t.title, queryId: t.queryId })),
  )

  const activeTab = computed(() => {
    const tab = tabs.value.find((t) => t.id === activeTabId.value)
    return tab ?? tabs.value[0] ?? { id: '', title: '', sql: '', result: null, queryId: null }
  })

  function addTab() {
    tabCounter++
    const id = `sql-tab-${Date.now()}-${tabCounter}`
    tabs.value.push({ id, title: `查询 ${tabCounter}`, sql: '', result: null, message: '', isError: false, queryId: null })
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

  function closeAll() {
    tabs.value = []
    addTab()
  }

  function closeSaved() {
    // 关闭所有已保存的查询标签（queryId 不为 null），保留未保存的
    tabs.value = tabs.value.filter((t) => t.queryId === null)
    if (tabs.value.length === 0) addTab()
    if (!tabs.value.find((t) => t.id === activeTabId.value)) {
      activeTabId.value = tabs.value[0].id
    }
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

  /** 打开一个已保存的查询文件到新标签 */
  function openQueryFile(queryId: string, title: string, sql: string) {
    // 检查是否已经打开了这个查询文件
    const existing = tabs.value.find((t) => t.queryId === queryId)
    if (existing) {
      activeTabId.value = existing.id
      return
    }
    tabCounter++
    const id = `sql-tab-${Date.now()}-${tabCounter}`
    tabs.value.push({ id, title, sql, result: null, message: '', isError: false, queryId })
    activeTabId.value = id
  }

  /** 标记当前标签为已保存 */
  function markSaved(id: string, queryId: string) {
    const tab = tabs.value.find((t) => t.id === id)
    if (tab) tab.queryId = queryId
  }

  /** 获取当前标签的查询文件 ID */
  function getQueryId(id: string): string | null {
    return tabs.value.find((t) => t.id === id)?.queryId ?? null
  }

  async function execute(sql: string) {
    if (!sql.trim() || executing.value) return
    executing.value = true
    try {
      const result = await executeSql(resourceId, sql)
      activeTab.value.result = result
      activeTab.value.message = `Query OK, ${result.affected_rows ?? result.rows.length} rows affected (${(result.elapsed_ms / 1000).toFixed(3)}s)`
      activeTab.value.isError = false
      onExecuted?.(sql, result)
    } catch (e: unknown) {
      activeTab.value.result = { columns: [], rows: [], affected_rows: 0, elapsed_ms: 0 }
      const msg = getErrorMessage(e, '执行失败')
      activeTab.value.message = `ERROR: ${msg}`
      activeTab.value.isError = true
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
      const va = a[colIdx] as string | number | null
      const vb = b[colIdx] as string | number | null
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
    closeAll,
    closeSaved,
    renameTab,
    getTabSql,
    clearEditor,
    openQueryFile,
    markSaved,
    getQueryId,
    execute,
    handleSort,
    handleGenerateSql,
  }
}
