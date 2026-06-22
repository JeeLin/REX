import client from './client'

export interface AuditStats {
  total: number
  success: number
  failed: number
}

export async function getAuditStats(period?: string): Promise<AuditStats> {
  const params = period ? { period } : {}
  const res = await client.get('/audit/stats', { params })
  return res.data.data
}

export interface AuditLogEntry {
  id: string
  time: string
  user: string
  ip: string | null
  environment_id: string | null
  resource_id: string | null
  agent_id: string | null
  type: string
  result: string
  summary: string
  detail: string | null
}

export interface AuditLogList {
  items: AuditLogEntry[]
  total: number
  page: number
  page_size: number
}

export interface AuditLogParams {
  from?: string
  to?: string
  type?: string
  page?: number
  page_size?: number
}

export async function listAuditLog(params?: AuditLogParams): Promise<AuditLogList> {
  const res = await client.get('/audit-log', { params })
  return res.data.data
}
