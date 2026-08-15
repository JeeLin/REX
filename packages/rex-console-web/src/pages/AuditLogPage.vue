<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { auditApi, type AuditEntry, type AuditStats } from '@/api/audit'
import { useEnvironmentsStore } from '@/stores/environments'
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import ContextMenu from '@/components/ui/ContextMenu.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Select from '@/components/ui/Select.vue'
import ResponsiveTable from '@/components/ResponsiveTable.vue'

const { t } = useI18n()
const store = useEnvironmentsStore()

const entries = ref<AuditEntry[]>([])
const loading = ref(true)
const stats = ref<AuditStats>({ total: 0, success_count: 0, failure_count: 0 })
const expandedId = ref<string | null>(null)
const currentPage = ref(1)
const pageSize = ref(50)
const pageSizeOptions = [
  { label: '20', value: 20 },
  { label: '50', value: 50 },
  { label: '100', value: 100 },
]
const totalCount = ref(0)

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
    try {
      await navigator.clipboard.writeText(JSON.stringify(ctxMenu.value.entry, null, 2))
    } catch {
      // fallback for non-HTTPS
      const ta = document.createElement('textarea')
      ta.value = JSON.stringify(ctxMenu.value.entry, null, 2)
      ta.style.position = 'fixed'
      ta.style.left = '-9999px'
      document.body.appendChild(ta)
      ta.select()
      document.execCommand('copy')
      document.body.removeChild(ta)
    }
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

function handleCtxAction(action: string) {
  switch (action) {
    case 'detail': ctxViewDetail(); break
    case 'copy': ctxCopyRecord(); break
    case 'filterType': ctxFilterByType(); break
    case 'filterEnv': ctxFilterByEnv(); break
    case 'refresh': ctxRefresh(); break
    case 'export': ctxExportCsv(); break
    case 'clearFilters': ctxClearFilters(); break
  }
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
    const offset = (currentPage.value - 1) * pageSize.value
    const [data, statsData] = await Promise.all([
      auditApi.query({
        action: actionFilter.value || undefined,
        result: resultFilter.value || undefined,
        environment_id: environmentFilter.value || undefined,
        ...range,
        limit: pageSize.value,
        offset,
      }),
      auditApi.stats({
        action: actionFilter.value || undefined,
        result: resultFilter.value || undefined,
        environment_id: environmentFilter.value || undefined,
        ...range,
      }),
    ])
    entries.value = data
    totalCount.value = statsData.total
    stats.value = statsData
  } catch {
    entries.value = []
    totalCount.value = 0
  } finally {
    loading.value = false
  }
}

function refreshAll() {
  fetchEntries()
}

function toggleExpand(id: string) {
  expandedId.value = expandedId.value === id ? null : id
}

