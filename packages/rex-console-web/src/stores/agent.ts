import { defineStore } from 'pinia'
import { ref } from 'vue'
import { listAgents, type Agent } from '@/api/agent'

export const useAgentStore = defineStore('agent', () => {
  const agentsByEnv = ref<Record<string, Agent[]>>({})
  const loading = ref(false)

  async function fetchAgents(envId: string) {
    loading.value = true
    try {
      const agents = await listAgents(envId)
      agentsByEnv.value[envId] = agents
    } catch {
      // 静默处理
    } finally {
      loading.value = false
    }
  }

  function getAgents(envId: string): Agent[] {
    return agentsByEnv.value[envId] || []
  }

  function getOnlineCount(envId: string): number {
    return getAgents(envId).filter((a) => a.status === 'online').length
  }

  function getTotalCount(envId: string): number {
    return getAgents(envId).length
  }

  return { agentsByEnv, loading, fetchAgents, getAgents, getOnlineCount, getTotalCount }
})
