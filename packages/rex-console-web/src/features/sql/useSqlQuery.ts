import { ref } from 'vue'
import { executeQuery, type QueryResult } from '@/api/sql'

export type ExecuteMode = 'all' | 'current' | 'selected'

export interface QueryState {
  result: QueryResult | null
  loading: boolean
  error: string | null
}

/**
 * 将 SQL 文本按语句分割（简单版：按 ; 分割，忽略引号内的 ;）
 */
function splitStatements(sql: string): string[] {
  const stmts: string[] = []
  let current = ''
  let inSingleQuote = false
  let inDoubleQuote = false

  for (const ch of sql) {
    if (ch === "'" && !inDoubleQuote) {
      inSingleQuote = !inSingleQuote
    } else if (ch === '"' && !inSingleQuote) {
      inDoubleQuote = !inDoubleQuote
    } else if (ch === ';' && !inSingleQuote && !inDoubleQuote) {
      const trimmed = current.trim()
      if (trimmed) stmts.push(trimmed)
      current = ''
      continue
    }
    current += ch
  }
  const trimmed = current.trim()
  if (trimmed) stmts.push(trimmed)
  return stmts
}

/**
 * 根据光标位置找到所在语句
 */
function findStatementAtCursor(sql: string, cursorPos: number): string {
  const stmts = splitStatements(sql)
  let offset = 0
  for (const stmt of stmts) {
    const end = offset + stmt.length
    if (cursorPos >= offset && cursorPos <= end) {
      return stmt
    }
    offset = end + 1 // +1 for the ';' separator
  }
  return stmts[stmts.length - 1] || sql
}

export function useSqlQuery(sessionId: () => string | null) {
  const mode = ref<ExecuteMode>('all')

  async function run(
    sql: string,
    state: QueryState,
    selectedText?: string,
    cursorPos?: number,
  ) {
    let sqlToRun = sql

    if (mode.value === 'selected' && selectedText) {
      sqlToRun = selectedText.trim()
    } else if (mode.value === 'current' && cursorPos !== undefined) {
      sqlToRun = findStatementAtCursor(sql, cursorPos)
    }

    if (!sqlToRun.trim()) return

    state.loading = true
    state.error = null
    try {
      const sid = sessionId()
      if (!sid) throw new Error('Not connected')
      state.result = await executeQuery(sid, sqlToRun)
    } catch (e: unknown) {
      state.error = e instanceof Error ? e.message : String(e)
      state.result = null
    } finally {
      state.loading = false
    }
  }

  return { mode, run }
}
