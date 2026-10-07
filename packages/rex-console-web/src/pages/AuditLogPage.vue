<script setup lang="ts">
import { formatDateTime } from '@/utils/datetime'
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { auditApi, type AuditEntry, type AuditStats } from '@/api/audit'
import { clipboard } from '@/utils/clipboard'
import { agentsApi, type Agent } from '@/api/agents'
import { useEnvironmentsStore } from '@/stores/environments'
import { useNotificationStore } from '@/stores/notification'
import Button from '@/components/ui/Button.vue'
import ContextMenu from '@/components/ui/ContextMenu.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Select from '@/components/ui/Select.vue'
import ResponsiveTable from '@/components/ResponsiveTable.vue'

const { t } = useI18n()
const store = useEnvironmentsStore()
const notify = useNotificationStore()

const entries = ref<AuditEntry[]>([])
const loading = ref(true)
const stats = ref<AuditStats>({ total: 0, success_count: 0, failure_count: 0 })
const listError = ref('')
const statsError = ref('')
const expandedId = ref<string | null>(null)
const currentPage = ref(1)
const pageSize = ref(50)
const pageSizeOptions = [
  { label: '50', value: 50 },
  { label: '100', value: 100 },
  { label: '200', value: 200 },
]
const totalCount = ref(0)
// The exact total only comes from the statistics endpoint. Without it the row
// count of the current page cannot be turned into a total, so the total stays
// explicitly unknown instead of being guessed from the rows that did arrive.
const totalKnown = ref(false)
const agentsMap = ref<Map<string, Agent>>(new Map())

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
    await clipboard.writeText(JSON.stringify(ctxMenu.value.entry, null, 2))
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

function ctxFilterByResource() {
  if (ctxMenu.value.entry) resourceFilter.value = ctxMenu.value.entry.resource_id || ''
  closeCtxMenu()
}

