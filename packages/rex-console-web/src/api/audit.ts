import client from './client'

export interface AuditStats {
  total: number
  success: number
  failed: number
}

export async function getAuditStats(period?: string): Promise<AuditStats> {
  const params = period ? { period } : {}
  const res = await client.get('/api/audit/stats', { params })
  return res.data.data
}
