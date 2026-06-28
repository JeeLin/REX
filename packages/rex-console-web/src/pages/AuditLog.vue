<template>
  <div class="audit-page">
    <!-- Header -->
    <div class="audit-header">
      <router-link to="/settings" class="btn btn-ghost btn-sm">← {{ t('audit.back') }}</router-link>
      <div class="header-spacer"></div>
      <button class="btn btn-ghost btn-sm" @click="exportCsv">{{ t('audit.exportCsv') }}</button>
    </div>

    <!-- Filters -->
    <div class="audit-filters">
      <div class="filter-group">
        <span class="filter-label">{{ t('audit.filters.time') }}</span>
        <select v-model="filters.time">
          <option value="1h">{{ t('audit.filters.time1h') }}</option>
          <option value="24h">{{ t('audit.filters.time24h') }}</option>
          <option value="7d">{{ t('audit.filters.time7d') }}</option>
          <option value="30d">{{ t('audit.filters.time30d') }}</option>
        </select>
      </div>
      <div class="filter-sep"></div>
      <div class="filter-group">
        <span class="filter-label">{{ t('audit.filters.user') }}</span>
        <select v-model="filters.user">
          <option value="">{{ t('audit.filters.all') }}</option>
          <option value="admin">admin</option>
        </select>
      </div>
      <div class="filter-sep"></div>
      <div class="filter-group">
        <span class="filter-label">{{ t('audit.filters.env') }}</span>
        <select v-model="filters.env">
          <option value="">{{ t('audit.filters.all') }}</option>
          <option v-for="env in environments" :key="env.id" :value="env.name">{{ env.name }}</option>
        </select>
      </div>
      <div class="filter-sep"></div>
      <div class="filter-group">
        <span class="filter-label">{{ t('audit.filters.operation') }}</span>
        <select v-model="filters.operation">
          <option value="">{{ t('audit.filters.all') }}</option>
          <option v-for="op in operationTypes" :key="op.value" :value="op.value">{{ op.label }}</option>
        </select>
      </div>
      <span class="filter-spacer"></span>
      <button class="btn btn-ghost btn-sm" @click="resetFilters">{{ t('audit.filters.reset') }}</button>
    </div>

    <LoadingSpinner v-if="loading" :text="t('common.loading')" />

    <ErrorState v-else-if="loadError" :message="loadError" :retry="fetchLogs" />

    <template v-else>
      <!-- Stats -->
      <div class="audit-stats">
        <div class="audit-stat">
          <span class="stat-label">{{ t('audit.stats.total') }}</span>
          <span class="stat-num text-accent">{{ stats.total }}</span>
        </div>
        <div class="audit-stat">
          <span class="stat-label">{{ t('audit.stats.success') }}</span>
          <span class="stat-num text-success">{{ stats.success }}</span>
        </div>
        <div class="audit-stat">
          <span class="stat-label">{{ t('audit.stats.failed') }}</span>
          <span class="stat-num text-danger">{{ stats.failed }}</span>
        </div>
        <div class="audit-stat">
          <span class="stat-label">{{ t('audit.stats.activeUsers') }}</span>
          <span class="stat-num" style="color: var(--info)">{{ stats.activeUsers }}</span>
        </div>
      </div>

      <!-- Log Table -->
      <div class="audit-table-wrap">
        <table class="audit-table">
          <thead>
            <tr>
              <th @click="toggleSort('time')">{{ t('audit.table.time') }} {{ sortIndicator('time') }}</th>
              <th @click="toggleSort('user')">{{ t('audit.table.user') }} {{ sortIndicator('user') }}</th>
              <th @click="toggleSort('envName')">{{ t('audit.table.environment') }} {{ sortIndicator('envName') }}</th>
              <th @click="toggleSort('operation')">{{ t('audit.table.operation') }} {{ sortIndicator('operation') }}</th>
              <th>{{ t('audit.table.summary') }}</th>
              <th @click="toggleSort('result')">{{ t('audit.table.result') }} {{ sortIndicator('result') }}</th>
            </tr>
          </thead>
          <tbody>
            <template v-for="record in sortedFilteredRecords" :key="record.id">
              <tr
                class="log-row"
                :class="{ expanded: expandedId === record.id }"
                @click="toggleDetail(record.id)"
                @contextmenu.prevent="onLogRowCtx($event, record)"
              >
                <td class="audit-time">{{ record.time }}</td>
                <td class="audit-user">{{ record.user }}</td>
                <td>
                  <span class="audit-env" @contextmenu.stop="onEnvNameCtx($event, record)">
                    <span class="env-dot"></span>
                    {{ record.envName }}
                  </span>
                </td>
                <td>
                  <span class="audit-op" :class="record.operation" @contextmenu.stop="onOpTagCtx($event, record)">
                    {{ t(`audit.ops.${record.operation}`) }}
                  </span>
                </td>
                <td class="audit-summary">{{ record.summary }}</td>
                <td>
                  <span class="audit-result" :class="record.result">
                    {{ record.result === 'ok' ? t('audit.table.success') : t('audit.table.failed') }}
                  </span>
                </td>
              </tr>
              <tr v-if="expandedId === record.id" class="audit-detail">
                <td colspan="6">
                  <div class="audit-detail-inner">
                    <div class="detail-title">{{ t(`audit.ops.${record.operation}`) }}</div>
                    <div class="detail-grid">
                      <template v-if="record.detail">
                        <template v-for="(value, key) in record.detail" :key="String(key)">
                          <span class="detail-label">{{ String(key) }}</span>
                          <span class="detail-value">{{ String(value) }}</span>
                        </template>
                      </template>
                    </div>
                  </div>
                </td>
              </tr>
            </template>
          </tbody>
        </table>
      </div>

      <!-- Pagination -->
      <div class="audit-pagination">
        <span>{{ paginationText }}</span>
        <div class="page-btns">
          <button class="page-btn" :disabled="currentPage === 1" @click="currentPage--">&lsaquo;</button>
          <button
            v-for="page in totalPages"
            :key="page"
            class="page-btn"
            :class="{ active: page === currentPage }"
            @click="currentPage = page"
          >
            {{ page }}
          </button>
          <button class="page-btn" :disabled="currentPage === totalPages" @click="currentPage++">&rsaquo;</button>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'
