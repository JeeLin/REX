//! Health API — 检查连接模式（Hub/Agent）

import { api } from './client'

export interface HealthResponse {
  status: string
  mode: 'hub' | 'agent'
  version: string
}

/**
 * 获取健康检查信息
 * 用于检测连接的是 Hub 还是 Agent
 */
export async function getHealth(): Promise<HealthResponse> {
  return api.get<HealthResponse>('/health')
}