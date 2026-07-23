import { api } from './client'

export interface Resource {
  id: string
  environment_id: string
  name: string
  protocol: string
  host: string
  port: number | null
  username: string
  config_json: string
  color: string | null
  sort_order: number
  created_at: string
  updated_at: string
}

export interface NewResource {
  name: string
  protocol: string
  host: string
  port?: number | null
  username?: string
  config_json?: string
  color?: string | null
  sort_order?: number
}

export interface TestConnectionRequest {
  protocol: string
  host: string
  port?: number | null
  username?: string
  config_json?: string
}

export interface TestConnectionResult {
  ok: boolean
  latency_ms?: number
  error?: string
}

export const resourcesApi = {
  listByEnv: (envId: string) =>
    api.get<Resource[]>(`/resources/${envId}`),
  get: (envId: string, id: string) =>
    api.get<Resource>(`/resources/${envId}/${id}`),
  create: (envId: string, data: NewResource) =>
    api.post<Resource>(`/resources/${envId}`, data),
  update: (envId: string, id: string, data: NewResource) =>
    api.put<Resource>(`/resources/${envId}/${id}`, data),
  delete: (envId: string, id: string) =>
    api.del<{ ok: boolean }>(`/resources/${envId}/${id}`),
  testConnection: (data: TestConnectionRequest) =>
    api.post<TestConnectionResult>('/resources/test-connection', data),
}
