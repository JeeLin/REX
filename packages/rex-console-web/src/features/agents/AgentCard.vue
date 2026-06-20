<template>
  <div class="agent-card" :class="{ offline: agent.status !== 'online' }" @contextmenu.prevent="onCtx">
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
        <span class="info-value mono">
          <AgentVersionBadge :version="agent.version" :needs-update="needsUpdate()" />
        </span>
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

    <div class="card-actions">
      <button class="btn btn-ghost btn-sm" @click.stop="$emit('openConfig')">{{ t('ctx.config') }}</button>
      <button class="btn btn-ghost btn-sm" @click.stop="$emit('openLog')">{{ t('ctx.viewLog') }}</button>
      <button class="btn btn-danger btn-sm" @click.stop="$emit('resetToken')">{{ t('ctx.resetToken') }}</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'
import type { Agent } from '@/api/agent'
import AgentVersionBadge from './AgentVersionBadge.vue'

const props = defineProps<{ agent: Agent; hubVersion?: string }>()
const emit = defineEmits<{ openConfig: []; openLog: []; resetToken: [] }>()
const { show: showMenu } = useContextMenu()

function onCtx(e: MouseEvent) {
  showMenu(e, [
    { label: t('ctx.config'), action: () => emit('openConfig') },
    { label: t('ctx.viewLog'), action: () => emit('openLog') },
    { separator: true },
    { label: t('ctx.copyAgentId'), action: () => navigator.clipboard?.writeText(props.agent.id) },
    { label: t('ctx.copyToken') },
    { separator: true },
    { label: t('ctx.restartAgent'), danger: true },
    { label: t('ctx.resetToken'), danger: true, action: () => emit('resetToken') },
  ])
}

function needsUpdate(): boolean {
  return !!(props.hubVersion && props.agent.version !== props.hubVersion)
}

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

.card-actions {
  display: flex;
  gap: var(--sp-sm);
  margin-top: var(--sp-md);
  padding-top: var(--sp-md);
  border-top: 1px solid var(--border);
}

.btn {
  padding: var(--sp-xs) var(--sp-md);
  border-radius: var(--radius-sm);
  font-size: var(--fs-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
  border: 1px solid var(--border);
  background: none;
  color: var(--text-secondary);
}

.btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.btn-sm {
  padding: var(--sp-xs) var(--sp-sm);
}

.btn-danger {
  color: var(--danger);
  border-color: transparent;
}

.btn-danger:hover {
  background: rgba(248, 81, 73, 0.1);
}
</style>
