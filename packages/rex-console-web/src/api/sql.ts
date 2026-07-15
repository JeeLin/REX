//! SQL 控制台 API 调用封装

const API_BASE = '/api/sql'

export interface ConnectRequest {
  type: string
  host: string
  port: number
  username: string
  password?: string
  database?: string
}

export interface TableInfo {
  name: string
  table_type: string
}

export interface ColumnInfo {
  name: string
  data_type: string
  nullable: boolean
  is_primary_key: boolean
}

export interface QueryResult {
  columns: ColumnInfo[]
  rows: unknown[][]
  affected_rows: number
  elapsed_ms: number
}

export async function connect(req: ConnectRequest): Promise<string> {
  const res = await fetch(`${API_BASE}/connect`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  })
  if (!res.ok) {
    const body = await res.json().catch(() => null)
    throw new Error(body?.error?.message || 'Connection failed')
  }
  return (await res.json()).session_id
}

export async function disconnect(sessionId: string): Promise<void> {
  await fetch(`${API_BASE}/disconnect`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ session_id: sessionId }),
  })
}

export async function executeQuery(sessionId: string, sql: string): Promise<QueryResult> {
  const res = await fetch(`${API_BASE}/query`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ session_id: sessionId, sql }),
  })
  if (!res.ok) {
    const body = await res.json().catch(() => null)
    throw new Error(body?.error?.message || 'Query failed')
  }
  return await res.json()
}

export async function getDatabases(sessionId: string): Promise<string[]> {
  const res = await fetch(`${API_BASE}/databases?session_id=${sessionId}`)
  if (!res.ok) throw new Error('Failed to fetch databases')
  return await res.json()
}

export async function getTables(sessionId: string, db: string): Promise<TableInfo[]> {
  const res = await fetch(`${API_BASE}/tables?session_id=${sessionId}&db=${encodeURIComponent(db)}`)
  if (!res.ok) throw new Error('Failed to fetch tables')
  return await res.json()
}

export async function getColumns(sessionId: string, db: string, table: string): Promise<ColumnInfo[]> {
  const res = await fetch(
    `${API_BASE}/columns?session_id=${sessionId}&db=${encodeURIComponent(db)}&table=${encodeURIComponent(table)}`,
  )
  if (!res.ok) throw new Error('Failed to fetch columns')
  return await res.json()
}
