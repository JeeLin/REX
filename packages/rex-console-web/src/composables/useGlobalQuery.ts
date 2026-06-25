import { ref } from 'vue'

export interface SqlResource {
  id: string
  name: string
  protocol: string // 'mysql' | 'postgresql'
}

interface GlobalQueryRequest {
  resource_ids: string[]
  sql: string
  limit?: number
  offset?: number
}

interface GlobalQueryEvent {
  type: 'Start' | 'Result' | 'Progress' | 'Done' | 'Error'
  data?: {
    connection_id?: string
    connectionId?: string
    data?: any[]
    columns?: string[]
    row_count?: number
    rowCount?: number
    completed?: number
    total?: number
    total_rows?: number
    message?: string
  }
}

export function useGlobalQuery(resources: SqlResource[]) {
  const selectedResources = ref<string[]>([])
  const sqlQuery = ref('')
  const isExecuting = ref(false)
  const progress = ref(0)
  const results = ref<Array<{
    connectionId: string
    connectionName: string
    data: any[]
    columns: string[]
    rowCount: number
    error?: string
  }>>([])
  const activeTab = ref(0)

  let abortController: AbortController | null = null

  function handleEvent(event: GlobalQueryEvent) {
    const d = event.data
    switch (event.type) {
      case 'Start':
        break

      case 'Result': {
        const connId = d?.connection_id ?? d?.connectionId
        if (!connId || !d?.data) break

        const columns = d.columns ?? (d.data.length > 0 ? Object.keys(d.data[0]) : [])
        const rowCount = d.row_count ?? d.rowCount ?? d.data.length

        const existingIndex = results.value.findIndex(r => r.connectionId === connId)
        const entry = {
          connectionId: connId,
          connectionName: connId,
          data: d.data,
          columns,
          rowCount,
        }

        if (existingIndex >= 0) {
          results.value[existingIndex] = entry
        } else {
          results.value.push(entry)
        }

        if (results.value.length === 1) {
          activeTab.value = 0
        }
        break
      }

      case 'Progress':
        if (d?.completed !== undefined && d?.total !== undefined) {
          progress.value = Math.round((d.completed / d.total) * 100)
        }
        break

      case 'Done':
        isExecuting.value = false
        break

      case 'Error': {
        const connId = d?.connection_id ?? d?.connectionId
        if (!connId || !d?.message) break

        const existingIndex = results.value.findIndex(r => r.connectionId === connId)
        if (existingIndex >= 0) {
          results.value[existingIndex].error = d.message
        } else {
          results.value.push({
            connectionId: connId,
            connectionName: connId,
            data: [],
            columns: [],
            rowCount: 0,
            error: d.message,
          })
        }
        break
      }
    }
  }

  async function executeGlobalQuery() {
    if (!selectedResources.value.length || !sqlQuery.value.trim()) {
      return
    }

    isExecuting.value = true
    progress.value = 0
    results.value = []
    activeTab.value = 0

    abortController = new AbortController()

    try {
      const request: GlobalQueryRequest = {
        resource_ids: selectedResources.value,
        sql: sqlQuery.value.trim(),
      }

      const response = await fetch('/api/sql/global-query', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
        signal: abortController.signal,
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`)
      }

      const reader = response.body!.getReader()
      const decoder = new TextDecoder()
      let buffer = ''

      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        buffer += decoder.decode(value, { stream: true })
        const lines = buffer.split('\n')
        buffer = lines.pop() ?? ''

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            try {
              const event: GlobalQueryEvent = JSON.parse(line.slice(6))
              handleEvent(event)
            } catch {
              // skip malformed SSE lines
            }
          }
        }
      }
    } catch (error) {
      if ((error as Error).name !== 'AbortError') {
        isExecuting.value = false
      }
    }
  }

  function cancelQuery() {
    abortController?.abort()
    abortController = null
    isExecuting.value = false
  }

  function checkCompatibility(resourceProtocol: string): boolean {
    if (!selectedResources.value.length) return true

    const first = resources.find(r => selectedResources.value.includes(r.id))
    if (!first) return true

    return first.protocol === resourceProtocol
  }

  function selectAllCompatible() {
    if (!selectedResources.value.length) return

    const first = resources.find(r => selectedResources.value.includes(r.id))
    if (!first) return

    selectedResources.value = resources
      .filter(r => r.protocol === first.protocol)
      .map(r => r.id)
  }

  function onResourceSelectionChange() {
    // no-op, Vue reactivity handles checkbox state
  }

  return {
    selectedResources,
    sqlQuery,
    isExecuting,
    progress,
    results,
    activeTab,
    executeGlobalQuery,
    cancelQuery,
    checkCompatibility,
    onResourceSelectionChange,
    selectAllCompatible,
  }
}
