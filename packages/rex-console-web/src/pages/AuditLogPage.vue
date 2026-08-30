<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { auditApi, type AuditEntry, type AuditStats } from '@/api/audit'
import { useEnvironmentsStore } from '@/stores/environments'
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

function formatTime(time: string): string {
  const d = new Date(time)
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  const h = String(d.getHours()).padStart(2, '0')
  const min = String(d.getMinutes()).padStart(2, '0')
  const s = String(d.getSeconds()).padStart(2, '0')
  return `${m}-${day} ${h}:${min}:${s}`
}

function opTagClass(action: string): string {
  if (action.includes('SSH')) return 'ssh'
  if (action.includes('SQL')) return 'sql'
  if (action.includes('REDIS')) return 'redis'
  if (action.includes('FILE')) return 'file'
  if (action.includes('ENV')) return 'env'
  if (action.includes('AGENT')) return 'agent'
  return 'env'
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
      </div>
    </header>

    <p class="page-desc">{{ t('auditLog.subtitle', 'System activity log') }}</p>

    <!-- Toolbar: filters + actions -->
    <div class="toolbar">
      <div class="field">
        <span class="field-label">{{ t('auditLog.allEnvironments') }}</span>
        <Select
          v-model="environmentFilter"
          :options="[
            { label: t('auditLog.allEnvironments'), value: '' },
            ...store.environments.map(e => ({ label: e.name, value: e.id })),
          ]"
          size="sm"
        />
      </div>
      <div class="field">
        <span class="field-label">{{ t('auditLog.allResults') }}</span>
        <Select
          v-model="actionFilter"
          :options="actionOptions.map(o => ({ label: o.value ? o.value : t(o.label), value: o.value }))"
          size="sm"
        />
      </div>
      <div class="field">
        <span class="field-label">{{ t('auditLog.time', 'Time') }}</span>
        <Select
          v-model="timeRange"
          :options="timeRangeOptions.map(o => ({ label: t(o.label), value: o.value }))"
          size="sm"
        />
      </div>
      <div class="field">
        <span class="field-label">{{ t('auditLog.result') }}</span>
        <Select
          v-model="resultFilter"
          :options="[
            { label: t('auditLog.allResults'), value: '' },
            { label: t('auditLog.success'), value: 'success' },
            { label: t('auditLog.failure'), value: 'failure' },
          ]"
          size="sm"
        />
      </div>
      <span class="spacer"></span>
      <Button variant="ghost" size="sm" @click="actionFilter = ''; resultFilter = ''; environmentFilter = ''; timeRange = 'all'">
        {{ t('auditLog.clearFilters') }}
      </Button>
      <Button variant="primary" size="sm" @click="exportCsv">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
        {{ t('auditLog.exportCsv') }}
      </Button>
    </div>

    <!-- Stats cards -->
    <div class="stats">
      <div class="stat">
        <div class="stat-key">{{ t('auditLog.statTotal') }}</div>
        <div class="stat-value" :class="{ loading }">{{ loading ? '—' : stats.total.toLocaleString() }}</div>
      </div>
      <div class="stat green">
        <div class="stat-key">{{ t('auditLog.statSuccess') }}</div>
        <div class="stat-value" :class="{ loading }">{{ loading ? '—' : stats.success_count.toLocaleString() }}</div>
      </div>
      <div class="stat red">
        <div class="stat-key">{{ t('auditLog.statFailure') }}</div>
        <div class="stat-value" :class="{ loading }">{{ loading ? '—' : stats.failure_count.toLocaleString() }}</div>
      </div>
      <div class="stat brand">
        <div class="stat-key">{{ t('auditLog.activeUsers', 'Active users') }}</div>
        <div class="stat-value" :class="{ loading }">{{ loading ? '—' : 1 }}</div>
      </div>
    </div>

    <!-- Empty state -->
    <EmptyState
      v-if="!loading && entries.length === 0"
      icon="📋"
      :title="t('auditLog.noEntries')"
      :description="t('auditLog.emptyDesc')"
    />

    <!-- Data table -->
    <div v-else class="table-wrap">
      <div v-if="loading" class="loading muted">{{ t('common.loadingEllipsis') }}</div>
      <ResponsiveTable v-else>
        <table class="tbl">
          <thead>
            <tr>
              <th>{{ t('auditLog.time') }}</th>
              <th>{{ t('auditLog.user', 'User') }}</th>
              <th>{{ t('auditLog.environment') }}</th>
              <th>{{ t('auditLog.action') }}</th>
              <th>{{ t('auditLog.target') }}</th>
              <th>{{ t('auditLog.result') }}</th>
            </tr>
          </thead>
          <tbody>
            <template v-for="entry in entries" :key="entry.id">
              <tr
                class="tbl-row"
                :class="{ open: expandedId === entry.id }"
                @click="toggleExpand(entry.id)"
                @contextmenu.prevent="onContextMenu($event, entry)"
              >
                <td class="time">{{ formatTime(entry.time) }}</td>
                <td class="user">admin</td>
                <td>{{ envName(entry.environment_id) }}</td>
                <td>
                  <span class="otag" :class="opTagClass(entry.action)">
                    {{ entry.action.replace(/_/g, ' ') }}
                  </span>
                </td>
                <td>{{ entry.target || '—' }}</td>
                <td>
                  <span class="rc" :class="entry.result === 'success' ? 'ok' : 'fail'">
                    {{ entry.result }}
                  </span>
                </td>
              </tr>
              <tr v-if="expandedId === entry.id" class="detail-row">
                <td colspan="6">
                  <div class="detail-content">
                    <dl class="kv">
                      <dt>ID</dt>
                      <dd class="mono">{{ entry.id }}</dd>
                      <dt>{{ t('auditLog.time', 'Time') }}</dt>
                      <dd class="mono">{{ entry.time }}</dd>
                      <dt>{{ t('auditLog.action', 'Action') }}</dt>
                      <dd class="mono">{{ entry.action }}</dd>
                      <dt>{{ t('auditLog.result', 'Result') }}</dt>
                      <dd>
                        <span class="rc" :class="entry.result === 'success' ? 'ok' : 'fail'">{{ entry.result }}</span>
                      </dd>
                      <template v-if="entry.target">
                        <dt>{{ t('auditLog.target', 'Target') }}</dt>
                        <dd>{{ entry.target }}</dd>
                      </template>
                      <template v-if="entry.environment_id">
                        <dt>{{ t('auditLog.environment', 'Environment') }}</dt>
                        <dd>{{ envName(entry.environment_id) }}</dd>
                      </template>
                      <template v-if="entry.agent_id">
                        <dt>Agent ID</dt>
                        <dd class="mono">{{ entry.agent_id }}</dd>
                      </template>
                      <template v-if="entry.resource_id">
                        <dt>{{ t('auditLog.resource', 'Resource') }}</dt>
                        <dd>{{ resourceName(entry.resource_id) }}</dd>
                      </template>
                    </dl>
                    <pre v-if="entry.detail" class="detail-code mono"><span class="cm">{{ t('auditLog.detail', 'Detail') }}</span>
{{ formatDetail(entry.detail) }}</pre>
                  </div>
                </td>
              </tr>
            </template>
          </tbody>
        </table>
      </ResponsiveTable>
    </div>

    <!-- Pagination -->
    <div class="pagination">
      <span class="page-total muted">{{ totalCount.toLocaleString() }} {{ t('auditLog.totalCount', 'total') }}</span>
      <Select
        v-model="pageSize"
        :options="pageSizeOptions"
        size="sm"
      />
      <button class="page-btn" :disabled="currentPage <= 1" @click="currentPage--">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="15 18 9 12 15 6"/></svg>
      </button>
      <span class="page-info mono">{{ currentPage }} / {{ totalPages }}</span>
      <button class="page-btn" :disabled="currentPage >= totalPages" @click="currentPage++">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>
      </button>
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
.audit-page {
  height: 100%;
  overflow-y: auto;
  padding: var(--space-6);
}

