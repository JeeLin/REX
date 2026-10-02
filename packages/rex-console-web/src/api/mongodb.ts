//! MongoDB 控制台 API 调用封装（统一走 ApiClient，错误携带 code，自动弹 toast）

import { api } from './client'

export async function connect(resourceId: string): Promise<string> {
  const data = await api.post<{ session_id: string }>('/mongodb/connect', { resource_id: resourceId })
  return data.session_id
}

export async function disconnect(sessionId: string): Promise<void> {
  await api.post<{ ok: boolean }>('/mongodb/disconnect', { session_id: sessionId })
}

export async function getDatabases(sessionId: string): Promise<string[]> {
  const data = await api.get<unknown>('/mongodb/databases', { session_id: sessionId })
  return Array.isArray(data) ? data : (data as { databases?: string[] }).databases || []
}

export async function getCollections(sessionId: string, database: string): Promise<string[]> {
  const data = await api.get<unknown>('/mongodb/collections', {
    session_id: sessionId,
    database,
  })
  return Array.isArray(data) ? data : (data as { collections?: string[] }).collections || []
}

export interface QueryResult {
  documents?: Record<string, unknown>[]
  count?: number
  error?: string
}

export async function query(
  sessionId: string,
  database: string,
  collection: string,
  operation: string,
  filter?: Record<string, unknown>,
  options?: { sort?: Record<string, unknown>; projection?: Record<string, unknown>; limit?: number },
): Promise<QueryResult> {
  return api.post<QueryResult>('/mongodb/query', {
    session_id: sessionId,
    database,
    collection,
    operation,
    filter,
    ...options,
  })
}