async function exportCsv() {
  const range = getTimeRange()
  const allEntries = await auditApi.query({
    action: actionFilter.value || undefined,
    result: resultFilter.value || undefined,
    environment_id: environmentFilter.value || undefined,
    ...range,
    limit: 10000,
  })
  const headers = ['time', 'action', 'target', 'environment_id', 'resource_id', 'agent_id', 'result', 'detail']
  const rows = allEntries.map(e => headers.map(h => {
    const val = (e as unknown as Record<string, unknown>)[h]
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

function actionBadge(action: string) {
  if (action.includes('DELETE')) return 'danger'
  if (action.includes('CREATE')) return 'success'
  if (action.includes('ONLINE')) return 'success'
  if (action.includes('OFFLINE')) return 'danger'
  return 'info'
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

const totalPages = computed(() => Math.max(1, Math.ceil(totalCount.value / pageSize.value)))

const gotoPage = ref(1)

watch([actionFilter, resultFilter, environmentFilter, timeRange], () => {
  currentPage.value = 1
  gotoPage.value = 1
  refreshAll()
})

watch(pageSize, () => {
  currentPage.value = 1
  gotoPage.value = 1
  refreshAll()
})

// 跳页：输入页码后回车跳转到目标页
function applyGoto() {
  const target = Math.min(Math.max(1, Math.floor(gotoPage.value || 1)), totalPages.value)
  currentPage.value = target
}

watch(currentPage, () => {
  gotoPage.value = currentPage.value
  fetchEntries()
})

onMounted(async () => {
  await store.fetchEnvironments()
  // Load resources for all environments to enable name resolution
  await Promise.all(store.environments.map(e => store.fetchResources(e.id)))
  refreshAll()
})
</script>

<template>
  <div class="page-container audit-page">
    <header class="page-header">
      <div class="page-header-left">
        <h1 class="page-title mono">{{ t('auditLog.title') }}</h1>
        <span class="page-subtitle">{{ t('auditLog.subtitle', 'System activity log') }}</span>
      </div>
      <div class="page-header-actions">
        <Button variant="ghost" size="sm" @click="refreshAll">↻ {{ t('common.refresh') }}</Button>
        <Button variant="ghost" size="sm" @click="exportCsv">↓ {{ t('auditLog.exportCsv') }}</Button>
      </div>
    </header>

    <!-- Stats cards -->
    <div class="stats-row">
      <Card class="stat-card">
        <div class="stat-value" :class="{ loading }">{{ loading ? '—' : stats.total }}</div>
        <div class="stat-label muted">{{ t('auditLog.statTotal') }}</div>
      </Card>
      <Card class="stat-card stat-card--success">
        <div class="stat-value stat-value--success" :class="{ loading }">{{ loading ? '—' : stats.success_count }}</div>
        <div class="stat-label muted">{{ t('auditLog.statSuccess') }}</div>
      </Card>
      <Card class="stat-card stat-card--failure">
        <div class="stat-value stat-value--failure" :class="{ loading }">{{ loading ? '—' : stats.failure_count }}</div>
        <div class="stat-label muted">{{ t('auditLog.statFailure') }}</div>
      </Card>
    </div>

    <!-- Filters -->
    <div class="filters">
      <Select
        v-model="actionFilter"
        :options="actionOptions.map(o => ({ label: o.value ? o.value : t(o.label), value: o.value }))"
        size="sm"
      />
      <Select
        v-model="resultFilter"
        :options="[
          { label: t('auditLog.allResults'), value: '' },
          { label: t('auditLog.success'), value: 'success' },
          { label: t('auditLog.failure'), value: 'failure' },
        ]"
        size="sm"
      />
      <Select
        v-model="environmentFilter"
        :options="[
          { label: t('auditLog.allEnvironments'), value: '' },
          ...store.environments.map(e => ({ label: e.name, value: e.id })),
        ]"
        size="sm"
      />
      <Select
        v-model="timeRange"
        :options="timeRangeOptions.map(o => ({ label: t(o.label), value: o.value }))"
        size="sm"
      />
    </div>

    <EmptyState
      v-if="!loading && entries.length === 0"
      icon="📋"
      :title="t('auditLog.noEntries')"
      :description="t('auditLog.emptyDesc')"
    />

    <Card v-else class="log-card">
      <div v-if="loading" class="loading muted">{{ t('common.loadingEllipsis') }}</div>
      <ResponsiveTable v-else>
        <table class="log-table">
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
                <td colspan="7">
                  <div class="detail-content">
                    <div class="detail-field">
                      <span class="detail-label muted">ID:</span>
                      <span class="mono">{{ entry.id }}</span>
                    </div>
                    <div class="detail-field">
                      <span class="detail-label muted">{{ t('auditLog.time', 'Time') }}:</span>
                      <span>{{ entry.time }}</span>
                    </div>
                    <div class="detail-field">
                      <span class="detail-label muted">{{ t('auditLog.action', 'Action') }}:</span>
                      <Badge :tone="actionBadge(entry.action)">{{ entry.action }}</Badge>
                    </div>
                    <div v-if="entry.target" class="detail-field">
                      <span class="detail-label muted">{{ t('auditLog.target', 'Target') }}:</span>
                      <span>{{ entry.target }}</span>
                    </div>
                    <div v-if="entry.environment_id" class="detail-field">
                      <span class="detail-label muted">{{ t('auditLog.environment', 'Environment') }}:</span>
                      <span>{{ envName(entry.environment_id) }}</span>
                    </div>
                    <div v-if="entry.agent_id" class="detail-field">
                      <span class="detail-label muted">Agent ID:</span>
                      <span class="mono">{{ entry.agent_id }}</span>
                    </div>
                    <div v-if="entry.resource_id" class="detail-field">
                      <span class="detail-label muted">{{ t('auditLog.resource') }}:</span>
                      <span>{{ resourceName(entry.resource_id) }}</span>
                    </div>
                    <div class="detail-field">
                      <span class="detail-label muted">{{ t('auditLog.result', 'Result') }}:</span>
                      <Badge :tone="resultBadge(entry.result)">{{ entry.result }}</Badge>
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
      </ResponsiveTable>
    </Card>

    <!-- Pagination -->
    <div class="pagination">
      <span class="page-total muted">{{ t('auditLog.totalCount', { n: totalCount }) }}</span>
      <Select
        v-model="pageSize"
        :options="pageSizeOptions"
        size="sm"
      />
      <button class="page-btn" :disabled="currentPage <= 1" @click="currentPage--">← {{ t('common.prev', 'Prev') }}</button>
      <span class="page-info mono">{{ currentPage }} / {{ totalPages }}</span>
      <button class="page-btn" :disabled="currentPage >= totalPages" @click="currentPage++">{{ t('common.next', 'Next') }} →</button>
      <span class="page-goto">
        <span class="muted">{{ t('auditLog.gotoPage') }}</span>
        <input
          v-model.number="gotoPage"
          class="page-goto-input mono"
          type="number"
          min="1"
          :max="totalPages"
          @keyup.enter="applyGoto"
        />
      </span>
    </div>

    <!-- Context menu -->
    <ContextMenu
      v-model="ctxMenu.show"
      :x="ctxMenu.x"
      :y="ctxMenu.y"
      @select="(action: string) => handleCtxAction(action)"
    >
      <template #default="{ choose }">
        <div class="ctx-item" @click="choose('detail')">📋 {{ t('auditLog.viewDetail') }}</div>
        <div class="ctx-item" @click="choose('copy')">📋 {{ t('auditLog.copy') }}</div>
        <div class="ctx-divider"></div>
        <div class="ctx-item" @click="choose('filterType')">🏷 {{ t('auditLog.filterByType') }}</div>
        <div class="ctx-item" @click="choose('filterEnv')">🌍 {{ t('auditLog.filterByEnv') }}</div>
        <div class="ctx-divider"></div>
        <div class="ctx-item" @click="choose('refresh')">🔄 {{ t('auditLog.refresh') }}</div>
        <div class="ctx-item" @click="choose('export')">📥 {{ t('auditLog.export') }}</div>
        <div class="ctx-item" @click="choose('clearFilters')">🧹 {{ t('auditLog.clearFilters') }}</div>
      </template>
    </ContextMenu>
  </div>
</template>

<style scoped>
.audit-page { height: 100%; overflow-y: auto; }
.page-header-actions { display: flex; gap: var(--space-2); }
.stats-row { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--space-3); margin-bottom: var(--space-4); }
.stat-card { text-align: center; padding: var(--space-3); }
.stat-value { font-size: var(--text-xl); font-weight: 700; color: var(--text-primary); }
.stat-value--success { color: var(--success); }
.stat-value--failure { color: var(--danger); }
.stat-label { font-size: var(--text-xs); margin-top: var(--space-1); }
.filters { display: flex; gap: var(--space-2); align-items: center; margin-bottom: var(--space-4); flex-wrap: wrap; }
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
.ctx-item {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  cursor: pointer;
  color: var(--text-primary);
}
.ctx-item:hover { background: var(--bg-hover); }
.ctx-divider {
  height: 1px; background: var(--border);
  margin: var(--space-1) 0;
}

/* Pagination */
.pagination {
  display: flex; align-items: center; justify-content: center;
  flex-wrap: wrap;
  gap: var(--space-3); padding: var(--space-4) 0;
}
.page-total { font-size: var(--text-xs); }
.page-btn {
  padding: var(--space-1) var(--space-3);
  background: var(--bg-surface); border: 1px solid var(--border);
  border-radius: var(--radius); color: var(--text-secondary);
  font-size: var(--text-sm); cursor: pointer;
  transition: border-color var(--transition), color var(--transition);
}
.page-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--text-primary); }
.page-btn:disabled { opacity: var(--disabled-opacity); cursor: not-allowed; }
.page-info { font-size: var(--text-xs); color: var(--text-muted); }
.page-goto { display: flex; align-items: center; gap: var(--space-1); font-size: var(--text-xs); }
.page-goto-input {
  width: 56px;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 4px 8px;
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}
.page-goto-input:focus { border-color: var(--accent); }
</style>
