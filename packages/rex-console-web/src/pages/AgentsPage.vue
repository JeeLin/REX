<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { agentsApi, type Agent, type AuditEntry } from '@/api/agents'
import { useEnvironmentsStore } from '@/stores/environments'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Modal from '@/components/ui/Modal.vue'
import Tabs from '@/components/ui/Tabs.vue'
import Badge from '@/components/ui/Badge.vue'
import Input from '@/components/ui/Input.vue'
import Switch from '@/components/ui/Switch.vue'
import Toast from '@/components/ui/Toast.vue'
import { agentStatus } from '@/utils/status'

const { t } = useI18n()
const store = useEnvironmentsStore()
const toast = ref<InstanceType<typeof Toast> | null>(null)
const agents = ref<Agent[]>([])
const loading = ref(true)
const resetModal = ref(false)
const resetAgentId = ref('')
const resetToken = ref('')
const resetLoading = ref(false)
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
  return env?.registration_token || 'YOUR_TOKEN'
}

function envName(envId: string): string {
  return store.environments.find(e => e.id === envId)?.name || envId
}

async function fetchAgents() {
  loading.value = true
  await store.fetchEnvironments()
  const results = await Promise.allSettled(
    store.environments.map(env => agentsApi.listByEnv(env.id))
  )
  agents.value = results
    .filter((r): r is PromiseFulfilledResult<Agent[]> => r.status === 'fulfilled')
    .flatMap(r => r.value)
  const failed = results.filter(r => r.status === 'rejected')
  if (failed.length > 0) {
    toast.value?.push(t('agents.fetchError', { count: failed.length }), 'error')
  }
  loading.value = false
}

onMounted(fetchAgents)

const hasAgents = computed(() => agents.value.length > 0)

async function openResetToken(agentId: string) {
  resetAgentId.value = agentId
  resetToken.value = ''
  resetModal.value = true
}

