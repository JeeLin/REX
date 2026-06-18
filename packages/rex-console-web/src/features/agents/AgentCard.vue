<template>
  <div class="agent-card" :class="{ offline: agent.status !== 'online' }">
    <div class="card-header">
      <div class="status-dot" :class="agent.status === 'online' ? 'online' : 'offline'" />
      <span class="agent-name">{{ agent.name }}</span>
      <span class="badge" :class="agent.status === 'online' ? 'badge-success' : 'badge-offline'">
        {{ agent.status === 'online' ? t('status.online') : t('status.offline') }}
      </span>
    </div>

    <div class="card-body">
      <div class="info-row">
        <span class="info-label">{{ t('agent.device') }}</span>
        <span class="info-value">{{ osIcon(agent.os) }} {{ agent.os }} / {{ agent.arch }}</span>
      </div>
      <div v-if="agent.os_version" class="info-row">
        <span class="info-label">{{ t('env.connectionModeLabel') }}</span>
        <span class="info-value">{{ agent.os_version }}</span>
      </div>
      <div class="info-row">
        <span class="info-label">{{ t('agent.version') }}</span>
        <span class="info-value mono">v{{ agent.version }}</span>
      </div>
      <div class="info-row">
        <span class="info-label">{{ t('agent.agentId') }}</span>
        <span class="info-value mono">{{ agent.id }}</span>
      </div>
      <div class="info-row">
        <span class="info-label">{{ t('agent.environment') }}</span>
        <span class="info-value mono">{{ agent.environment_id }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Agent } from '@/api/agent'

defineProps<{ agent: Agent }>()

const { t } = useI18n()

function osIcon(os: string): string {
  if (os === 'linux') return '🐧'
  if (os === 'macos' || os === 'darwin') return '🍎'
  if (os === 'windows') return '🪟'
  return '💻'
}
</script>

<style scoped>
.agent-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--sp-lg);
  transition: all var(--transition-fast);
}

.agent-card:hover {
  border-color: rgba(232, 145, 45, 0.2);
}

.agent-card.offline {
  opacity: 0.7;
}

.card-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  margin-bottom: var(--sp-md);
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-dot.online {
  background: var(--success);
  box-shadow: 0 0 6px var(--success);
}

.status-dot.offline {
  background: var(--text-muted);
}

.agent-name {
  font-weight: 600;
  font-size: var(--fs-base);
}

.card-body {
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
}

.info-row {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  font-size: var(--fs-sm);
}

.info-label {
  color: var(--text-muted);
  min-width: 70px;
}

.info-value {
  color: var(--text-secondary);
}

.mono {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
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
