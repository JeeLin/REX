<template>
  <div>
    <!-- Loading -->
    <LoadingSpinner v-if="loading" :text="t('common.loading')" />

    <!-- Error -->
    <ErrorState v-else-if="loadError" :message="loadError" :retry="loadData" />

    <!-- Content -->
    <template v-else>
      <!-- Stats -->
      <div class="stats-row">
        <div class="stat-card" @contextmenu.prevent="onStatCardCtx($event)">
          <div class="stat-label">{{ t('dashboard.envCount') }}</div>
          <div class="stat-value" style="color: var(--accent)">{{ envCount }}</div>
        </div>
        <div class="stat-card" @contextmenu.prevent="onStatCardCtx($event)">
          <div class="stat-label">{{ t('dashboard.resCount') }}</div>
          <div class="stat-value" style="color: var(--info)">{{ resourceCount }}</div>
        </div>
        <div class="stat-card" @contextmenu.prevent="onStatCardCtx($event)">
          <div class="stat-label">{{ t('dashboard.agentOnline') }}</div>
          <div class="stat-value" style="color: var(--success)">{{ agentOnlineCount }}</div>
        </div>
        <div class="stat-card" @contextmenu.prevent="onStatCardCtx($event)">
          <div class="stat-label">{{ t('dashboard.todayOps') }}</div>
          <div class="stat-value">{{ todayOps }}</div>
        </div>
      </div>

      <!-- Environments -->
      <div class="section-header">
        <h2 class="section-title">{{ t('dashboard.environments') }}</h2>
        <span class="text-sm text-secondary">{{ envCount }} {{ t('dashboard.envCountLabel') }}</span>
      </div>
      <div class="env-grid">
        <router-link
          v-for="env in environments"
          :key="env.id"
          :to="`/environments/${env.id}`"
          class="env-card"
          @contextmenu.prevent="onEnvCardCtx($event, env)"
        >
          <div class="env-card-header">
            <span class="env-card-name">{{ env.name }}</span>
            <span class="badge" :class="env.connection_mode === 'direct' ? 'badge-info' : (env.agent_online ? 'badge-success' : 'badge-offline')">
              {{ env.connection_mode === 'direct' ? t('env.connectionModeLabel') : (env.agent_online ? t('status.online') : t('status.offline')) }}
            </span>
          </div>
          <div class="env-card-desc">{{ env.description || '—' }}</div>
          <div v-if="env.resources.length > 0" class="env-card-badges">
            <span
              v-for="(count, proto) in getResourceStats(env)"
              :key="proto"
              class="res-badge"
              :style="{ background: getProtocolIcon(proto as string).color + '18', color: getProtocolIcon(proto as string).color }"
            >
              {{ getProtocolIcon(proto as string).icon }} {{ proto.toUpperCase() }} ×{{ count }}
            </span>
          </div>
          <div class="env-card-footer">
            <span>{{ env.connection_mode === 'direct' ? t('env.direct') : t('env.agentProxy') }}</span>
          </div>
        </router-link>

        <router-link to="/environments/new" class="add-env-card">
          <div class="add-icon">+</div>
          <div>{{ t('dashboard.createEnv') }}</div>
        </router-link>
      </div>

      <!-- Quick Connect -->
      <div v-if="allResources.length > 0" class="section-header" style="margin-top: var(--sp-xl)">
        <h2 class="section-title">{{ t('dashboard.quickConnect') }}</h2>
      </div>
      <div v-if="allResources.length > 0" class="quick-connect-grid">
        <button
          v-for="item in allResources.slice(0, 8)"
          :key="item.resource.id"
          class="quick-card"
          @click="connectToResource(item.resource, item.envName)"
          @contextmenu.prevent="onQuickConnectCtx($event, item)"
        >
          <div class="quick-icon" :style="{ background: getProtocolIcon(item.resource.protocol).color + '20', color: getProtocolIcon(item.resource.protocol).color }">
            {{ getProtocolIcon(item.resource.protocol).icon }}
          </div>
          <div class="quick-info">
            <div class="quick-name">{{ item.resource.name }}</div>
            <div class="quick-proto">{{ item.resource.protocol }}</div>
          </div>
        </button>
      </div>

      <!-- Recent Used -->
      <div v-if="recent.length > 0" class="section-header" style="margin-top: var(--sp-xl)">
        <h2 class="section-title">{{ t('dashboard.recentUsed') }}</h2>
      </div>
      <div v-if="recent.length > 0" class="recent-list">
        <button
          v-for="item in recent"
          :key="item.resourceId"
          class="recent-item"
          @click="connectToResource({ id: item.resourceId, protocol: item.protocol, name: item.name }, item.envName)"
        >
          <span class="recent-dot" :style="{ background: getProtocolIcon(item.protocol).color }"></span>
          <span class="recent-name">{{ item.name }}</span>
          <span class="recent-proto">{{ item.protocol }}</span>
          <span class="recent-time">{{ formatTime(item.usedAt) }}</span>
        </button>
      </div>

      <!-- Empty -->
      <EmptyState
        v-if="environments.length === 0"
        icon="🏠"
        :title="t('common.noData')"
        :action="{ label: t('dashboard.createEnv'), handler: () => router.push('/environments/new') }"
      />
    </template>

    <!-- Edit Modal -->
    <EnvironmentEditModal
      v-model:visible="editModalVisible"
      :env-id="editingEnvId"
    />

    <!-- Delete Confirm -->
    <ConfirmDialog
      :visible="showDeleteConfirm"
      :title="t('env.deleteTitle')"
      :message="t('env.deleteConfirm')"
      :danger="true"
      @confirm="confirmDeleteEnv"
      @cancel="showDeleteConfirm = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useRecent } from '@/composables/useRecent'