import { useSort } from '@/composables/useSort'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import ErrorState from '@/components/ErrorState.vue'
import { listAuditLog } from '@/api/audit'
import { listEnvironments } from '@/api/env'
import type { Environment } from '@/api/env'

const { t } = useI18n()
const { show: showMenu } = useContextMenu()

// ── Filter state ──
const filters = ref({
  time: '24h',
  user: '',
  env: '',
  operation: '',
})

function resetFilters() {
  filters.value = { time: '24h', user: '', env: '', operation: '' }
  currentPage.value = 1
}

// ── Environments (fetched from API) ──
const environments = ref<Environment[]>([])

// ── Operation types (i18n) ──
const operationTypes = computed(() => [
  { value: 'connect', label: t('audit.ops.connect') },
  { value: 'disconnect', label: t('audit.ops.disconnect') },
  { value: 'query', label: t('audit.ops.query') },
  { value: 'upload', label: t('audit.ops.upload') },
  { value: 'download', label: t('audit.ops.download') },
  { value: 'delete', label: t('audit.ops.delete') },
  { value: 'ssh_command', label: t('audit.ops.command') },
  { value: 'login', label: t('audit.ops.login') },
])

// ── Audit record type ──
interface AuditRecord {
  id: string
  time: string
  user: string
  envName: string
  operation: string
  summary: string
  result: 'ok' | 'fail'
  detail?: Record<string, unknown>
}

// ── Data state ──
const records = ref<AuditRecord[]>([])
const loading = ref(false)
const loadError = ref('')
const currentPage = ref(1)
const pageSize = 20
const apiTotal = ref(0)

