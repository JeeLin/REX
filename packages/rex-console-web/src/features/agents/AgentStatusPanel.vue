<template>
  <div class="agent-status-panel">
    <div class="section-header">
      <h3 class="section-title">{{ t('env.agents') }}</h3>
    </div>

    <div v-if="agents.length === 0" class="empty-state">
      <p>{{ t('agent.noAgents') }}</p>
      <p class="empty-hint">{{ t('agent.deployHint') }}</p>
    </div>

    <div v-else class="agent-list">
      <div v-for="agent in agents" :key="agent.id" class="agent-item">
        <div class="agent-status-dot" :class="agent.status === 'online' ? 'online' : 'offline'" />
        <div class="agent-info">
          <div class="agent-name">{{ agent.name }}</div>
          <div class="agent-meta">
            <span class="agent-os">{{ osIcon(agent.os) }} {{ agent.os || '—' }}</span>
            <span v-if="agent.os_version" class="agent-os-version">{{ agent.os_version }}</span>
            <span class="agent-arch">{{ agent.arch }}</span>
          </div>
        </div>
        <div class="agent-right">
          <span class="badge" :class="agent.status === 'online' ? 'badge-success' : 'badge-offline'">
            {{ agent.status === 'online' ? t('status.online') : t('status.offline') }}
          </span>
          <span class="agent-version">v{{ agent.version }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAgentStore } from '@/stores/agent'
import type { Agent } from '@/api/agent'

const props = defineProps<{ envId: string }>()

const { t } = useI18n()
const agentStore = useAgentStore()

const agents = computed<Agent[]>(() => agentStore.getAgents(props.envId))

function osIcon(os: string): string {
  if (os === 'linux') return '🐧'
  if (os === 'macos' || os === 'darwin') return '🍎'
  if (os === 'windows') return '🪟'
  return '💻'
}

onMounted(() => {
  agentStore.fetchAgents(props.envId)
})

watch(() => props.envId, (id) => {
  if (id) agentStore.fetchAgents(id)
})
</script>

<style scoped>
.agent-status-panel {
  margin-top: var(--sp-xl);
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--sp-md);
}

.section-title {
  font-size: var(--fs-base);
  font-weight: 600;
}

.empty-state {
  text-align: center;
  padding: var(--sp-xl);
  color: var(--text-secondary);
}

.empty-hint {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  margin-top: var(--sp-xs);
}

.agent-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
}

.agent-item {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
  padding: var(--sp-md) var(--sp-lg);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
}

.agent-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.agent-status-dot.online {
  background: var(--success);
  box-shadow: 0 0 6px var(--success);
}

.agent-status-dot.offline {
  background: var(--text-muted);
}

.agent-info {
  flex: 1;
  min-width: 0;
}

.agent-name {
  font-weight: 500;
  font-size: var(--fs-sm);
}

.agent-meta {
  display: flex;
  gap: var(--sp-sm);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  margin-top: 2px;
}

.agent-right {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  flex-shrink: 0;
}

.agent-version {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.badge {
  padding: 2px var(--sp-sm);
  border-radius: var(--radius-sm);
  font-size: var(--fs-xs);
  font-weight: 500;
}

.badge-success {
  color: var(--success);
  background: rgba(34, 197, 94, 0.1);
}

.badge-offline {
  color: var(--text-muted);
  background: var(--bg-elevated);
}
</style>