import { useSidebar } from '@/composables/useSidebar'
import { getProtocolIcon, useProtocol } from '@/composables/useProtocol'
import { useContextMenu } from '@/composables/useContextMenu'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import ErrorState from '@/components/ErrorState.vue'
import EmptyState from '@/components/EmptyState.vue'
import EnvironmentEditModal from '@/components/EnvironmentEditModal.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import type { EnvWithResources } from '@/api/env'
import { listEnvsWithResources, deleteEnvironment } from '@/api/env'
import { getAuditStats } from '@/api/audit'
import { fetchHealth } from '@/api/health'

const router = useRouter()
const { t } = useI18n()
const { recent, removeRecent } = useRecent()
const { connectToResource } = useProtocol()
const { addFavorite } = useSidebar()
const { show: showMenu } = useContextMenu()

const environments = ref<EnvWithResources[]>([])
const envCount = ref(0)
const resourceCount = ref(0)
const agentOnlineCount = ref(0)
const todayOps = ref(0)
const allResources = ref<{ resource: { id: string; name: string; protocol: string }; envName: string }[]>([])
const loading = ref(true)
const loadError = ref('')
const editModalVisible = ref(false)
const editingEnvId = ref('')
const showDeleteConfirm = ref(false)
const deletingEnvId = ref('')

function getResourceStats(env: EnvWithResources): Record<string, number> {
  const stats: Record<string, number> = {}
  for (const r of env.resources) {
    stats[r.protocol] = (stats[r.protocol] || 0) + 1
  }
  return stats
}

function formatTime(ts: number): string {
  const diff = Date.now() - ts
  const mins = Math.floor(diff / 60000)
  if (mins < 60) return `${mins}m ago`
  const hrs = Math.floor(mins / 60)
  if (hrs < 24) return `${hrs}h ago`
  const days = Math.floor(hrs / 24)
  return `${days}d ago`
}

function onQuickConnectCtx(e: MouseEvent, item: { resource: { id: string; name: string; protocol: string }; envName: string }) {
  showMenu(e, [
    { label: t('ctx.connect'), action: () => connectToResource(item.resource, item.envName) },
    { label: t('ctx.connectNewTab'), action: () => connectToResource(item.resource, item.envName) },
    { separator: true },
    { label: t('ctx.copyAddress'), action: () => navigator.clipboard?.writeText(`${item.resource.name} (${item.envName})`) },
    { label: t('ctx.addFavorite'), action: () => addFavorite(item.resource.id) },
    { separator: true },
    { label: t('ctx.removeRecent'), danger: true, action: () => removeRecent(item.resource.id) },
  ])
}

function onEnvCardCtx(e: MouseEvent, env: EnvWithResources) {
  showMenu(e, [
    { label: t('ctx.openDetail'), action: () => router.push(`/environments/${env.id}`) },
    { label: t('ctx.newResource'), action: () => router.push(`/environments/${env.id}/resources/new`) },
    { label: t('ctx.addAgent'), action: () => router.push('/agents') },
    { separator: true },
    { label: t('ctx.editEnv'), action: () => openEditModal(env) },
    { label: t('ctx.deleteEnv'), danger: true, action: () => requestDeleteEnv(env) },
  ])
}

