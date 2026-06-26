import client from './client'

export interface Agent {
  id: string
  environment_id: string
  name: string
  version: string
  os: string
  arch: string
  hostname: string | null
  os_version: string | null
  status: 'online' | 'offline'
  last_seen_at: string | null
}

export async function listAgents(envId: string): Promise<Agent[]> {
  const { data } = await client.get<{ data: Agent[] }>(
    `/environments/${envId}/agents`,
  )
  return data.data
}

/**
 * 重置指定 Agent 所在环境的注册令牌。
 * 返回新的明文 token（仅展示一次）。
 */
export async function resetAgentToken(agentId: string): Promise<string> {
  const { data } = await client.post<{ data: { token: string } }>(
    `/agents/${agentId}/reset-token`,
  )
  return data.data.token
}

export interface AgentConfig {
  auto_update: boolean
}

/** 获取 Agent 配置 */
export async function getAgentConfig(agentId: string): Promise<AgentConfig> {
  const { data } = await client.get<{ data: AgentConfig }>(
    `/agents/${agentId}/config`,
  )
  return data.data
}

/** 更新 Agent 配置（部分更新） */
export async function updateAgentConfig(
  agentId: string,
  config: Partial<AgentConfig>,
): Promise<AgentConfig> {
  const { data } = await client.patch<{ data: AgentConfig }>(
    `/agents/${agentId}/config`,
    config,
  )
  return data.data
}
