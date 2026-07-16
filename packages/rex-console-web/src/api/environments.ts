import { api } from './client'

export interface Environment {
  id: string
  name: string
  description: string
  connection_mode: string
  resource_count: number
  agent_status: string | null
  created_at: string
  updated_at: string
}

export interface NewEnvironment {
  name: string
  description?: string
  connection_mode?: string
}

export const environmentsApi = {
  list: () => api.get<Environment[]>('/environments'),
  get: (id: string) => api.get<Environment>(`/environments/${id}`),
  create: (data: NewEnvironment) => api.post<Environment>('/environments', data),
  update: (id: string, data: Partial<NewEnvironment>) =>
    api.put<Environment>(`/environments/${id}`, data),
  delete: (id: string) => api.del<{ ok: boolean }>(`/environments/${id}`),
}
