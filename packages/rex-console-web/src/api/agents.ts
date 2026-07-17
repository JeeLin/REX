import { api } from './client'

export interface Agent {
  id: string
  environment_id: string
  name: string
  version: string
  os: string
  arch: string
  hostname: string
  ip: string
  status: string
  last_seen_at: string | null
  created_at: string
  updated_at: string
}

export const agentsApi = {
  listByEnv: (envId: string) =>
    api.get<Agent[]>(`/environments/${envId}/agents`),
  get: (id: string) =>
    api.get<Agent>(`/agents/${id}`),
  resetToken: (id: string) =>
    api.post<{ token: string }>(`/agents/${id}/reset-token`),
}
