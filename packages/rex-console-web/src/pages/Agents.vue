<template>
  <div>
    <div class="section-header">
      <h2 class="section-title">{{ t('agent.title') }}</h2>
    </div>

    <LoadingSpinner v-if="loading" :text="t('common.loading')" />

    <ErrorState v-else-if="loadError" :message="loadError" :retry="loadAgents" />

    <template v-else>
      <EmptyState
        v-if="agents.length === 0"
        icon="🤖"
        :title="t('agent.noAgents')"
        :hint="t('agent.noAgentsHint')"
      />

      <div v-else class="agent-grid">
        <AgentCard
          v-for="agent in agents"
          :key="agent.id"
          :agent="agent"
          :hub-version="hubVersion"
          @open-config="openConfig(agent)"
          @open-log="openLog(agent)"
          @reset-token="openResetToken(agent)"
          @restart="restartConfirm(agent)"
        />
      </div>
    </template>

    <DeployGuide />

    <!-- Modals -->
    <AgentConfigModal
      :agent="configAgent"
      :visible="showConfigModal"
      @close="showConfigModal = false"
    />
    <AgentLogModal
      :visible="showLogModal"
      :agent-id="logAgentId"
      @close="showLogModal = false"
    />
    <AgentResetTokenModal
      :agent="resetAgent"
      :visible="showResetModal"
      @close="showResetModal = false"
      @success="loadAgents"
    />
    <ConfirmDialog
      :visible="showRestartConfirm"
      :title="t('confirm.title')"
      :message="t('agent.restartConfirm', { name: restartAgentName })"
      :confirm-label="t('common.confirm')"
      :cancel-label="t('common.cancel')"
      @confirm="doRestart"
      @cancel="showRestartConfirm = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import client from '@/api/client'
import { getUpdateStatus } from '@/api/update'
import { restartAgent, type Agent } from '@/api/agent'
import { useToast } from '@/composables/useToast'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import ErrorState from '@/components/ErrorState.vue'
import EmptyState from '@/components/EmptyState.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import AgentCard from '@/features/agents/AgentCard.vue'
import DeployGuide from '@/features/agents/DeployGuide.vue'
import AgentConfigModal from '@/features/agents/AgentConfigModal.vue'
import AgentLogModal from '@/features/agents/AgentLogModal.vue'
import AgentResetTokenModal from '@/features/agents/AgentResetTokenModal.vue'

const { t } = useI18n()
const { success, error: toastError } = useToast()

const agents = ref<Agent[]>([])
const hubVersion = ref('')
const loading = ref(true)
const loadError = ref('')

// Modal state
const showConfigModal = ref(false)
const configAgent = ref<Agent | null>(null)
const showLogModal = ref(false)
const logAgentId = ref('')
const showResetModal = ref(false)
const resetAgent = ref<Agent | null>(null)
const showRestartConfirm = ref(false)
const restartAgentName = ref('')
let pendingRestartId = ''

function openConfig(agent: Agent) {
  configAgent.value = agent
  showConfigModal.value = true
}

function openLog(agent: Agent) {
  logAgentId.value = agent.id
  showLogModal.value = true
}

function openResetToken(agent: Agent) {
  resetAgent.value = agent
  showResetModal.value = true
}

function restartConfirm(agent: Agent) {
  restartAgentName.value = agent.name
  pendingRestartId = agent.id
  showRestartConfirm.value = true
}

async function doRestart() {
  showRestartConfirm.value = false
  try {
    await restartAgent(pendingRestartId)
    success(t('agent.restarted'))
  } catch {
    toastError(t('agent.restartFailed'))
  }
}

onMounted(async () => {
  try {
    const status = await getUpdateStatus()
    hubVersion.value = status.current_version
  } catch {
    // ignore
  }

  await loadAgents()
})

async function loadAgents() {
  loading.value = true
  loadError.value = ''
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
    loadError.value = t('agent.loadFailed')
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.agent-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: var(--sp-md);
}
</style>
