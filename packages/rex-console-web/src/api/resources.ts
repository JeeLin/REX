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
  // v0.70.7：资源子类（通用可空列）。SQL 资源存探测出的方言（mysql/postgresql/sqlite）；其他为 null。
  subtype?: string | null
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
  // v0.70.7：可选子类。SQL 资源显式指定 dialect 以跳过自动识别；缺省由后端连接时探测。
  subtype?: string | null
  color?: string | null
  sort_order?: number
}

export interface TestConnectionRequest {
  protocol: string
  host: string
  port?: number | null
  username?: string
  config_json?: string
  environment_id?: string
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
  setActiveAccount: (envId: string, id: string, accountId: string) =>
    api.post<Resource>(`/environments/${envId}/resources/${id}/active-account`, {
      account_id: accountId,
    }),
  testConnection: (data: TestConnectionRequest) =>
    api.post<TestConnectionResult>('/resources/test-connection', data),
}
