import { api } from './client'
import type { Resource } from './resources'

export interface DashboardStats {
  environment_count: number
  resource_count: number
  online_agents: number
}

export const dashboardApi = {
  stats: () => api.get<DashboardStats>('/dashboard/stats'),
  recent: () => api.get<Resource[]>('/dashboard/recent'),
}
