import client from './client'

// ── 类型定义 ──────────────────────────────────────────────

export interface SqlColumn {
  name: string
  data_type: string
}

export interface SqlResult {
  columns: SqlColumn[]
  rows: any[][]
  affected_rows: number
  elapsed_ms: number
}

export interface DatabaseInfo {
  name: string
}

export interface TableInfo {
  name: string
  row_count: number | null
}

export interface ColumnInfo {
  name: string
  data_type: string
  is_nullable: boolean
  is_primary_key: boolean
}

// ── API 函数 ──────────────────────────────────────────────

/** 执行 SQL 查询 */
export function executeSql(resourceId: string, sql: string): Promise<SqlResult> {
  return client
    .post(`/resources/${resourceId}/sql/execute`, { sql })
    .then((r) => r.data.data)
}

/** 列出数据库 */
export function listDatabases(resourceId: string): Promise<DatabaseInfo[]> {
  return client
    .get(`/resources/${resourceId}/sql/databases`)
    .then((r) => r.data.data)
}

/** 列出表 */
export function listTables(resourceId: string, database: string): Promise<TableInfo[]> {
  return client
    .get(`/resources/${resourceId}/sql/tables`, { params: { database } })
    .then((r) => r.data.data)
}

/** 列出列 */
export function listColumns(
  resourceId: string,
  database: string,
  table: string,
): Promise<ColumnInfo[]> {
  return client
    .get(`/resources/${resourceId}/sql/columns`, { params: { database, table } })
    .then((r) => r.data.data)
}