// ── Stats computed from filtered records ──
const stats = computed(() => {
  const all = filteredRecords.value
  return {
    total: apiTotal.value,
    success: all.filter(r => r.result === 'ok').length,
    failed: all.filter(r => r.result === 'fail').length,
    activeUsers: new Set(all.map(r => r.user)).size,
  }
})

// ── Detail expand ──
const expandedId = ref<string | null>(null)

function toggleDetail(id: string) {
  expandedId.value = expandedId.value === id ? null : id
}

// ── Context menus ──
function onLogRowCtx(e: MouseEvent, record: AuditRecord) {
  showMenu(e, [
    { label: t('ctx.viewDetail'), action: () => toggleDetail(record.id) },
    { separator: true },
    { label: t('ctx.copySummary'), action: () => navigator.clipboard?.writeText(record.summary) },
    { label: t('ctx.copyOpType'), action: () => navigator.clipboard?.writeText(t(`audit.ops.${record.operation}`)) },
    { label: t('ctx.copyTimestamp'), action: () => navigator.clipboard?.writeText(record.time) },
  ])
}

function onOpTagCtx(e: MouseEvent, record: AuditRecord) {
  showMenu(e, [
    { label: t('ctx.filterByOp'), action: () => { filters.value.operation = record.operation } },
    { label: t('ctx.copyOpType'), action: () => navigator.clipboard?.writeText(t(`audit.ops.${record.operation}`)) },
  ])
}

function onEnvNameCtx(e: MouseEvent, record: AuditRecord) {
  showMenu(e, [
    { label: t('ctx.filterByEnv'), action: () => { filters.value.env = record.envName } },
  ])
}

// ── Time range computation ──
function computeTimeRange(timeFilter: string): { from?: string; to?: string } {
  const now = Math.floor(Date.now() / 1000)
  const to = new Date(now * 1000).toISOString()
  switch (timeFilter) {
    case '1h': return { from: new Date((now - 3600) * 1000).toISOString(), to }
    case '24h': return { from: new Date((now - 86400) * 1000).toISOString(), to }
    case '7d': return { from: new Date((now - 604800) * 1000).toISOString(), to }
    case '30d': return { from: new Date((now - 2592000) * 1000).toISOString(), to }
    default: return {}
  }
}

// ── Fetch logs from API ──
async function fetchLogs() {
  loading.value = true
  try {
    const timeRange = computeTimeRange(filters.value.time)
    const params: Record<string, string | number> = {
      page: currentPage.value,
      page_size: pageSize,
    }
    if (timeRange.from) params.from = timeRange.from
    if (timeRange.to) params.to = timeRange.to
    if (filters.value.operation) params.type = filters.value.operation

    const res = await listAuditLog(params)
    apiTotal.value = res.total

    // Map API records to display records
    const envMap = new Map(environments.value.map(e => [e.id, e.name]))
    records.value = res.items.map(item => ({
      id: item.id,
      time: item.time,
      user: item.user,
      envName: envMap.get(item.environment_id ?? '') ?? '—',
      operation: item.type,
      summary: item.summary,
      result: item.result === 'success' ? 'ok' as const : 'fail' as const,
      detail: item.detail ? (() => { try { return JSON.parse(item.detail) } catch { return undefined } })() : undefined,
    }))
  } catch {
    records.value = []
  } finally {
    loading.value = false
  }
}

// ── Client-side filtered records (user/env are not server-filtered) ──
const filteredRecords = computed(() => {
  return records.value.filter(r => {
    if (filters.value.user && r.user !== filters.value.user) return false
    if (filters.value.env && r.envName !== filters.value.env) return false
    return true
  })
})

// ── Sorting ──
const { sortKey, sortDir, toggleSort, sorted: sortedFilteredRecords } = useSort(
  () => filteredRecords.value,
  'time',
  'desc',
)