function openEditModal(env: EnvWithResources) {
  editingEnvId.value = env.id
  editModalVisible.value = true
}

function requestDeleteEnv(env: EnvWithResources) {
  deletingEnvId.value = env.id
  showDeleteConfirm.value = true
}

async function confirmDeleteEnv() {
  try {
    await deleteEnvironment(deletingEnvId.value)
    environments.value = environments.value.filter(e => e.id !== deletingEnvId.value)
    envCount.value = environments.value.length
  } catch {
    // silent
  } finally {
    showDeleteConfirm.value = false
    deletingEnvId.value = ''
  }
}

function onStatCardCtx(e: MouseEvent) {
  showMenu(e, [
    { label: t('ctx.refreshStats'), action: () => location.reload() },
  ])
}

onMounted(() => loadData())

async function loadData() {
  loading.value = true
  loadError.value = ''
  try {
    const envsWithRes = await listEnvsWithResources()
    environments.value = envsWithRes
    envCount.value = envsWithRes.length
    const allRes: { resource: { id: string; name: string; protocol: string }; envName: string }[] = []
    for (const env of envsWithRes) {
      for (const res of env.resources) {
        allRes.push({ resource: res, envName: env.name })
      }
    }
    allResources.value = allRes
    resourceCount.value = allRes.length

    const stats = await getAuditStats('today')
    todayOps.value = stats.total

    const health = await fetchHealth()
    agentOnlineCount.value = health.connections.agents_online
  } catch {
    loadError.value = t('dashboard.loadFailed')
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.env-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  cursor: pointer;
  transition: all var(--transition-base);
  text-decoration: none;
  color: inherit;
  display: block;
}

.env-card:hover {
  border-color: rgba(232, 145, 45, 0.2);
  transform: translateY(-2px);
  box-shadow: var(--shadow-glow);
  text-decoration: none;
}

.env-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--sp-md);
}

.env-card-name {
  font-family: var(--font-mono);
  font-weight: 600;
  font-size: var(--fs-lg);
}

.env-card-desc {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  margin-bottom: var(--sp-lg);
}

.env-card-badges {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-xs);
  margin-bottom: var(--sp-md);
}

.res-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  font-size: 11px;
  font-family: var(--font-mono);
  font-weight: 500;
  white-space: nowrap;
}

.env-card-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: var(--sp-md);
  border-top: 1px solid var(--border);
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.add-env-card {
  background: var(--bg-surface);
  border: 2px dashed var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-md);
  cursor: pointer;
  transition: all var(--transition-base);
  min-height: 200px;
  color: var(--text-muted);
  text-decoration: none;
}

.add-env-card:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: rgba(232, 145, 45, 0.03);
  text-decoration: none;
  box-shadow: 0 0 20px rgba(232, 145, 45, 0.06);
}

.add-icon {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  border: 2px solid currentColor;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  transition: box-shadow var(--transition-base);
}

.add-env-card:hover .add-icon {
  box-shadow: 0 0 16px var(--accent-glow);
}

.badge-info { color: var(--info); }
.text-sm { font-size: var(--fs-sm); }
.text-secondary { color: var(--text-secondary); }

/* Quick Connect */
.quick-connect-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: var(--sp-sm);
}

.quick-card {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-md);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
  text-align: left;
  color: inherit;
}

.quick-card:hover {
  border-color: var(--accent);
  background: var(--bg-elevated);
}

.quick-icon {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: var(--fs-sm);
  flex-shrink: 0;
}

.quick-info { min-width: 0; }
.quick-name {
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.quick-proto {
  font-size: 10px;
  color: var(--text-muted);
  text-transform: uppercase;
}

/* Recent */
.recent-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.recent-item {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-sm) var(--sp-md);
  border: none;
  background: none;
  cursor: pointer;
  border-radius: var(--radius-md);
  transition: background var(--transition-fast);
  color: var(--text-secondary);
  font-size: var(--fs-sm);
}

.recent-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.recent-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.recent-name {
  flex: 1;
  text-align: left;
  font-family: var(--font-mono);
}

.recent-proto {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-muted);
  text-transform: uppercase;
}

.recent-time {
  font-size: 11px;
  color: var(--text-muted);
}
</style>
