<template>
  <div>
    <div class="section-header">
      <h2 class="section-title">{{ t('agent.title') }}</h2>
    </div>

    <div v-if="loading" class="loading-text">{{ t('common.loading') }}</div>

    <template v-else>
      <div v-if="agents.length === 0" class="empty-state">
        <p>{{ t('agent.noAgents') }}</p>
        <p class="empty-hint">{{ t('agent.noAgentsHint') }}</p>
      </div>

      <div v-else class="agent-grid">
        <AgentCard v-for="agent in agents" :key="agent.id" :agent="agent" :hub-version="hubVersion" />
      </div>
    </template>

    <DeployGuide />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import client from '@/api/client'
import { getUpdateStatus } from '@/api/update'
import type { Agent } from '@/api/agent'
import AgentCard from '@/features/agents/AgentCard.vue'
import DeployGuide from '@/features/agents/DeployGuide.vue'

const { t } = useI18n()

const agents = ref<Agent[]>([])
const hubVersion = ref('')
const loading = ref(true)

onMounted(async () => {
  // Fetch update status for hub version
  try {
    const status = await getUpdateStatus()
    hubVersion.value = status.current_version
  } catch {
    // ignore
  }

  // Fetch all agents across all environments
  try {
    const envResp = await client.get<{ data: Array<{ id: string }> }>('/environments')
    const envs = envResp.data.data

    const allAgents: Agent[] = []
    for (const env of envs) {
      try {
        const agentResp = await client.get<{ data: Agent[] }>(`/environments/${env.id}/agents`)
        allAgents.push(...agentResp.data.data)
      } catch {
        // 静默处理
      }
    }
    agents.value = allAgents
  } catch {
    // 静默处理
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.loading-text {
  text-align: center;
  padding: var(--sp-3xl);
  color: var(--text-secondary);
}

.empty-state {
  text-align: center;
  padding: var(--sp-3xl);
  color: var(--text-secondary);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--sp-sm);
}

.empty-hint {
  font-size: var(--fs-sm);
  color: var(--text-muted);
}

.agent-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: var(--sp-md);
}
</style>