async function doResetToken() {
  resetLoading.value = true
  try {
    const result = await agentsApi.resetToken(resetAgentId.value)
    resetToken.value = result.token
  } catch {
    toast.value?.push(t('agents.resetFailed'), 'error')
  } finally {
    resetLoading.value = false
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
    toast.value?.push(t('agents.logFetchFailed'), 'error')
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
    if (!navigator.clipboard?.writeText) throw new Error('clipboard not available')
    await navigator.clipboard.writeText(text)
    copySuccess.value = 'copied'
    setTimeout(() => { copySuccess.value = '' }, 2000)
  } catch {
    // fallback for non-HTTPS environments
    const ta = document.createElement('textarea')
    ta.value = text
    ta.style.position = 'fixed'
    ta.style.left = '-9999px'
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
    copySuccess.value = 'copied'
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
  ghcr.io/jeelin/rex-agent:latest`
  }
  if (deployTab.value === 'compose') {
    return `services:
  rex-agent:
    image: ghcr.io/jeelin/rex-agent:latest
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

// Deployment guide
const guideExpanded = ref(true)
const guideTab = ref<'binary' | 'docker' | 'compose' | 'config'>('binary')
const guideCopySuccess = ref('')
const downloadArch = ref('linux-amd64')
const downloadUrl = computed(() => {
  const [os, arch] = downloadArch.value.split('-')
  return `${hubHost()}/api/agents/download?os=${os}&arch=${arch}`
})

watch(hasAgents, (val) => {
  if (val) guideExpanded.value = false
}, { immediate: true })

const guideCode = computed(() => {
  const host = hubHost()
  if (guideTab.value === 'binary') {
    return `# 1. Download the agent
curl -LO ${host}/api/agents/download?os=linux&arch=amd64
chmod +x rex-agent

# 2. Register with your environment
./rex-agent register --server ${host} --token YOUR_TOKEN`
  }
  if (guideTab.value === 'docker') {
    return `docker run -d \\
  --name rex-agent \\
  -e REX_SERVER=${host} \\
  -e REX_TOKEN=YOUR_TOKEN \\
  ghcr.io/jeelin/rex-agent:latest`
  }
  if (guideTab.value === 'compose') {
    return `services:
  rex-agent:
    image: ghcr.io/jeelin/rex-agent:latest
    environment:
      REX_SERVER: ${host}
      REX_TOKEN: YOUR_TOKEN
    restart: unless-stopped`
  }
  return `# ~/.rex/config.toml
[agent]
server = "${host}"
token = "YOUR_TOKEN"
auto_update = true`
})

function copyGuideCode() {
  copyText(guideCode.value)
  guideCopySuccess.value = 'copied'
  setTimeout(() => { guideCopySuccess.value = '' }, 2000)
}


const filteredLogs = computed(() => {
  if (!logFilter.value) return logEntries.value
  const q = logFilter.value.toLowerCase()
  return logEntries.value.filter(e => e.action.toLowerCase().includes(q))
})
</script>

<template>
  <div class="page-container agents-page">
    <header class="page-header">
      <div class="page-header-left">
        <h1 class="page-title mono">{{ t('agents.title') }}</h1>
        <span class="page-subtitle">{{ t('agents.subtitle', 'Registered agent nodes') }}</span>
      </div>
    </header>

    <!-- Deployment Guide — 始终可访问，无 Agent 时默认展开 -->
    <div v-if="!loading" class="guide-section">
      <button class="guide-toggle" @click="guideExpanded = !guideExpanded">
        <span class="guide-toggle-icon">{{ guideExpanded ? '▾' : '▸' }}</span>
        <span class="guide-toggle-title">{{ t('agents.guideTitle') }}</span>
        <span class="guide-toggle-desc muted">{{ t('agents.guideDesc') }}</span>
      </button>
      <div v-show="guideExpanded" class="guide-body">
        <div class="guide-steps">
          <div class="guide-step">
            <div class="guide-step-title">{{ t('agents.guideStep1') }}</div>
            <p class="guide-step-hint muted">{{ t('agents.guideDownloadHint') }}</p>
          </div>
          <div class="guide-step">
            <div class="guide-step-title">{{ t('agents.guideStep2') }}</div>
            <p class="guide-step-hint muted">{{ t('agents.guideTokenHint') }}</p>
          </div>
          <div class="guide-step">
            <div class="guide-step-title">{{ t('agents.guideStep3') }}</div>
            <p class="guide-step-hint muted">{{ t('agents.guideStartHint') }}</p>
          </div>
        </div>
        <div class="guide-tabs">
          <button class="guide-tab" :class="{ active: guideTab === 'binary' }" @click="guideTab = 'binary'">{{ t('agents.guideBinaryTab') }}</button>
          <button class="guide-tab" :class="{ active: guideTab === 'docker' }" @click="guideTab = 'docker'">{{ t('agents.guideDockerTab') }}</button>
          <button class="guide-tab" :class="{ active: guideTab === 'compose' }" @click="guideTab = 'compose'">{{ t('agents.guideComposeTab') }}</button>
          <button class="guide-tab" :class="{ active: guideTab === 'config' }" @click="guideTab = 'config'">{{ t('agents.guideConfigTab') }}</button>
        </div>
        <div v-if="guideTab === 'binary'" class="guide-download">
          <div class="guide-download-row">
            <select v-model="downloadArch" class="form-select">
              <option value="linux-amd64">Linux x86_64</option>
              <option value="linux-arm64">Linux ARM64</option>
              <option value="darwin-amd64">macOS x86_64</option>
              <option value="darwin-arm64">macOS ARM64</option>
              <option value="windows-amd64">Windows x86_64</option>
            </select>
            <a :href="downloadUrl" class="btn btn-primary" download>{{ t('agents.downloadBinary') }}</a>
          </div>
          <p class="muted" style="font-size: var(--text-xs); margin-top: var(--space-2)">{{ t('agents.downloadHint') }}</p>
        </div>
        <div v-else class="guide-code">
          <div class="guide-code-header">
            <span class="muted">{{ t('agents.deployCopyHint') }}</span>
            <Button variant="secondary" size="sm" @click="copyGuideCode">{{ guideCopySuccess ? t('agents.deployCopied') : t('agents.deployCopy') }}</Button>
          </div>
          <pre class="mono"><code>{{ guideCode }}</code></pre>
        </div>
      </div>
    </div>

    <!-- Agent 列表 -->
    <div v-if="hasAgents" class="agent-grid">
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
          <Badge :tone="agent.status === 'online' ? 'success' : agent.status === 'connecting' ? 'warning' : 'neutral'" size="sm">
            {{ agent.status }}
          </Badge>
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
          <Button variant="ghost" size="sm" @click="openConfig(agent)">{{ t('agents.config') }}</Button>
          <Button variant="ghost" size="sm" @click="openLogs(agent)">{{ t('agents.logs') }}</Button>
          <Button variant="ghost" size="sm" @click="openResetToken(agent.id)">{{ t('agents.resetToken') }}</Button>
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
          <Input v-model="logFilter" :placeholder="t('agents.logFilter')" size="sm" />
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
            <Switch v-model="configAutoUpdate" size="sm" />
          </div>
        </div>
      </div>
    </Modal>
  </div>
</template>

<style scoped>
.agents-page {}
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
.empty-hint {
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin-bottom: var(--space-3);
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
  background: var(--success-soft);
  color: var(--success);
}
.log-action--agent_offline {
  background: var(--danger-soft);
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
/* Deployment guide */
.guide-section {
  margin-bottom: var(--space-4);
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
}
.guide-toggle {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: var(--space-3) var(--space-4);
  background: var(--bg-deep);
  border: none;
  cursor: pointer;
  text-align: left;
  color: var(--text-primary);
}
.guide-toggle:hover {
  background: var(--bg-hover);
}
.guide-toggle-icon {
  font-size: var(--text-xs);
  color: var(--text-muted);
  flex-shrink: 0;
}
.guide-toggle-title {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
}
.guide-toggle-desc {
  font-size: var(--text-xs);
  margin-left: auto;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.guide-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-4);
  border-top: 1px solid var(--border);
}
.guide-steps {
  display: flex;
  gap: var(--space-4);
}
.guide-step {
  flex: 1;
  padding: var(--space-3);
  background: var(--bg-deep);
  border-radius: 6px;
}
.guide-step-title {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: var(--space-1);
}
.guide-step-hint {
  font-size: var(--text-xs);
  line-height: 1.4;
}
.guide-tabs {
  display: flex;
  gap: 2px;
  background: var(--bg-deep);
  border-radius: 6px;
  padding: 2px;
}
.guide-tab {
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
.guide-tab:hover { color: var(--text-primary); }
.guide-tab.active {
  background: var(--bg-hover);
  color: var(--text-primary);
  font-weight: 500;
}
.guide-code {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}
.guide-code-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border);
  font-size: var(--text-xs);
}
.guide-code pre {
  margin: 0;
  padding: 12px;
  font-size: var(--text-xs);
  line-height: 1.6;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--text-primary);
}
.guide-download {
  padding: var(--space-4);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 6px;
}
.guide-download-row {
  display: flex;
  gap: var(--space-3);
  align-items: center;
}
.guide-download .form-select {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 6px 12px;
  color: var(--text-primary);
  font-size: var(--text-sm);
}
.guide-download .btn {
  padding: 6px 16px;
  border-radius: 6px;
  font-size: var(--text-sm);
  font-weight: 500;
  cursor: pointer;
  text-decoration: none;
  display: inline-flex;
  align-items: center;
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
</style>
