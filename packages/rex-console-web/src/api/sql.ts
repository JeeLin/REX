import client from './client'

// ── 类型定义 ──────────────────────────────────────────────

export interface SqlColumn {
  name: string
  data_type: string
}

export interface SqlResult {
  columns: SqlColumn[]
  rows: unknown[][]
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

export interface SqlResourceInfo {
  id: string
  name: string
  protocol: string
}

export interface ExplainResult {
  columns: string[]
  rows: unknown[][]
  raw_output: string
}

// ── API 函数 ──────────────────────────────────────────────

/** 获取资源基本信息 */
export function getResourceInfo(
  resourceId: string,
): Promise<{ name: string; protocol: string }> {
  return client
    .get(`/resources/${resourceId}/sql/info`)
    .then((r) => r.data.data)
}

/** 获取同环境下的 SQL 资源列表（全局查询用） */
export function listPeerSqlResources(resourceId: string): Promise<SqlResourceInfo[]> {
  return client
    .get(`/resources/${resourceId}/sql/peers`)
    .then((r) => r.data.data)
}

/** 执行 SQL 查询 */
export function executeSql(resourceId: string, sql: string): Promise<SqlResult> {
  return client
    .post(`/resources/${resourceId}/sql/execute`, { sql })
    .then((r) => r.data.data)
}

/** 获取 SQL 执行计划 */
export function explainSql(resourceId: string, sql: string): Promise<ExplainResult> {
  return client
    .post(`/resources/${resourceId}/sql/explain`, { sql })
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

// ── 查询文件 API ──────────────────────────────────────────

export interface QueryFileMeta {
  id: string
  name: string
  database: string
  created_at: string
  updated_at: string
}

export interface QueryFileDetail extends QueryFileMeta {
  sql: string
}

/** 列出资源的所有查询文件 */
export function listQueries(resourceId: string): Promise<QueryFileMeta[]> {
  return client
    .get(`/resources/${resourceId}/queries`)
    .then((r) => r.data.data)
}

/** 保存查询文件 */
export function saveQuery(
  resourceId: string,
  name: string,
  sql: string,
  database: string,
): Promise<QueryFileMeta> {
  return client
    .post(`/resources/${resourceId}/queries`, { name, sql, database })
    .then((r) => r.data.data)
}

/** 读取查询文件 */
export function getQuery(resourceId: string, id: string): Promise<QueryFileDetail> {
  return client
    .get(`/resources/${resourceId}/queries/${id}`)
    .then((r) => r.data.data)
}

/** 更新查询文件 */
export function updateQuery(
  resourceId: string,
  id: string,
  data: { name?: string; sql?: string; database?: string },
): Promise<QueryFileMeta> {
  return client
    .put(`/resources/${resourceId}/queries/${id}`, data)
    .then((r) => r.data.data)
}

/** 删除查询文件 */
export function deleteQuery(resourceId: string, id: string): Promise<void> {
  return client
    .delete(`/resources/${resourceId}/queries/${id}`)
    .then(() => undefined)
}

/** 重命名查询文件 */
export function renameQuery(
  resourceId: string,
  id: string,
  name: string,
): Promise<QueryFileMeta> {
  return client
    .put(`/resources/${resourceId}/queries/${id}/rename`, { name })
    .then((r) => r.data.data)
}

// ── 历史记录 API ──────────────────────────────────────────

export interface HistoryRecord {
  id: string
  sql: string
  database: string
  executed_at: string
  elapsed_ms: number
  row_count: number
}

/** 列出 SQL 执行历史 */
export function listHistory(resourceId: string): Promise<HistoryRecord[]> {
  return client
    .get(`/resources/${resourceId}/sql/history`)
    .then((r) => r.data.data)
}

/** 清空 SQL 执行历史 */
export function clearHistory(resourceId: string): Promise<void> {
  return client
    .delete(`/resources/${resourceId}/sql/history`)
    .then(() => undefined)
}

/** 记录一条 SQL 执行历史 */
export function recordHistory(
  resourceId: string,
  sql: string,
  database: string,
  elapsedMs: number,
  rowCount: number,
): Promise<void> {
  return client
    .post(`/resources/${resourceId}/sql/history`, {
      sql,
      database,
      elapsed_ms: elapsedMs,
      row_count: rowCount,
    })
    .then(() => undefined)
}