.page-header {
  margin-bottom: var(--space-1);
}

.page-desc {
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin-bottom: var(--space-4);
  line-height: 1.5;
}

/* Toolbar */
.toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-4);
  flex-wrap: wrap;
}

.field {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  height: 34px;
  padding: 0 var(--space-3);
  border-radius: 7px;
  border: 1px solid var(--border-strong);
  background: var(--bg-surface);
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.field-label {
  color: var(--text-muted);
  font-size: var(--text-xs);
  white-space: nowrap;
}

.spacer {
  flex: 1;
}

/* Stats */
.stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 14px;
  margin-bottom: 18px;
}

.stat {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--space-3) var(--space-4);
}

.stat-key {
  font-size: 10.5px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.stat-value {
  font-family: var(--font-mono);
  font-size: 26px;
  font-weight: 700;
  margin-top: 6px;
  color: var(--text-primary);
}

.stat-value.loading {
  opacity: 0.4;
}

.stat.green .stat-value {
  color: var(--success);
}

.stat.red .stat-value {
  color: var(--danger);
}

.stat.brand .stat-value {
  color: var(--accent);
}

/* Table */
.table-wrap {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
}

.loading {
  padding: var(--space-6);
  text-align: center;
}

.tbl {
  width: 100%;
  border-collapse: collapse;
}

.tbl thead th {
  text-align: left;
  font-size: 10.5px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted);
  font-family: var(--font-mono);
  padding: 10px 14px;
  border-bottom: 1px solid var(--border-strong);
  background: var(--bg-elevated);
}

