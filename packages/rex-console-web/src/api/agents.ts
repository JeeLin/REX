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

export interface VersionInfo {
  hub_version: string
  latest_version: string | null
  download_url: string | null
  agents: AgentVersionInfo[]
}

export interface AgentVersionInfo {
  agent_id: string
  name: string
  version: string
  is_online: boolean
  is_up_to_date: boolean
}

export interface UpdateCheckResult {
  current_version: string
  latest_version: string
  update_available: boolean
  download_url: string
  release_notes: string
}

export interface UpdateStatus {
  phase: string
  progress: number
  current_version: string
  target_version: string
  error: string | null
  started_at: string | null
}

export const agentsApi = {
  listByEnv: (envId: string) =>
    api.get<Agent[]>(`/environments/${envId}/agents`),
  get: (id: string) =>
    api.get<Agent>(`/agents/${id}`),
  resetToken: (id: string) =>
    api.post<{ token: string }>(`/agents/${id}/reset-token`),
}

export const updateApi = {
  getVersion: () =>
    api.get<VersionInfo>('/version'),
  checkLatest: () =>
    api.get<UpdateCheckResult>('/version/check'),
  triggerUpdate: (agentId: string) =>
    api.post<{ ok: boolean; message: string }>(`/agents/${agentId}/update/trigger`),
  getUpdateStatus: (agentId: string) =>
    api.get<UpdateStatus>(`/agents/${agentId}/update/status`),
}
