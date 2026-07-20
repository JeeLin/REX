//! SQL 控制台 API 调用封装

const API_BASE = '/api/sql'

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('rex-token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}

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
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
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
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId }),
  })
}

export async function executeQuery(sessionId: string, sql: string): Promise<QueryResult> {
  const res = await fetch(`${API_BASE}/query`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, sql }),
  })
  if (!res.ok) {
    const body = await res.json().catch(() => null)
    throw new Error(body?.error?.message || 'Query failed')
  }
  return await res.json()
}

export async function getDatabases(sessionId: string): Promise<string[]> {
  const res = await fetch(`${API_BASE}/databases?session_id=${sessionId}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Failed to fetch databases')
  return await res.json()
}

export async function getTables(sessionId: string, db: string): Promise<TableInfo[]> {
  const res = await fetch(`${API_BASE}/tables?session_id=${sessionId}&db=${encodeURIComponent(db)}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Failed to fetch tables')
  return await res.json()
}

export async function getColumns(sessionId: string, db: string, table: string): Promise<ColumnInfo[]> {
  const res = await fetch(
    `${API_BASE}/columns?session_id=${sessionId}&db=${encodeURIComponent(db)}&table=${encodeURIComponent(table)}`,
    { headers: authHeaders() },
  )
  if (!res.ok) throw new Error('Failed to fetch columns')
  return await res.json()
}

export interface IndexInfo {
  name: string
  columns: string[]
  unique: boolean
  index_type: string
}

export interface ForeignKeyInfo {
  name: string
  columns: string[]
  ref_table: string
  ref_columns: string[]
  on_delete: string
  on_update: string
}

export interface DdlResult {
  ddl: string
}

export async function getIndexes(sessionId: string, db: string, table: string): Promise<IndexInfo[]> {
  const res = await fetch(
    `${API_BASE}/indexes?session_id=${sessionId}&db=${encodeURIComponent(db)}&table=${encodeURIComponent(table)}`,
    { headers: authHeaders() },
  )
  if (!res.ok) throw new Error('Failed to fetch indexes')
  return await res.json()
}

export async function getForeignKeys(sessionId: string, db: string, table: string): Promise<ForeignKeyInfo[]> {
  const res = await fetch(
    `${API_BASE}/foreign_keys?session_id=${sessionId}&db=${encodeURIComponent(db)}&table=${encodeURIComponent(table)}`,
    { headers: authHeaders() },
  )
  if (!res.ok) throw new Error('Failed to fetch foreign keys')
  return await res.json()
}

export async function getDdl(sessionId: string, db: string, table: string): Promise<DdlResult> {
  const res = await fetch(
    `${API_BASE}/ddl?session_id=${sessionId}&db=${encodeURIComponent(db)}&table=${encodeURIComponent(table)}`,
    { headers: authHeaders() },
  )
  if (!res.ok) throw new Error('Failed to fetch DDL')
  return await res.json()
}