.tbl tbody td {
  padding: 11px 14px;
  border-bottom: 1px solid var(--border);
  font-size: var(--text-base);
  vertical-align: top;
}

.tbl tbody tr:last-child td {
  border-bottom: 0;
}

.tbl-row {
  cursor: pointer;
}

.tbl-row:hover td {
  background: var(--bg-hover);
}

.tbl-row.open td {
  background: var(--accent-soft);
}

.tbl .time {
  font-family: var(--font-mono);
  color: var(--text-muted);
  white-space: nowrap;
}

.tbl .user {
  font-family: var(--font-mono);
}

/* Operation tags */
.otag {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 22px;
  padding: 0 9px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 600;
  font-family: var(--font-mono);
}

.otag.ssh {
  background: var(--accent-soft);
  color: var(--accent);
}

.otag.sql {
  background: var(--info-soft);
  color: var(--info);
}

.otag.redis {
  background: var(--purple-soft);
  color: var(--purple);
}

.otag.file {
  background: var(--purple-soft);
  color: var(--purple);
}

.otag.env {
  background: var(--bg-elevated);
  color: var(--text-muted);
}

.otag.agent {
  background: var(--success-soft);
  color: var(--success);
}

/* Result codes */
.rc {
  font-family: var(--font-mono);
  font-weight: 700;
}

.rc.ok {
  color: var(--success);
}

.rc.fail {
  color: var(--danger);
}

/* Detail row */
.detail-row td {
  background: var(--bg-deep);
  padding: 0;
}

.detail-content {
  padding: var(--space-4);
}

.kv {
  display: grid;
  grid-template-columns: 120px 1fr;
  gap: 4px 14px;
  font-size: var(--text-sm);
  margin: 0;
}

.kv dt {
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.kv dd {
  margin: 0;
  font-family: var(--font-mono);
  color: var(--text-primary);
}

.detail-code {
  margin: var(--space-3) 0 0 0;
  padding: var(--space-4);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  line-height: 1.6;
  color: var(--text-primary);
  white-space: pre-wrap;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow-x: auto;
}

.cm {
  color: var(--text-muted);
}

.mono {
  font-family: var(--font-mono);
}

.muted {
  color: var(--text-muted);
}

/* Context menu */
.ctx-item {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  cursor: pointer;
  color: var(--text-primary);
}

.ctx-item:hover {
  background: var(--bg-hover);
  color: var(--accent);
}

.ctx-divider {
  height: 1px;
  background: var(--border);
  margin: var(--space-1) 0;
}

/* Pagination */
.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: var(--space-3);
  padding: var(--space-4) 0;
}

.page-total {
  font-size: var(--text-xs);
}

.page-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-1) var(--space-2);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-secondary);
  cursor: pointer;
  transition: border-color var(--transition), color var(--transition);
}

.page-btn:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--text-primary);
}

.page-btn:disabled {
  opacity: var(--disabled-opacity);
  cursor: not-allowed;
}

.page-info {
  font-size: var(--text-xs);
  color: var(--text-muted);
}

.page-goto {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  font-size: var(--text-xs);
}

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

.page-goto-input:focus {
  border-color: var(--accent);
}
</style>
