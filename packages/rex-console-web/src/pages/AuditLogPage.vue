<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { auditApi, type AuditEntry, type AuditStats } from '@/api/audit'
import { useEnvironmentsStore } from '@/stores/environments'
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import EmptyState from '@/components/ui/EmptyState.vue'

const { t } = useI18n()
const store = useEnvironmentsStore()

const entries = ref<AuditEntry[]>([])
const loading = ref(true)
const stats = ref<AuditStats>({ total: 0, success_count: 0, failure_count: 0 })
const statsLoading = ref(false)
const expandedId = ref<string | null>(null)

// Context menu
const ctxMenu = ref({ show: false, x: 0, y: 0, entry: null as AuditEntry | null })

function onContextMenu(e: MouseEvent, entry: AuditEntry) {
  e.preventDefault()
  ctxMenu.value = { show: true, x: e.clientX, y: e.clientY, entry }
}

function closeCtxMenu() {
  ctxMenu.value.show = false
}

function ctxViewDetail() {
  if (ctxMenu.value.entry) toggleExpand(ctxMenu.value.entry.id)
  closeCtxMenu()
}

async function ctxCopyRecord() {
  if (ctxMenu.value.entry) {
    await navigator.clipboard.writeText(JSON.stringify(ctxMenu.value.entry, null, 2))
  }
  closeCtxMenu()
}

function ctxFilterByType() {
  if (ctxMenu.value.entry) actionFilter.value = ctxMenu.value.entry.action
  closeCtxMenu()
}

function ctxFilterByEnv() {
  if (ctxMenu.value.entry) environmentFilter.value = ctxMenu.value.entry.environment_id || ''
  closeCtxMenu()
}

function ctxRefresh() {
  refreshAll()
  closeCtxMenu()
}

function ctxExportCsv() {
  exportCsv()
  closeCtxMenu()
}

function ctxClearFilters() {
  actionFilter.value = ''
  resultFilter.value = ''
  environmentFilter.value = ''
  timeRange.value = 'all'
  closeCtxMenu()
}

// Filters
const actionFilter = ref('')
const resultFilter = ref('')
const environmentFilter = ref('')
const timeRange = ref('all')

const actionOptions = [
  { label: 'auditLog.all', value: '' },
  { label: 'ENV_CREATE', value: 'ENV_CREATE' },
  { label: 'ENV_UPDATE', value: 'ENV_UPDATE' },
  { label: 'ENV_DELETE', value: 'ENV_DELETE' },
  { label: 'RESOURCE_CREATE', value: 'RESOURCE_CREATE' },
  { label: 'RESOURCE_DELETE', value: 'RESOURCE_DELETE' },
  { label: 'AGENT_REGISTER', value: 'AGENT_REGISTER' },
  { label: 'AGENT_ONLINE', value: 'AGENT_ONLINE' },
  { label: 'AGENT_OFFLINE', value: 'AGENT_OFFLINE' },
  { label: 'SSH_CONNECT', value: 'SSH_CONNECT' },
  { label: 'SQL_QUERY', value: 'SQL_QUERY' },
  { label: 'REDIS_COMMAND', value: 'REDIS_COMMAND' },
  { label: 'FILE_OPERATION', value: 'FILE_OPERATION' },
  { label: 'AUTH_LOGIN', value: 'AUTH_LOGIN' },
  { label: 'AUTH_LOGOUT', value: 'AUTH_LOGOUT' },
]

const timeRangeOptions = [
  { label: 'auditLog.timeAll', value: 'all' },
  { label: 'auditLog.timeToday', value: 'today' },
  { label: 'auditLog.time7days', value: '7days' },
  { label: 'auditLog.time30days', value: '30days' },
]

function getTimeRange(): { time_from?: string; time_to?: string } {
  if (timeRange.value === 'all') return {}
  const now = new Date()
  if (timeRange.value === 'today') {
    const start = new Date(now.getFullYear(), now.getMonth(), now.getDate())
    return { time_from: start.toISOString() }
  }
  const days = timeRange.value === '7days' ? 7 : 30
  const start = new Date(now.getTime() - days * 86400000)
  return { time_from: start.toISOString() }
}

