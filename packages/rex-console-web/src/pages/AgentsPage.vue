<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { agentsApi, type Agent, type AuditEntry } from '@/api/agents'
import { useEnvironmentsStore } from '@/stores/environments'
import Card from '@/components/ui/Card.vue'
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
const logModal = ref(false)
const logAgentName = ref('')
const logEntries = ref<AuditEntry[]>([])
const logLoading = ref(false)
const logFilter = ref('')

// Deploy guide modal
const deployModal = ref(false)
const deployAgent = ref<Agent | null>(null)
const deployTab = ref<'binary' | 'docker' | 'compose' | 'config'>('binary')
const copySuccess = ref('')

// Config modal
const configModal = ref(false)
const configAgent = ref<Agent | null>(null)
const configAutoUpdate = ref(true)

function hubHost(): string {
  return window.location.origin
}

function envToken(agentEnvId: string): string {
  const env = store.environments.find(e => e.id === agentEnvId)
  return env?.agent_token || 'YOUR_TOKEN'
}

function envName(envId: string): string {
  return store.environments.find(e => e.id === envId)?.name || envId
}

onMounted(async () => {
  await store.fetchEnvironments()
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

async function openLogs(agent: Agent) {
  logAgentName.value = agent.name
  logFilter.value = ''
  logEntries.value = []
  logModal.value = true
  logLoading.value = true
  try {
    logEntries.value = await agentsApi.getLogs(agent.id)
  } catch {
    // ignore
  } finally {
    logLoading.value = false
  }
}

function openDeploy(agent: Agent) {
  deployAgent.value = agent
  deployTab.value = 'binary'
  copySuccess.value = ''
  deployModal.value = true
}

function openConfig(agent: Agent) {
  configAgent.value = agent
  configAutoUpdate.value = true
  configModal.value = true
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    copySuccess.value = 'copied'
    setTimeout(() => { copySuccess.value = '' }, 2000)
  } catch {
    copySuccess.value = 'failed'
    setTimeout(() => { copySuccess.value = '' }, 2000)
  }
}

const deployCode = computed(() => {
  if (!deployAgent.value) return ''
  const host = hubHost()
  const token = envToken(deployAgent.value.environment_id)
  const os = deployAgent.value.os || 'linux'
  const arch = deployAgent.value.arch || 'amd64'
  if (deployTab.value === 'binary') {
    return `# Download for ${os}/${arch}
curl -LO ${host}/api/agents/download?os=${os}&arch=${arch}
chmod +x rex-agent

# Register with your environment
./rex-agent register --server ${host} --token ${token}`
  }
  if (deployTab.value === 'docker') {
    return `docker run -d \\
  --name rex-agent \\
  -e REX_SERVER=${host} \\
  -e REX_TOKEN=${token} \\
  rex/rex-agent:latest`
  }
  if (deployTab.value === 'compose') {
    return `services:
  rex-agent:
    image: rex/rex-agent:latest
    environment:
      REX_SERVER: ${host}
      REX_TOKEN: ${token}
    restart: unless-stopped`
  }
  // config file
  return `# ~/.rex/config.toml
[agent]
server = "${host}"
token = "${token}"
auto_update = true`
})
const allDirectMode = computed(() => {
  return store.environments.length > 0 && store.environments.every(e => e.connection_mode === 'direct')
})


const filteredLogs = computed(() => {
  if (!logFilter.value) return logEntries.value
  const q = logFilter.value.toLowerCase()
  return logEntries.value.filter(e => e.action.toLowerCase().includes(q))
})
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
      :description="allDirectMode ? t('agents.directModeNote') : t('agents.noAgentsDesc')"
    >
      <Button v-if="!allDirectMode" @click="deployAgent = null; deployModal = true">{{ t('agents.quickStart') }}</Button>
    </EmptyState>

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
            <span>{{ agent.os || '—' }}/{{ agent.arch }}</span>
          </div>
          <div class="agent-detail">
            <span class="muted">{{ t('agents.lastSeen') }}</span>
            <span>{{ agent.last_seen_at ? new Date(agent.last_seen_at).toLocaleString() : '—' }}</span>
          </div>
        </div>
        <div class="agent-footer">
          <Button variant="secondary" size="sm" @click="openDeploy(agent)">{{ t('agents.deploy') }}</Button>
          <Button variant="secondary" size="sm" @click="openConfig(agent)">{{ t('agents.config') }}</Button>
          <Button variant="secondary" size="sm" @click="openLogs(agent)">{{ t('agents.logs') }}</Button>
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

    <!-- Log Viewer Modal -->
    <Modal v-model="logModal">
      <template #title>{{ t('agents.logTitle') }} — {{ logAgentName }}</template>
      <div class="log-content">
        <div class="log-filter">
          <input v-model="logFilter" class="form-input" :placeholder="t('agents.logFilter')" />
        </div>
        <div v-if="logLoading" class="log-loading">{{ t('common.loading') }}</div>
        <div v-else-if="filteredLogs.length === 0" class="log-empty">{{ t('agents.noLogs') }}</div>
        <div v-else class="log-list">
          <div v-for="entry in filteredLogs" :key="entry.id" class="log-entry">
            <span class="log-time mono">{{ new Date(entry.time).toLocaleString() }}</span>
            <span class="log-action" :class="`log-action--${entry.action.toLowerCase()}`">{{ entry.action }}</span>
            <span class="log-result" :class="entry.result === 'success' ? 'log-result--ok' : 'log-result--fail'">{{ entry.result }}</span>
            <span v-if="entry.detail" class="log-detail muted">{{ entry.detail }}</span>
          </div>
        </div>
      </div>
    </Modal>

    <!-- Deploy Guide Modal -->
    <Modal v-model="deployModal">
      <template #title>{{ t('agents.deployTitle') }} — {{ deployAgent?.name }}</template>
      <div v-if="deployAgent" class="deploy-content">
        <div class="deploy-tabs">
          <button class="deploy-tab" :class="{ active: deployTab === 'binary' }" @click="deployTab = 'binary'">{{ t('agents.deployBinary') }}</button>
          <button class="deploy-tab" :class="{ active: deployTab === 'docker' }" @click="deployTab = 'docker'">{{ t('agents.deployDocker') }}</button>
          <button class="deploy-tab" :class="{ active: deployTab === 'compose' }" @click="deployTab = 'compose'">{{ t('agents.deployCompose') }}</button>
          <button class="deploy-tab" :class="{ active: deployTab === 'config' }" @click="deployTab = 'config'">{{ t('agents.deployConfig') }}</button>
        </div>
        <div class="deploy-code">
          <div class="deploy-code-header">
            <span class="muted">{{ t('agents.deployCopyHint') }}</span>
            <Button variant="secondary" size="sm" @click="copyText(deployCode)">{{ copySuccess ? t('agents.deployCopied') : t('agents.deployCopy') }}</Button>
          </div>
          <pre class="mono"><code>{{ deployCode }}</code></pre>
        </div>
      </div>
    </Modal>

    <!-- Config Modal -->
    <Modal v-model="configModal">
      <template #title>{{ t('agents.configTitle') }} — {{ configAgent?.name }}</template>
      <div v-if="configAgent" class="config-content">
        <div class="config-section">
          <h3 class="config-section-title">{{ t('agents.configInfo') }}</h3>
          <div class="config-row">
            <span class="config-label muted">{{ t('agents.configEnvironment') }}</span>
            <span>{{ envName(configAgent.environment_id) }}</span>
          </div>
          <div class="config-row">
            <span class="config-label muted">Agent ID</span>
            <span class="mono">{{ configAgent.id }}</span>
          </div>
          <div class="config-row">
            <span class="config-label muted">{{ t('agents.version') }}</span>
            <span class="mono">{{ configAgent.version || '—' }}</span>
          </div>
          <div class="config-row">
            <span class="config-label muted">{{ t('agents.os') }}</span>
            <span>{{ configAgent.os || '—' }}/{{ configAgent.arch }}</span>
          </div>
          <div class="config-row">
            <span class="config-label muted">{{ t('agents.hostname') }}</span>
            <span>{{ configAgent.hostname || '—' }}</span>
          </div>
        </div>
        <div class="config-section">
          <h3 class="config-section-title">{{ t('agents.configSettings') }}</h3>
          <div class="config-row">
            <span class="config-label muted">{{ t('agents.configServer') }}</span>
            <span class="mono">{{ hubHost() }}</span>
          </div>
          <div class="config-row">
            <span class="config-label muted">{{ t('agents.configAutoUpdate') }}</span>
            <label class="toggle">
              <input type="checkbox" v-model="configAutoUpdate" />
              <span class="toggle-slider"></span>
            </label>
          </div>
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
.empty-hint {
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin-bottom: var(--space-3);
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
.log-content {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  max-height: 400px;
}
.log-filter .form-input {
  width: 100%;
  padding: 6px 10px;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-primary);
  font-size: var(--text-sm);
}
.log-loading, .log-empty {
  padding: var(--space-6);
  text-align: center;
  color: var(--text-muted);
  font-size: var(--text-sm);
}
.log-list {
  overflow-y: auto;
  max-height: 320px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.log-entry {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: 6px var(--space-3);
  border-radius: 4px;
  font-size: var(--text-sm);
  background: var(--bg-deep);
}
.log-entry:hover {
  background: var(--bg-hover);
}
.log-time {
  flex-shrink: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
}
.log-action {
  flex-shrink: 0;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: var(--text-xs);
  font-weight: 500;
  background: var(--bg-hover);
  color: var(--text-secondary);
}
.log-action--agent_online {
  background: rgba(63, 185, 80, 0.15);
  color: var(--success);
}
.log-action--agent_offline {
  background: rgba(248, 81, 73, 0.15);
  color: var(--danger);
}
.log-result {
  flex-shrink: 0;
  font-size: var(--text-xs);
}
.log-result--ok { color: var(--success); }
.log-result--fail { color: var(--danger); }
.log-detail {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--text-xs);
}
/* Deploy guide modal */
.deploy-content {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.deploy-tabs {
  display: flex;
  gap: 2px;
  background: var(--bg-deep);
  border-radius: 6px;
  padding: 2px;
}
.deploy-tab {
  flex: 1;
  padding: 6px 8px;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--text-muted);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: all 0.15s;
}
.deploy-tab:hover { color: var(--text-primary); }
.deploy-tab.active {
  background: var(--bg-hover);
  color: var(--text-primary);
  font-weight: 500;
}
.deploy-code {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}
.deploy-code-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border);
  font-size: var(--text-xs);
}
.deploy-code pre {
  margin: 0;
  padding: 12px;
  font-size: var(--text-xs);
  line-height: 1.6;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--text-primary);
}
/* Config modal */
.config-content {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.config-section-title {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: var(--space-2);
}
.config-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 0;
  font-size: var(--text-sm);
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border);
}
.config-row:last-child { border-bottom: none; }
.config-label {
  flex-shrink: 0;
  font-size: var(--text-xs);
  min-width: 100px;
}
/* Toggle switch */
.toggle {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 20px;
  cursor: pointer;
}
.toggle input { opacity: 0; width: 0; height: 0; }
.toggle-slider {
  position: absolute;
  inset: 0;
  background: var(--bg-hover);
  border-radius: 10px;
  transition: background 0.2s;
}
.toggle-slider::before {
  content: '';
  position: absolute;
  width: 16px;
  height: 16px;
  left: 2px;
  top: 2px;
  background: var(--text-muted);
  border-radius: 50%;
  transition: transform 0.2s, background 0.2s;
}
.toggle input:checked + .toggle-slider {
  background: var(--accent);
}
.toggle input:checked + .toggle-slider::before {
  transform: translateX(16px);
  background: #fff;
}
</style>
