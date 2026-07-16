import { api } from './client'

export interface AuditEntry {
  id: string
  time: string
  action: string
  target: string | null
  environment_id: string | null
  resource_id: string | null
  agent_id: string | null
  result: string
  detail: string | null
}

export interface AuditQuery {
  time_from?: string
  time_to?: string
  action?: string
  result?: string
  limit?: number
  offset?: number
}

export const auditApi = {
  query: (params?: AuditQuery) => {
    const query: Record<string, string> = {}
    if (params?.time_from) query.time_from = params.time_from
    if (params?.time_to) query.time_to = params.time_to
    if (params?.action) query.action = params.action
    if (params?.result) query.result = params.result
    if (params?.limit) query.limit = String(params.limit)
    if (params?.offset) query.offset = String(params.offset)
    return api.get<AuditEntry[]>('/audit-log', query)
  },
}