async function fetchEntries() {
  loading.value = true
  const range = getTimeRange()
  try {
    entries.value = await auditApi.query({
      action: actionFilter.value || undefined,
      result: resultFilter.value || undefined,
      environment_id: environmentFilter.value || undefined,
      ...range,
      limit: 100,
    })
  } catch {
    entries.value = []
  } finally {
    loading.value = false
  }
}

async function fetchStats() {
  statsLoading.value = true
  const range = getTimeRange()
  try {
    stats.value = await auditApi.stats({
      action: actionFilter.value || undefined,
      environment_id: environmentFilter.value || undefined,
      ...range,
    })
  } catch {
    stats.value = { total: 0, success_count: 0, failure_count: 0 }
  } finally {
    statsLoading.value = false
  }
}

function refreshAll() {
  fetchEntries()
  fetchStats()
}

function toggleExpand(id: string) {
  expandedId.value = expandedId.value === id ? null : id
}

function exportCsv() {
  const headers = ['time', 'action', 'target', 'environment_id', 'resource_id', 'agent_id', 'result', 'detail']
  const rows = entries.value.map(e => headers.map(h => {
    const val = (e as Record<string, unknown>)[h]
    const str = val === null || val === undefined ? '' : String(val)
    return `"${str.replace(/"/g, '""')}"`
  }).join(','))
  const csv = [headers.join(','), ...rows].join('\n')
  const blob = new Blob([csv], { type: 'text/csv' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `audit-log-${new Date().toISOString().slice(0, 10)}.csv`
  a.click()
  URL.revokeObjectURL(url)
}

function resultBadge(result: string) {
  return result === 'success' ? 'success' : 'danger'
}

function envName(id: string | null): string {
  if (!id) return '—'
  return store.environments.find(e => e.id === id)?.name || id
}
function resourceName(resId: string): string {
  for (const resources of store.envResources.values()) {
    const r = resources.find(r => r.id === resId)
    if (r) return r.name
  }
  return resId.slice(0, 8) + '…'
}


function timeAgo(time: string): string {
  const diff = Date.now() - new Date(time).getTime()
  const mins = Math.floor(diff / 60000)
  if (mins < 1) return t('auditLog.justNow')
  if (mins < 60) return t('auditLog.minutesAgo', { n: mins })
  const hours = Math.floor(mins / 60)
  if (hours < 24) return t('auditLog.hoursAgo', { n: hours })
  return t('auditLog.daysAgo', { n: Math.floor(hours / 24) })
}

function formatDetail(detail: string | null): string {
  if (!detail) return ''
  try {
    return JSON.stringify(JSON.parse(detail), null, 2)
  } catch {
    return detail
  }
}

function isJsonDetail(detail: string | null): boolean {
  if (!detail) return false
  try {
    JSON.parse(detail)
    return true
  } catch {
    return false
  }
}

watch([actionFilter, resultFilter, environmentFilter, timeRange], refreshAll)

onMounted(async () => {
  await store.fetchEnvironments()
  // Load resources for all environments to enable name resolution
  await Promise.all(store.environments.map(e => store.fetchResources(e.id)))
  refreshAll()
})
</script>

<template>
  <div class="audit-page">
    <header class="page-header">
      <h1 class="page-title">{{ t('auditLog.title') }}</h1>
      <div class="header-actions">
        <Button variant="secondary" size="sm" @click="exportCsv">{{ t('auditLog.exportCsv') }}</Button>
        <Button variant="secondary" size="sm" @click="refreshAll">{{ t('common.refresh') }}</Button>
      </div>
    </header>

    <!-- Stats cards -->
    <div class="stats-row">
      <Card class="stat-card">
        <div class="stat-value" :class="{ loading: statsLoading }">{{ statsLoading ? '—' : stats.total }}</div>
        <div class="stat-label muted">{{ t('auditLog.statTotal') }}</div>
      </Card>
      <Card class="stat-card stat-card--success">
        <div class="stat-value stat-value--success" :class="{ loading: statsLoading }">{{ statsLoading ? '—' : stats.success_count }}</div>
        <div class="stat-label muted">{{ t('auditLog.statSuccess') }}</div>
      </Card>
      <Card class="stat-card stat-card--failure">
        <div class="stat-value stat-value--failure" :class="{ loading: statsLoading }">{{ statsLoading ? '—' : stats.failure_count }}</div>
        <div class="stat-label muted">{{ t('auditLog.statFailure') }}</div>
      </Card>
    </div>

    <!-- Filters -->
    <div class="filters">
      <select v-model="actionFilter" class="filter-select">
        <option v-for="opt in actionOptions" :key="opt.value" :value="opt.value">{{ opt.value ? opt.value : t(opt.label) }}</option>
      </select>
      <select v-model="resultFilter" class="filter-select">
        <option value="">{{ t('auditLog.allResults') }}</option>
        <option value="success">{{ t('auditLog.success') }}</option>
        <option value="failure">{{ t('auditLog.failure') }}</option>
      </select>
      <select v-model="environmentFilter" class="filter-select">
        <option value="">{{ t('auditLog.allEnvironments') }}</option>
        <option v-for="env in store.environments" :key="env.id" :value="env.id">{{ env.name }}</option>
      </select>
      <select v-model="timeRange" class="filter-select">
        <option v-for="opt in timeRangeOptions" :key="opt.value" :value="opt.value">{{ t(opt.label) }}</option>
      </select>
    </div>

    <EmptyState
      v-if="!loading && entries.length === 0"
      icon="📋"
      :title="t('auditLog.noEntries')"
      :description="t('auditLog.emptyDesc')"
    />

    <Card v-else class="log-card">
      <div v-if="loading" class="loading muted">{{ t('common.loadingEllipsis') }}</div>
      <table v-else class="log-table">
        <thead>
          <tr>
            <th></th>
            <th>{{ t('auditLog.time') }}</th>
            <th>{{ t('auditLog.action') }}</th>
            <th>{{ t('auditLog.target') }}</th>
            <th>{{ t('auditLog.environment') }}</th>
            <th>{{ t('auditLog.result') }}</th>
          </tr>
        </thead>
        <tbody>
          <template v-for="entry in entries" :key="entry.id">
            <tr class="log-row" @click="toggleExpand(entry.id)" @contextmenu.prevent="onContextMenu($event, entry)">
              <td class="expand-icon">{{ expandedId === entry.id ? '▾' : '▸' }}</td>
              <td class="mono">{{ timeAgo(entry.time) }}</td>
              <td>
                <Badge :tone="entry.action.includes('DELETE') ? 'danger' : entry.action.includes('CREATE') ? 'success' : entry.action.includes('ONLINE') ? 'success' : entry.action.includes('OFFLINE') ? 'danger' : 'info'">
                  {{ entry.action }}
                </Badge>
              </td>
              <td>{{ entry.target || '—' }}</td>
              <td>{{ envName(entry.environment_id) }}</td>
              <td><Badge :tone="resultBadge(entry.result)">{{ entry.result }}</Badge></td>
            </tr>
            <tr v-if="expandedId === entry.id" class="detail-row">
              <td colspan="6">
                <div class="detail-content">
                  <div v-if="entry.agent_id" class="detail-field">
                    <span class="detail-label muted">Agent ID:</span>
                    <span class="mono">{{ entry.agent_id }}</span>
                  </div>
                  <div v-if="entry.resource_id" class="detail-field">
                    <span class="detail-label muted">{{ t('auditLog.resource') }}:</span>
                    <span>{{ resourceName(entry.resource_id) }}</span>
                  </div>
                  <div v-if="entry.detail" class="detail-field">
                    <span class="detail-label muted">Detail:</span>
                    <pre v-if="isJsonDetail(entry.detail)" class="detail-code mono">{{ formatDetail(entry.detail) }}</pre>
                    <span v-else>{{ entry.detail }}</span>
                  </div>
                </div>
              </td>
            </tr>
          </template>
        </tbody>
      </table>
    </Card>

    <!-- Context menu -->
    <Teleport to="body">
      <div v-if="ctxMenu.show" class="audit-ctx-overlay" @click="closeCtxMenu" @contextmenu.prevent="closeCtxMenu">
        <div class="audit-ctx-menu" :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }" @click.stop>
          <button class="ctx-item" @click="ctxViewDetail">📋 {{ t('auditLog.viewDetail') }}</button>
          <button class="ctx-item" @click="ctxCopyRecord">📋 {{ t('auditLog.copy') }}</button>
          <div class="ctx-divider"></div>
          <button class="ctx-item" @click="ctxFilterByType">🏷 {{ t('auditLog.filterByType') }}</button>
          <button class="ctx-item" @click="ctxFilterByEnv">🌍 {{ t('auditLog.filterByEnv') }}</button>
          <div class="ctx-divider"></div>
          <button class="ctx-item" @click="ctxRefresh">🔄 {{ t('auditLog.refresh') }}</button>
          <button class="ctx-item" @click="ctxExportCsv">📥 {{ t('auditLog.export') }}</button>
          <button class="ctx-item" @click="ctxClearFilters">🧹 {{ t('auditLog.clearFilters') }}</button>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.audit-page { height: 100%; overflow-y: auto; }
.page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--space-4); flex-wrap: wrap; gap: var(--space-3); }
.page-title { font-size: var(--text-xl); font-weight: 600; color: var(--text-primary); }
.header-actions { display: flex; gap: var(--space-2); }
.stats-row { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--space-3); margin-bottom: var(--space-4); }
.stat-card { text-align: center; padding: var(--space-3); }
.stat-value { font-size: var(--text-xl); font-weight: 700; color: var(--text-primary); }
.stat-value--success { color: var(--success); }
.stat-value--failure { color: var(--danger); }
.stat-label { font-size: var(--text-xs); margin-top: var(--space-1); }
.filters { display: flex; gap: var(--space-2); align-items: center; margin-bottom: var(--space-4); flex-wrap: wrap; }
.filter-select {
  background: var(--bg-deep); border: 1px solid var(--border); border-radius: 6px;
  padding: 6px 10px; color: var(--text-primary); font-size: var(--text-sm); outline: none;
}
.filter-select:focus { border-color: var(--accent); }
.log-card { overflow-x: auto; }
.loading { padding: var(--space-6); text-align: center; }
.log-table { width: 100%; border-collapse: collapse; font-size: var(--text-sm); }
.log-table th { text-align: left; padding: var(--space-2) var(--space-3); color: var(--text-muted); font-weight: 500; border-bottom: 1px solid var(--border); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.5px; }
.log-table td { padding: var(--space-2) var(--space-3); border-bottom: 1px solid var(--border); color: var(--text-secondary); }
.log-row { cursor: pointer; }
.log-row:hover td { background: var(--bg-hover); }
.expand-icon { width: 24px; text-align: center; color: var(--text-muted); font-size: var(--text-xs); }
.detail-row td { background: var(--bg-deep); }
.detail-content { padding: var(--space-3) var(--space-4); display: flex; flex-direction: column; gap: var(--space-2); }
.detail-field { display: flex; gap: var(--space-2); font-size: var(--text-sm); align-items: baseline; }
.detail-label { font-size: var(--text-xs); min-width: 100px; flex-shrink: 0; }
.detail-code { background: var(--bg-surface); border: 1px solid var(--border); border-radius: 4px; padding: var(--space-2); font-size: var(--text-xs); margin: 0; overflow-x: auto; white-space: pre-wrap; }
.muted { color: var(--text-muted); }
.mono { font-family: var(--font-mono); }

/* Context menu */
.audit-ctx-overlay {
  position: fixed; inset: 0; z-index: 200;
}
.audit-ctx-menu {
  position: fixed; z-index: 201;
  background: var(--bg-elevated, var(--bg-surface));
  border: 1px solid var(--border); border-radius: var(--radius, 8px);
  box-shadow: 0 8px 24px rgba(0,0,0,0.35);
  min-width: 180px; padding: var(--space-1, 4px) 0;
}
.ctx-item {
  display: flex; align-items: center; gap: 8px;
  width: 100%; padding: var(--space-2, 8px) var(--space-3, 12px);
  background: none; border: none; color: var(--text-primary); font-size: var(--text-sm, 13px);
  cursor: pointer; text-align: left;
}
.ctx-item:hover { background: var(--bg-hover); }
.ctx-divider {
  height: 1px; background: var(--border);
  margin: var(--space-1, 4px) 0;
}
</style>