function sortIndicator(key: string) {
  if (sortKey.value !== key) return ''
  return sortDir.value === 'asc' ? '▲' : '▼'
}

// ── Watch filters and page changes ──
watch(filters, () => {
  currentPage.value = 1
  fetchLogs()
}, { deep: true })

watch(currentPage, () => fetchLogs())

onMounted(async () => {
  try {
    environments.value = await listEnvironments()
  } catch {
    // ignore
  }
  fetchLogs()
})

// ── Computed ──
const totalPages = computed(() => Math.max(1, Math.ceil(apiTotal.value / pageSize)))

const paginationText = computed(() => {
  const total = filteredRecords.value.length
  if (total === 0) return t('audit.pagination.showing', { from: 0, to: 0, total: 0 })
  const from = (currentPage.value - 1) * pageSize + 1
  const to = Math.min(currentPage.value * pageSize, total)
  return t('audit.pagination.showing', { from, to, total })
})

// ── CSV Export ──
function exportCsv() {
  const headers = [
    t('audit.table.time'),
    t('audit.table.user'),
    t('audit.table.environment'),
    t('audit.table.operation'),
    t('audit.table.summary'),
    t('audit.table.result'),
  ]
  const rows = filteredRecords.value.map(r => [
    r.time,
    r.user,
    r.envName,
    t(`audit.ops.${r.operation}`),
    r.summary,
    r.result === 'ok' ? t('audit.table.success') : t('audit.table.failed'),
  ])
  const csv = [headers, ...rows].map(row => row.map(cell => `"${cell}"`).join(',')).join('\n')
  const blob = new Blob(['﻿' + csv], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `audit-log-${new Date().toISOString().slice(0, 10)}.csv`
  a.click()
  URL.revokeObjectURL(url)
}
</script>

<style scoped>
.audit-page {
  max-width: 1100px;
}

/* ── Header ── */
.audit-header {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
  margin-bottom: var(--sp-lg);
}

.header-spacer {
  flex: 1;
}

/* ── Filters ── */
.audit-filters {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-lg) var(--sp-xl);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  margin-bottom: var(--sp-lg);
  flex-wrap: wrap;
}

.filter-group {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
}

.filter-label {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.audit-filters select {
  padding: var(--sp-xs) var(--sp-sm);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-deep);
  color: var(--text-primary);
  font-family: var(--font-body);
  font-size: var(--fs-sm);
  outline: none;
  height: 28px;
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3E%3Cpath fill='%238B949E' d='M4 6L0 2h8z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 6px center;
  padding-right: 20px;
}

.filter-sep {
  width: 1px;
  height: 20px;
  background: var(--border);
  margin: 0 var(--sp-xs);
}

.filter-spacer {
  flex: 1;
}

/* ── Stats ── */
.audit-stats {
  display: flex;
  gap: var(--sp-md);
  margin-bottom: var(--sp-lg);
}

.audit-stat {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-sm) var(--sp-lg);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  font-size: var(--fs-sm);
}

.stat-label {
  color: var(--text-muted);
}

.stat-num {
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: var(--fs-lg);
}

.text-accent { color: var(--accent); }
.text-success { color: var(--success); }
.text-danger { color: var(--danger); }

/* ── Table ── */
.audit-table-wrap {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
  margin-bottom: 0;
}

.audit-table {
  width: 100%;
  border-collapse: collapse;
}

.audit-table th {
  padding: var(--sp-md) var(--sp-lg);
  text-align: left;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
  background: var(--bg-elevated);
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  user-select: none;
}

.audit-table th:hover {
  color: var(--text-primary);
}

.audit-table td {
  padding: var(--sp-md) var(--sp-lg);
  border-bottom: 1px solid var(--border);
  font-size: var(--fs-sm);
}

.audit-table tr:last-child td {
  border-bottom: none;
}

.log-row {
  cursor: pointer;
  transition: background var(--transition-fast);
}

.log-row:hover td {
  background: var(--bg-hover);
}

