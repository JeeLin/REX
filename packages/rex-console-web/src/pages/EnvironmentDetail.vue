<template>
  <div>
    <div class="env-detail-header">
      <router-link to="/environments" class="btn btn-ghost btn-sm">
        ← {{ t('common.back') }}
      </router-link>
      <h2 class="page-title">{{ env?.name || '...' }}</h2>
      <span v-if="env" class="badge" :class="env.connection_mode === 'direct' ? 'badge-info' : 'badge-success'">
        {{ env.connection_mode === 'direct' ? t('env.direct') : t('env.agentProxy') }}
      </span>
    </div>

    <div v-if="env?.description" class="env-desc">{{ env.description }}</div>

    <!-- Resources -->
    <div class="section-header">
      <h3 class="section-title">{{ t('env.resources') }}</h3>
      <router-link
        v-if="env"
        :to="`/environments/${env.id}/resources/new`"
        class="btn btn-primary btn-sm"
      >
        {{ t('env.addResource') }}
      </router-link>
    </div>

    <div v-if="resources.length === 0" class="empty-state">
      <p>{{ t('env.noResources') }}</p>
    </div>

    <div v-else class="resource-list">
      <div v-for="res in resources" :key="res.id" class="resource-item">
        <div class="resource-icon" :style="{ background: protocolColor(res.protocol) + '15', color: protocolColor(res.protocol) }">
          {{ protocolIcon(res.protocol) }}
        </div>
        <div class="resource-info">
          <div class="resource-name">{{ res.name }}</div>
          <div class="resource-proto">{{ res.protocol.toUpperCase() }}</div>
        </div>
        <span class="resource-status badge badge-success">{{ t('status.online') }}</span>
      </div>
    </div>

    <!-- Agent Status -->
    <AgentStatusPanel v-if="env" :env-id="env.id" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import client from '@/api/client'
import AgentStatusPanel from '@/features/agents/AgentStatusPanel.vue'

const { t } = useI18n()
const route = useRoute()

interface Environment {
  id: string
  name: string
  description: string | null
  connection_mode: string
  created_at: string
  updated_at: string
}

interface Resource {
  id: string
  name: string
  protocol: string
  connection_mode: string
  status: string
}

const env = ref<Environment | null>(null)
const resources = ref<Resource[]>([])

const PROTOCOL_ICONS: Record<string, string> = {
  ssh: '$', sftp: '📁', mysql: 'dB', postgresql: 'pg',
  redis: 'R', docker: '🐳', sqlite: 'S', s3: '☁',
}
const PROTOCOL_COLORS: Record<string, string> = {
  ssh: 'var(--success)', sftp: 'var(--accent-purple)', mysql: 'var(--info)',
  postgresql: 'var(--accent-purple)', redis: 'var(--danger)', docker: 'var(--info)',
  sqlite: 'var(--warning)', s3: 'var(--accent)',
}

function protocolIcon(p: string) { return PROTOCOL_ICONS[p] || '·' }
function protocolColor(p: string) { return PROTOCOL_COLORS[p] || 'var(--text-secondary)' }

onMounted(async () => {
  const id = route.params.id as string
  try {
    const { data } = await client.get<{ data: Environment }>(`/environments/${id}`)
    env.value = data.data
    const resResp = await client.get<{ data: Resource[] }>(`/environments/${id}/resources`)
    resources.value = resResp.data.data
  } catch {
    // 静默处理
  }
})
</script>

<style scoped>
.env-detail-header {
  display: flex;
  align-items: center;
  gap: var(--sp-lg);
  margin-bottom: var(--sp-md);
}

.env-desc {
  color: var(--text-secondary);
  font-size: var(--fs-sm);
  margin-bottom: var(--sp-xl);
}

.empty-state {
  text-align: center;
  padding: var(--sp-3xl);
  color: var(--text-secondary);
}

.resource-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
}

.resource-item {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
  padding: var(--sp-md) var(--sp-lg);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
}

.resource-item:hover {
  border-color: rgba(232, 145, 45, 0.2);
  background: var(--bg-elevated);
}

.resource-icon {
  width: 36px;
  height: 36px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: var(--fs-sm);
  flex-shrink: 0;
}

.resource-info {
  flex: 1;
  min-width: 0;
}

.resource-name {
  font-family: var(--font-mono);
  font-weight: 500;
  font-size: var(--fs-base);
}

.resource-proto {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.badge-info {
  color: var(--info);
}
</style>