function ctxFilterByAgent() {
  if (ctxMenu.value.entry) agentFilter.value = ctxMenu.value.entry.agent_id || ''
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

function clearFilters() {
  actionFilter.value = ''
  resultFilter.value = ''
  environmentFilter.value = ''
  resourceFilter.value = ''
  agentFilter.value = ''
  timeRange.value = 'all'
}

function ctxClearFilters() {
  clearFilters()
  closeCtxMenu()
}

function handleCtxAction(action: string) {
  switch (action) {
    case 'detail': ctxViewDetail(); break
    case 'copy': ctxCopyRecord(); break
    case 'filterType': ctxFilterByType(); break
    case 'filterEnv': ctxFilterByEnv(); break
    case 'filterResource': ctxFilterByResource(); break
    case 'filterAgent': ctxFilterByAgent(); break
    case 'refresh': ctxRefresh(); break
    case 'export': ctxExportCsv(); break
    case 'clearFilters': ctxClearFilters(); break
  }
}

// Filters
const actionFilter = ref('')
const resultFilter = ref('')
const environmentFilter = ref('')
const resourceFilter = ref('')
const agentFilter = ref('')
const timeRange = ref('all')

// Resource / Agent chips reuse the data already loaded for name resolution (no extra request)
const resourceOptions = computed(() => {
  const envId = environmentFilter.value
  const groups = envId ? [store.envResources.get(envId)] : [...store.envResources.values()]
  const seen = new Set<string>()
  const options: { id: string; name: string }[] = []
  for (const list of groups) {
    for (const r of list ?? []) {
      if (seen.has(r.id)) continue
      seen.add(r.id)
      options.push({ id: r.id, name: r.name })
    }
  }
  return options.sort((a, b) => a.name.localeCompare(b.name))
})

const agentOptions = computed(() => {
  const envId = environmentFilter.value
  return [...agentsMap.value.values()]
    .filter(a => !envId || a.environment_id === envId)
    .sort((a, b) => a.name.localeCompare(b.name))
})

const statsUnavailable = computed(() => loading.value || !!statsError.value)
const totalUnavailable = computed(() => !!listError.value || !!statsError.value)

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

function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

async function fetchEntries() {
  loading.value = true
  const range = getTimeRange()
  const offset = (currentPage.value - 1) * pageSize.value
  const filters = {
    action: actionFilter.value || undefined,
    result: resultFilter.value || undefined,
    environment_id: environmentFilter.value || undefined,
    resource_id: resourceFilter.value || undefined,
    agent_id: agentFilter.value || undefined,
    ...range,
  }
  try {
    // List and stats degrade independently: a failing stats call must not blank the table
    const [listResult, statsResult] = await Promise.allSettled([
      auditApi.query({ ...filters, limit: pageSize.value, offset }),
      auditApi.stats({ ...filters }),
    ])
    const hadListError = !!listError.value
    const hadStatsError = !!statsError.value

    if (listResult.status === 'fulfilled') {
      entries.value = listResult.value
      listError.value = ''
    } else {
      console.error('Audit log query failed:', listResult.reason)
      entries.value = []
      listError.value = errorMessage(listResult.reason)
      if (!hadListError) notify.error(t('auditLog.loadFailed', 'Failed to load audit entries'))
    }

    if (statsResult.status === 'fulfilled') {
      stats.value = statsResult.value
      totalCount.value = statsResult.value.total
      totalKnown.value = true
      statsError.value = ''
    } else {
      console.error('Audit log stats query failed:', statsResult.reason)
      // The rows of the current page say nothing about the total: mark it unknown
      totalKnown.value = false
      statsError.value = errorMessage(statsResult.reason)
      if (!hadStatsError && listResult.status === 'fulfilled') {
        notify.warning(t('auditLog.statsFailed', 'Statistics unavailable, showing the list only'))
      }
    }
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
  try {
    const allEntries = await auditApi.query({
      action: actionFilter.value || undefined,
      result: resultFilter.value || undefined,
      environment_id: environmentFilter.value || undefined,
      resource_id: resourceFilter.value || undefined,
      agent_id: agentFilter.value || undefined,
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
  } catch (e) {
    console.error('Audit log CSV export failed:', e)
    notify.error(t('auditLog.exportFailed', 'Failed to export CSV'))
  }
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
function agentName(agentId: string | null): string {
  if (!agentId) return '—'
  const agent = agentsMap.value.get(agentId)
  return agent?.name || agentId.slice(0, 8) + '…'
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

const formatTime = formatDateTime

function opTagClass(action: string): string {
  if (action.includes('SSH')) return 'ssh'
  if (action.includes('SQL')) return 'sql'
  if (action.includes('REDIS')) return 'redis'
  if (action.includes('FILE')) return 'file'
  if (action.includes('ENV')) return 'env'
  if (action.includes('AGENT')) return 'agent'
  return 'env'
}

// Total pages only exist when the total is known; the received rows say nothing
// about how many entries follow them.
const totalPages = computed(() =>
  totalKnown.value ? Math.max(1, Math.ceil(totalCount.value / pageSize.value)) : null,
)
const pageInfo = computed(() =>
  totalPages.value === null ? `${currentPage.value} / —` : `${currentPage.value} / ${totalPages.value}`,
)
// Jump to page needs a last page to clamp against, so it stays disabled without a known total
const gotoDisabled = computed(() => !!listError.value || totalPages.value === null)
// An unknown total can only be probed: a partial page is the end of the list,
// a full page may still have entries after it
const pageHasMore = computed(() =>
  totalPages.value !== null
    ? currentPage.value < totalPages.value
    : entries.value.length >= pageSize.value,
)

const gotoPage = ref(1)

// Scope filters follow the environment: drop a resource / agent selection that left the scope
watch(environmentFilter, () => {
  if (resourceFilter.value && !resourceOptions.value.some(r => r.id === resourceFilter.value)) {
    resourceFilter.value = ''
  }
  if (agentFilter.value && !agentOptions.value.some(a => a.id === agentFilter.value)) {
    agentFilter.value = ''
  }
})

watch([actionFilter, resultFilter, environmentFilter, resourceFilter, agentFilter, timeRange], () => {
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
  if (totalPages.value === null) return
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
  let resourceFailures = 0
  let agentFailures = 0
  await Promise.all(store.environments.map(async (e) => {
    // Settled independently: one failing env must not abort the rest or the audit fetch below
    const [resourceResult, agentResult] = await Promise.allSettled([
      store.fetchResources(e.id),
      agentsApi.listByEnv(e.id),
    ])
    if (resourceResult.status === 'rejected') resourceFailures += 1
    if (agentResult.status === 'fulfilled') {
      for (const agent of agentResult.value) {
        agentsMap.value.set(agent.id, agent)
      }
    } else {
      agentFailures += 1
    }
  }))
  if (resourceFailures > 0) {
    console.error(`Failed to load resources for ${resourceFailures} environment(s)`)
    notify.warning(t('auditLog.resourcesResolveFailed', 'Resources failed to load for some environments, names cannot be resolved'))
  }
  if (agentFailures > 0) {
    console.error(`Failed to load agents for ${agentFailures} environment(s)`)
    notify.warning(t('auditLog.agentsResolveFailed', 'Agents failed to load for some environments, names cannot be resolved'))
  }
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

    <p class="page-desc">{{ t('auditLog.subtitle') }}</p>

    <!-- Toolbar: chip filters + actions -->
    <div class="toolbar">
      <div class="filter-chips">
        <span class="filter-chips-label">{{ t('auditLog.env', 'Env') }}</span>
        <button
          class="filter-chip"
          :class="{ 'filter-chip--on': !environmentFilter }"
          @click="environmentFilter = ''"
        >
          {{ t('auditLog.allEnvironments') }}
        </button>
        <button
          v-for="env in store.environments"
          :key="env.id"
          class="filter-chip"
          :class="{ 'filter-chip--on': environmentFilter === env.id }"
          @click="environmentFilter = env.id"
        >
          {{ env.name }}
        </button>
      </div>
      <div v-if="resourceOptions.length" class="filter-chips">
        <span class="filter-chips-label">{{ t('auditLog.resource', 'Resource') }}</span>
        <button
          class="filter-chip"
          :class="{ 'filter-chip--on': !resourceFilter }"
          @click="resourceFilter = ''"
        >
          {{ t('auditLog.allResources', 'All Resources') }}
        </button>
        <button
          v-for="opt in resourceOptions"
          :key="opt.id"
          class="filter-chip"
          :class="{ 'filter-chip--on': resourceFilter === opt.id }"
          @click="resourceFilter = opt.id"
        >
          {{ opt.name }}
        </button>
      </div>
      <div v-if="agentOptions.length" class="filter-chips">
        <span class="filter-chips-label">{{ t('auditLog.agent', 'Agent') }}</span>
        <button
          class="filter-chip"
          :class="{ 'filter-chip--on': !agentFilter }"
          @click="agentFilter = ''"
        >
          {{ t('auditLog.allAgents', 'All Agents') }}
        </button>
        <button
          v-for="opt in agentOptions"
          :key="opt.id"
          class="filter-chip"
          :class="{ 'filter-chip--on': agentFilter === opt.id }"
          @click="agentFilter = opt.id"
        >
          {{ opt.name }}
        </button>
      </div>
      <div class="filter-chips">
        <span class="filter-chips-label">{{ t('auditLog.type', 'Type') }}</span>
        <button
          class="filter-chip"
          :class="{ 'filter-chip--on': !actionFilter }"
          @click="actionFilter = ''"
        >
          {{ t('auditLog.allResults') }}
        </button>
        <button
          v-for="opt in actionOptions.filter(o => o.value)"
          :key="opt.value"
          class="filter-chip"
          :class="{ 'filter-chip--on': actionFilter === opt.value }"
          @click="actionFilter = opt.value"
        >
          {{ opt.value }}
        </button>
      </div>
      <span class="spacer"></span>
      <Button variant="ghost" size="sm" @click="clearFilters">
        {{ t('auditLog.clearFilters') }}
      </Button>
      <Button variant="primary" size="sm" @click="exportCsv">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" /></svg>
        {{ t('auditLog.exportCsv') }}
      </Button>
    </div>
    <!-- Partial failure notice: list and stats load independently -->
    <div
      v-if="listError || statsError"
      class="load-note"
      :class="listError ? 'load-note--error' : 'load-note--warn'"
    >
      <span class="load-note-icon">{{ listError ? '✕' : '⚠' }}</span>
      <span class="load-note-text">
        {{ listError
          ? t('auditLog.loadFailed', 'Failed to load audit entries')
          : t('auditLog.statsFailed', 'Statistics unavailable, showing the list only') }}
      </span>
      <span class="load-note-detail mono" :title="listError || statsError">{{ listError || statsError }}</span>
      <Button variant="ghost" size="sm" @click="refreshAll">{{ t('common.refresh') }}</Button>
    </div>

    <!-- Stats cards -->
    <div class="stats">
      <div class="stat">
        <div class="stat-key">{{ t('auditLog.statTotal') }}</div>
        <div class="stat-value" :class="{ loading: loading && !statsError, 'stat-value--error': statsError }">{{ statsUnavailable ? '—' : stats.total.toLocaleString() }}</div>
      </div>
      <div class="stat green">
        <div class="stat-key">{{ t('auditLog.statSuccess') }}</div>
        <div class="stat-value" :class="{ loading: loading && !statsError, 'stat-value--error': statsError }">{{ statsUnavailable ? '—' : stats.success_count.toLocaleString() }}</div>
      </div>
      <div class="stat red">
        <div class="stat-key">{{ t('auditLog.statFailure') }}</div>
        <div class="stat-value" :class="{ loading: loading && !statsError, 'stat-value--error': statsError }">{{ statsUnavailable ? '—' : stats.failure_count.toLocaleString() }}</div>
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
      :title="listError
        ? t('auditLog.loadFailed', 'Failed to load audit entries')
        : t('auditLog.noEntries')"
      :description="listError || t('auditLog.emptyDesc')"
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
                <td>{{ entry.action.startsWith('AGENT_') ? agentName(entry.target) : (entry.target || '—') }}</td>
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
                      <dt>{{ t('auditLog.time') }}</dt>
                      <dd class="mono">{{ entry.time }}</dd>
                      <dt>{{ t('auditLog.action') }}</dt>
                      <dd class="mono">{{ entry.action }}</dd>
                      <dt>{{ t('auditLog.result') }}</dt>
                      <dd>
                        <span class="rc" :class="entry.result === 'success' ? 'ok' : 'fail'">{{ entry.result }}</span>
                      </dd>
                      <template v-if="entry.target">
                        <dt>{{ t('auditLog.target') }}</dt>
                        <dd>{{ entry.target }}</dd>
                      </template>
                      <template v-if="entry.environment_id">
                        <dt>{{ t('auditLog.environment') }}</dt>
                        <dd>{{ envName(entry.environment_id) }}</dd>
                      </template>
                      <template v-if="entry.agent_id">
                        <dt>{{ t('auditLog.agent', 'Agent') }}</dt>
                        <dd><span>{{ agentName(entry.agent_id) }}</span> <span v-if="agentsMap.get(entry.agent_id!)" class="muted" style="font-size:0.85em">({{ entry.agent_id }})</span></dd>
                      </template>
                      <template v-if="entry.resource_id">
                        <dt>{{ t('auditLog.resource') }}</dt>
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
    <div class="audit-table-footer">
      <span class="page-total muted">{{ t('auditLog.totalCount', { n: totalUnavailable ? '—' : totalCount }) }}</span>
      <span class="field-label">{{ t('auditLog.pageSize') }}</span>
      <Select v-model="pageSize" :options="pageSizeOptions" size="sm" />
      <button class="page-btn" :disabled="currentPage <= 1 || !!listError" @click="currentPage--">← {{ t('common.prev', 'Prev') }}</button>
      <!-- Total unknown: step forward while the backend keeps returning full pages, never past its end -->
      <span class="page-info mono">{{ pageInfo }}</span>
      <button class="page-btn" :disabled="!pageHasMore || !!listError" @click="currentPage++">{{ t('common.next', 'Next') }} →</button>
      <span class="page-goto">
        <span class="muted">{{ t('auditLog.gotoPage') }}</span>
        <input
          v-model.number="gotoPage"
          class="page-goto-input mono"
          type="number"
          min="1"
          :max="totalPages ?? undefined"
          :disabled="gotoDisabled"
          @keyup.enter="applyGoto"
        />
        <span class="muted">{{ t('auditLog.pageUnit') }}</span>
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
        <div class="ctx-item" @click="choose('filterResource')">🗄 {{ t('auditLog.filterByResource', 'Filter by Resource') }}</div>
        <div class="ctx-item" @click="choose('filterAgent')">🖥️ {{ t('auditLog.filterByAgent', 'Filter by Agent') }}</div>
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
  /* Fixed-height layout: only the table body scrolls inside the viewport */
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: var(--space-6);
}

.page-header {
  flex-shrink: 0;
  margin-bottom: var(--space-1);
}

.page-desc {
  flex-shrink: 0;
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin-bottom: var(--space-4);
  line-height: 1.5;
}

/* Toolbar */
.toolbar {
  flex-shrink: 0;
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

/* Partial failure notice */
.load-note {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);
  margin-bottom: var(--space-3);
  border: 1px solid var(--border);
  border-left-width: 3px;
  border-radius: var(--radius);
  background: var(--bg-surface);
  font-size: var(--text-sm);
}
.load-note--warn {
  border-left-color: var(--warning);
  background: var(--warning-soft);
}
.load-note--error {
  border-left-color: var(--danger);
  background: var(--danger-soft);
}
.load-note-icon {
  flex-shrink: 0;
  font-size: var(--text-sm);
}
.load-note--warn .load-note-icon {
  color: var(--warning);
}
.load-note--error .load-note-icon {
  color: var(--danger);
}
.load-note-text {
  flex-shrink: 0;
  color: var(--text-primary);
}
.load-note-detail {
  flex: 1;
  min-width: 0;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Stats */
.stats {
  flex-shrink: 0;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 14px;
  margin-bottom: 18px;
}

.stat {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 14px var(--space-4);
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

/* Stats endpoint failed: show the failure, never a misleading 0 */
.stat .stat-value--error,
.stat.green .stat-value--error,
.stat.red .stat-value--error {
  color: var(--danger);
}

/* Table */
.table-wrap {
  flex: 1;
  min-height: 0;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow-x: hidden;
  overflow-y: auto;
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
  background: var(--teal-soft);
  color: var(--teal);
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

/* Filter Chips */
.filter-chips {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.filter-chips-label {
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-right: 2px;
}
.filter-chip {
  display: inline-flex;
  align-items: center;
  height: 24px;
  padding: 0 10px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-muted);
  font-size: 11.5px;
  font-family: var(--font-mono);
  cursor: pointer;
  transition: background var(--transition), color var(--transition), border-color var(--transition);
}
.filter-chip:hover {
  background: var(--bg-hover);
  color: var(--text);
}
.filter-chip--on {
  background: var(--accent-soft);
  color: var(--accent);
  border-color: var(--accent);
}

.audit-table-footer {
  flex-shrink: 0;
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
