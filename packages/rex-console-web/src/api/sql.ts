//! SQL 控制台 API 调用封装（统一走 ApiClient，错误携带 code，自动弹 toast）

import { api } from './client'

export interface ConnectRequest {
  type: string
  resource_id: string
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
  const data = await api.post<{ session_id: string }>('/sql/connect', req)
  return data.session_id
}

export async function disconnect(sessionId: string): Promise<void> {
  await api.post<{ ok: boolean }>('/sql/disconnect', { session_id: sessionId })
}

export async function executeQuery(sessionId: string, sql: string): Promise<QueryResult> {
  return api.post<QueryResult>('/sql/query', { session_id: sessionId, sql })
}

export async function getDatabases(sessionId: string): Promise<string[]> {
  return api.get<string[]>('/sql/databases', { session_id: sessionId })
}

export async function getTables(sessionId: string, db: string): Promise<TableInfo[]> {
  return api.get<TableInfo[]>('/sql/tables', { session_id: sessionId, db })
}

export async function getColumns(sessionId: string, db: string, table: string): Promise<ColumnInfo[]> {
  return api.get<ColumnInfo[]>('/sql/columns', { session_id: sessionId, db, table })
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
  return api.get<IndexInfo[]>('/sql/indexes', { session_id: sessionId, db, table })
}

export async function getForeignKeys(sessionId: string, db: string, table: string): Promise<ForeignKeyInfo[]> {
  return api.get<ForeignKeyInfo[]>('/sql/foreign_keys', { session_id: sessionId, db, table })
}

export async function getDdl(sessionId: string, db: string, table: string): Promise<DdlResult> {
  return api.get<DdlResult>('/sql/ddl', { session_id: sessionId, db, table })
}

// --- Saved SQL Queries (命名查询，持久化于 Hub settings 表) ---

export interface SavedQuery {
  id: string
  name: string
  sql: string
  db_type?: string | null
  updated_at?: string | null
}

export async function listSavedQueries(): Promise<SavedQuery[]> {
  return api.get<SavedQuery[]>('/sql/saved-queries')
}

export async function upsertSavedQuery(q: Partial<SavedQuery> & { name: string; sql: string }): Promise<SavedQuery> {
  return api.post<SavedQuery>('/sql/saved-queries', q)
}

export async function deleteSavedQuery(id: string): Promise<void> {
  await api.del<{ ok: boolean }>(`/sql/saved-queries/${encodeURIComponent(id)}`)
}

// --- Data Compare ---

export interface CompareSummary {
  left_rows: number
  right_rows: number
  identical_rows: number
  modified_rows: number
  only_in_left: number
  only_in_right: number
}

export interface DiffRow {
  row_index: number
  diff_type: string
  column: string
  left_value: unknown
  right_value: unknown
}

export interface CompareResult {
  left: QueryResult
  right: QueryResult
  diffs: DiffRow[]
  summary: CompareSummary
}

export async function compare(
  sessionId: string,
  sqlLeft: string,
  sqlRight: string,
  keyColumns?: string[],
): Promise<CompareResult> {
  return api.post<CompareResult>('/sql/compare', {
    session_id: sessionId,
    sql_left: sqlLeft,
    sql_right: sqlRight,
    key_columns: keyColumns,
  })
}
