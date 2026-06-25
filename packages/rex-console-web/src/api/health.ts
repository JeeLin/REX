import { client } from './client'

export interface HealthResponse {
  status: string
  uptime_seconds: number
  version: string
  database: {
    size_bytes: number
    tables: {
      environments: number
      resources: number
      audit_log: number
      metrics: number
    }
  }
  system: {
    cpu_usage_percent: number
    memory_usage_percent: number
    disk_usage_percent: number
  }
  connections: {
    agents_online: number
    agents_total: number
    active_sessions: number
  }
}

export function fetchHealth(): Promise<HealthResponse> {
  return client.get('/api/health').then((res) => res.data)
}