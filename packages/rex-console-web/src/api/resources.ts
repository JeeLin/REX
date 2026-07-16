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
    api.get<Resource[]>(`/environments/${envId}/resources`),
  get: (envId: string, id: string) =>
    api.get<Resource>(`/environments/${envId}/resources/${id}`),
  create: (envId: string, data: NewResource) =>
    api.post<Resource>(`/environments/${envId}/resources`, data),
  update: (envId: string, id: string, data: NewResource) =>
    api.put<Resource>(`/environments/${envId}/resources/${id}`, data),
  delete: (envId: string, id: string) =>
    api.del<{ ok: boolean }>(`/environments/${envId}/resources/${id}`),
  testConnection: (data: TestConnectionRequest) =>
    api.post<TestConnectionResult>('/resources/test-connection', data),
}