.log-row.expanded td {
  background: var(--bg-deep);
}

.audit-time {
  font-family: var(--font-mono);
  color: var(--text-muted);
  white-space: nowrap;
}

.audit-user {
  font-family: var(--font-mono);
  font-weight: 500;
}

.audit-env {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: var(--fs-xs);
  color: var(--text-secondary);
}

.env-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--success);
  flex-shrink: 0;
}

.audit-op {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--sp-sm);
  border-radius: var(--radius-sm);
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  font-weight: 600;
}

.audit-op.connect { background: rgba(63,185,80,0.12); color: var(--success); }
.audit-op.disconnect { background: rgba(139,148,158,0.15); color: var(--text-muted); }
.audit-op.query { background: rgba(88,166,255,0.12); color: var(--info); }
.audit-op.upload { background: rgba(139,92,246,0.12); color: #8B5CF6; } /* unique purple, no CSS var */
.audit-op.download { background: rgba(63,185,80,0.12); color: var(--success); }
.audit-op.delete { background: rgba(248,81,73,0.12); color: var(--danger); }
.audit-op.login { background: rgba(232,145,45,0.12); color: var(--accent); }
.audit-op.ssh_command { background: rgba(210,153,34,0.12); color: var(--warning); }
.audit-op.connect { box-shadow: 0 0 6px var(--success-glow); }
.audit-op.login { box-shadow: 0 0 6px var(--accent-glow); }

.audit-summary {
  color: var(--text-secondary);
}

.audit-result {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
}

.audit-result.ok { color: var(--success); }
.audit-result.fail { color: var(--danger); }

/* ── Detail Panel ── */
.audit-detail td {
  padding: 0 !important;
  background: var(--bg-deep);
}

.audit-detail-inner {
  padding: var(--sp-lg) var(--sp-xl);
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  line-height: 1.6;
}

.audit-detail-inner .detail-title {
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: var(--sp-sm);
}

.audit-detail-inner .detail-grid {
  display: grid;
  grid-template-columns: 140px 1fr;
  gap: var(--sp-xs) var(--sp-lg);
}

.audit-detail-inner .detail-label {
  color: var(--text-muted);
}

.audit-detail-inner .detail-value {
  color: var(--text-primary);
  word-break: break-all;
}

.audit-detail-inner .detail-cmd {
  margin-top: var(--sp-md);
  padding: var(--sp-md);
  background: var(--bg-surface);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  white-space: pre-wrap;
}

/* ── Pagination ── */
.audit-pagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-md) var(--sp-xl);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-top: none;
  border-radius: 0 0 var(--radius-lg) var(--radius-lg);
  font-size: var(--fs-sm);
  color: var(--text-muted);
}

.page-btns {
  display: flex;
  gap: var(--sp-xs);
}

.page-btn {
  padding: var(--sp-xs) var(--sp-sm);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-deep);
  color: var(--text-secondary);
  font-size: var(--fs-xs);
  cursor: pointer;
  font-family: var(--font-mono);
  transition: all var(--transition-fast);
}

.page-btn:hover:not(:disabled) {
  color: var(--text-primary);
  border-color: var(--text-muted);
}

.page-btn.active {
  background: var(--accent);
  color: #000; /* intentional: black text on accent button for contrast */
  border-color: var(--accent);
  font-weight: 600;
  box-shadow: 0 0 8px var(--accent-glow);
}

.page-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

/* ── Mobile ── */
@media (max-width: 767px) {
  .audit-filters {
    flex-direction: column;
    align-items: stretch;
  }

  .filter-group {
    justify-content: space-between;
  }

  .filter-sep {
    display: none;
  }

  .filter-spacer {
    display: none;
  }

  .audit-stats {
    flex-wrap: wrap;
  }

  .audit-table th:nth-child(4),
  .audit-table td:nth-child(4),
  .audit-table th:nth-child(5),
  .audit-table td:nth-child(5) {
    display: none;
  }
}
</style>
