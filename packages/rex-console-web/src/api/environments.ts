import { api } from './client'

export interface Environment {
  id: string
  name: string
  description: string
  connection_mode: string
  resource_count: number
  agent_status: string | null
  agent_token?: string
  created_at: string
  updated_at: string
}

export interface NewEnvironment {
  name: string
  description?: string
  connection_mode?: string
}

export interface ExportData {
  version: string
  environments: Array<{
    name: string
    description: string
    connection_mode: string
    resources: Array<{
      name: string
      protocol: string
      host: string
      port?: number
      username: string
      config_json: string
      color?: string
    }>
  }>
}

export interface ImportResult {
  imported: number
  skipped: number
}

export const environmentsApi = {
  list: () => api.get<Environment[]>('/environments'),
  get: (id: string) => api.get<Environment>(`/environments/${id}`),
  create: (data: NewEnvironment) => api.post<Environment>('/environments', data),
  update: (id: string, data: Partial<NewEnvironment>) =>
    api.put<Environment>(`/environments/${id}`, data),
  delete: (id: string) => api.del<{ ok: boolean }>(`/environments/${id}`),
  export: () => api.get<ExportData>('/environments/export'),
  import: (data: ExportData) => api.post<ImportResult>('/environments/import', data),
}
