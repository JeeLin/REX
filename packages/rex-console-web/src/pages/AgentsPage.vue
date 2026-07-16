<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { agentsApi, type Agent } from '@/api/agents'
import { useEnvironmentsStore } from '@/stores/environments'
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Modal from '@/components/ui/Modal.vue'

const { t } = useI18n()
const store = useEnvironmentsStore()
const agents = ref<Agent[]>([])
const loading = ref(true)
const resetModal = ref(false)
const resetAgentId = ref('')
const resetToken = ref('')

onMounted(async () => {
  await store.fetchEnvironments()
  // 加载所有环境的 agents
  const allAgents: Agent[] = []
  for (const env of store.environments) {
    try {
      const envAgents = await agentsApi.listByEnv(env.id)
      allAgents.push(...envAgents)
    } catch {
      // ignore
    }
  }
  agents.value = allAgents
  loading.value = false
})

const hasAgents = computed(() => agents.value.length > 0)

function agentStatus(status: string): StatusDotStatus {
  if (status === 'online') return 'online'
  if (status === 'connecting') return 'connecting'
  return 'offline'
}

function envName(envId: string): string {
  return store.environments.find(e => e.id === envId)?.name || envId
}

async function openResetToken(agentId: string) {
  resetAgentId.value = agentId
  resetToken.value = ''
  resetModal.value = true
}

async function doResetToken() {
  try {
    const result = await agentsApi.resetToken(resetAgentId.value)
    resetToken.value = result.token
  } catch {
    // ignore
  }
}
</script>

<template>
  <div class="agents-page">
    <header class="page-header">
      <h1 class="page-title">{{ t('agents.title') }}</h1>
    </header>

    <EmptyState
      v-if="!loading && !hasAgents"
      icon="⬡"
      :title="t('agents.noAgents')"
      :description="t('agents.noAgentsDesc')"
    />

    <div v-else class="agent-grid">
      <Card v-for="agent in agents" :key="agent.id" class="agent-card">
        <div class="agent-card-header">
          <div class="agent-info">
            <div class="agent-name">
              <StatusDot :status="agentStatus(agent.status)" />
              <span class="mono">{{ agent.name }}</span>
            </div>
            <div class="agent-meta muted">
              {{ envName(agent.environment_id) }} · {{ agent.hostname || agent.ip || '—' }}
            </div>
          </div>
        </div>
        <div class="agent-details">
          <div class="agent-detail">
            <span class="muted">{{ t('agents.version') }}</span>
            <span class="mono">{{ agent.version || '—' }}</span>
          </div>
          <div class="agent-detail">
            <span class="muted">{{ t('agents.os') }}</span>
            <span>{{ agent.os || '—' }} {{ agent.arch }}</span>
          </div>
          <div class="agent-detail">
            <span class="muted">{{ t('agents.lastSeen') }}</span>
            <span>{{ agent.last_seen_at ? new Date(agent.last_seen_at).toLocaleString() : '—' }}</span>
          </div>
        </div>
        <div class="agent-footer">
          <Button variant="secondary" size="sm" @click="openResetToken(agent.id)">{{ t('agents.resetToken') }}</Button>
        </div>
      </Card>
    </div>

    <!-- Reset Token Modal -->
    <Modal v-model="resetModal">
      <template #title>{{ t('agents.resetTitle') }}</template>
      <div v-if="!resetToken" class="modal-content">
        <p class="muted">{{ t('agents.resetDesc') }}</p>
        <div class="form-actions">
          <Button variant="secondary" @click="resetModal = false">{{ t('common.cancel') }}</Button>
          <Button variant="primary" @click="doResetToken">{{ t('agents.generate') }}</Button>
        </div>
      </div>
      <div v-else class="modal-content">
        <p class="muted">{{ t('agents.newToken') }}</p>
        <code class="token-display mono">{{ resetToken }}</code>
        <div class="form-actions">
          <Button variant="primary" @click="resetModal = false">{{ t('agents.done') }}</Button>
        </div>
      </div>
    </Modal>
  </div>
</template>

<style scoped>
.agents-page {
  max-width: 900px;
}
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-6);
}
.page-title {
  font-size: var(--text-xl);
  font-weight: 600;
  color: var(--text-primary);
}
.agent-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: var(--space-4);
}
.agent-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.agent-card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}
.agent-info {
  flex: 1;
}
.agent-name {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
}
.agent-meta {
  font-size: var(--text-sm);
  margin-top: var(--space-1);
}
.agent-details {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding-top: var(--space-2);
  border-top: 1px solid var(--border);
}
.agent-detail {
  display: flex;
  justify-content: space-between;
  font-size: var(--text-sm);
  color: var(--text-secondary);
}
.agent-footer {
  display: flex;
  justify-content: flex-end;
  padding-top: var(--space-2);
  border-top: 1px solid var(--border);
}
.muted {
  color: var(--text-muted);
}
.mono {
  font-family: var(--font-mono);
}
.modal-content {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.token-display {
  display: block;
  padding: var(--space-3);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: var(--text-sm);
  word-break: break-all;
  color: var(--accent);
}
.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
}
</style>
