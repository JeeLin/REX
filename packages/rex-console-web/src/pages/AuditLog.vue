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
          <option v-for="env in environments" :key="env.id" :value="env.id">{{ env.name }}</option>
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
            <th>{{ t('audit.table.time') }}</th>
            <th>{{ t('audit.table.user') }}</th>
            <th>{{ t('audit.table.environment') }}</th>
            <th>{{ t('audit.table.operation') }}</th>
            <th>{{ t('audit.table.summary') }}</th>
            <th>{{ t('audit.table.result') }}</th>
          </tr>
        </thead>
        <tbody>
          <template v-for="record in paginatedRecords" :key="record.id">
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
                    <template v-for="field in record.detailFields" :key="field.label">
                      <span class="detail-label">{{ field.label }}</span>
                      <span class="detail-value">{{ field.value }}</span>
                    </template>
                  </div>
                  <div v-if="record.detailCommand" class="detail-cmd">{{ record.detailCommand }}</div>
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
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'

const { t } = useI18n()
const { show: showMenu } = useContextMenu()

// ── Filter state ──
const filters = ref({
  time: '24h',
  user: '',
  env: '',
  operation: '',
})

const environments = [
  { id: 'env_ali', name: '阿里云' },
  { id: 'env_pi', name: '树莓派集群' },
  { id: 'env_nas', name: '家庭 NAS' },
]

const operationTypes = [
  { value: 'connect', label: 'SSH 连接' },
  { value: 'disconnect', label: '连接断开' },
  { value: 'query', label: 'SQL 查询' },
  { value: 'upload', label: '文件上传' },
  { value: 'download', label: '文件下载' },
  { value: 'ssh_command', label: 'SSH 命令' },
  { value: 'login', label: '登录' },
]

function resetFilters() {
  filters.value = { time: '24h', user: '', env: '', operation: '' }
}

// ── Detail expand ──
const expandedId = ref<string | null>(null)

function toggleDetail(id: string) {
  expandedId.value = expandedId.value === id ? null : id
}

// ── Mock data ──
interface DetailField {
  label: string
  value: string
}

interface AuditRecord {
  id: string
  time: string
  user: string
  envId: string
  envName: string
  operation: string
  summary: string
  result: 'ok' | 'fail'
  detailFields: DetailField[]
  detailCommand?: string
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
    { label: t('ctx.filterByEnv'), action: () => { filters.value.env = record.envId } },
  ])
}

const records = ref<AuditRecord[]>([
  {
    id: 'log_001',
    time: '16:51:30',
    user: 'admin',
    envId: 'env_ali',
    envName: '阿里云',
    operation: 'connect',
    summary: 'root@192.168.1.100:22',
    result: 'ok',
    detailFields: [
      { label: t('audit.detail.protocol'), value: 'SSH' },
      { label: t('audit.detail.target'), value: '192.168.1.100:22' },
      { label: t('audit.detail.user'), value: 'root' },
      { label: t('audit.detail.agent'), value: 'agt_7x8k9m (阿里云 Agent)' },
      { label: t('audit.detail.duration'), value: '88 秒' },
      { label: t('audit.detail.transfer'), value: '↑ 1.2KB ↓ 15.6KB' },
    ],
    detailCommand: 'root@prod:~$ uptime\n16:45:12 up 14 days,  3:21,  1 user,  load average: 0.12, 0.08, 0.05\n\nroot@prod:~$ docker ps --format "table {{.Names}}\\t{{.Status}}\\t{{.Ports}}"\nNAMES                STATUS          PORTS\nrex-app              Up 2 days       0.0.0.0:3000->3000/tcp',
  },
  {
    id: 'log_002',
    time: '16:49:01',
    user: 'admin',
    envId: 'env_ali',
    envName: '阿里云',
    operation: 'query',
    summary: 'production_db · SELECT 5 行',
    result: 'ok',
    detailFields: [
      { label: t('audit.detail.database'), value: 'production_db (MySQL)' },
      { label: t('audit.detail.resource'), value: '主数据库 · db.internal:3306' },
      { label: t('audit.detail.elapsed'), value: '23ms' },
      { label: t('audit.detail.rows'), value: '5' },
    ],
    detailCommand: `SELECT
  u.id, u.username, u.email,
  COUNT(o.id) AS order_count,
  SUM(o.total_amount) AS total_spent
FROM users u
LEFT JOIN orders o ON o.user_id = u.id
WHERE u.created_at >= '2024-01-01'
GROUP BY u.id, u.username, u.email
HAVING COUNT(o.id) > 5
ORDER BY total_spent DESC
LIMIT 100;`,
  },
  {
    id: 'log_003',
    time: '16:47:02',
    user: 'admin',
    envId: 'env_pi',
    envName: '树莓派集群',
    operation: 'upload',
    summary: 'docker-compose.yml → /opt/rex/',
    result: 'ok',
    detailFields: [
      { label: t('audit.detail.protocol'), value: 'SFTP' },
      { label: t('audit.detail.resource'), value: 'Web 文件服务器' },
      { label: t('audit.detail.fileSize'), value: '564 B' },
      { label: t('audit.detail.targetPath'), value: '/opt/rex/docker-compose.yml' },
    ],
  },
  {
    id: 'log_004',
    time: '16:45:13',
    user: 'admin',
    envId: 'env_ali',
    envName: '阿里云',
    operation: 'login',
    summary: 'admin · 172.16.0.52',
    result: 'ok',
    detailFields: [
      { label: t('audit.detail.user'), value: 'admin' },
      { label: t('audit.detail.ip'), value: '172.16.0.52' },
      { label: t('audit.detail.browser'), value: 'Chrome 125.0 (macOS)' },
      { label: t('audit.detail.authMethod'), value: '密码登录' },
    ],
  },
  {
    id: 'log_005',
    time: '16:43:22',
    user: 'admin',
    envId: 'env_ali',
    envName: '阿里云',
    operation: 'ssh_command',
    summary: 'root@192.168.1.100 · docker ps',
    result: 'ok',
    detailFields: [
      { label: t('audit.detail.protocol'), value: 'SSH' },
      { label: t('audit.detail.resource'), value: '云服务器 · 192.168.1.100:22' },
      { label: t('audit.detail.execUser'), value: 'root' },
      { label: t('audit.detail.elapsed'), value: '0.3s' },
    ],
    detailCommand: 'docker ps --format "table {{.Names}}\\t{{.Status}}\\t{{.Ports}}"',
  },
  {
    id: 'log_006',
    time: '16:40:05',
    user: 'admin',
    envId: 'env_ali',
    envName: '阿里云',
    operation: 'download',
    summary: 'backup-20240610.tar.gz (23.4 MB)',
    result: 'ok',
    detailFields: [
      { label: t('audit.detail.protocol'), value: 'SFTP' },
      { label: t('audit.detail.resource'), value: 'Web 文件服务器' },
      { label: t('audit.detail.fileSize'), value: '23.4 MB' },
      { label: t('audit.detail.sourcePath'), value: '/opt/rex/backup-20240610.tar.gz' },
    ],
  },
  {
    id: 'log_007',
    time: '16:38:12',
    user: 'admin',
    envId: 'env_nas',
    envName: '家庭 NAS',
    operation: 'connect',
    summary: 'root@192.168.0.100:22',
    result: 'fail',
    detailFields: [
      { label: t('audit.detail.protocol'), value: 'SSH' },
      { label: t('audit.detail.target'), value: '192.168.0.100:22' },
      { label: t('audit.detail.user'), value: 'root' },
    ],
  },
  {
    id: 'log_008',
    time: '16:35:00',
    user: 'admin',
    envId: 'env_nas',
    envName: '家庭 NAS',
    operation: 'disconnect',
    summary: 'Agent agt_9p2q4r 离线',
    result: 'fail',
    detailFields: [
      { label: t('audit.detail.agent'), value: 'agt_9p2q4r' },
    ],
  },
  {
    id: 'log_009',
    time: '16:30:45',
    user: 'admin',
    envId: 'env_pi',
    envName: '树莓派集群',
    operation: 'query',
    summary: 'test_db · SELECT 12 行',
    result: 'ok',
    detailFields: [
      { label: t('audit.detail.database'), value: 'test_db (MySQL)' },
      { label: t('audit.detail.resource'), value: '测试数据库 · db.test:3306' },
      { label: t('audit.detail.elapsed'), value: '12ms' },
      { label: t('audit.detail.rows'), value: '12' },
    ],
  },
  {
    id: 'log_010',
    time: '16:25:10',
    user: 'admin',
    envId: 'env_ali',
    envName: '阿里云',
    operation: 'delete',
    summary: '/tmp/debug.log',
    result: 'ok',
    detailFields: [
      { label: t('audit.detail.protocol'), value: 'SFTP' },
      { label: t('audit.detail.resource'), value: '云服务器' },
      { label: t('audit.detail.fileSize'), value: '12 KB' },
      { label: t('audit.detail.targetPath'), value: '/tmp/debug.log' },
    ],
  },
  {
    id: 'log_011',
    time: '16:20:33',
    user: 'admin',
    envId: 'env_ali',
    envName: '阿里云',
    operation: 'connect',
    summary: 'root@192.168.1.100:22',
    result: 'ok',
    detailFields: [
      { label: t('audit.detail.protocol'), value: 'SSH' },
      { label: t('audit.detail.target'), value: '192.168.1.100:22' },
      { label: t('audit.detail.user'), value: 'root' },
      { label: t('audit.detail.agent'), value: 'agt_7x8k9m (阿里云 Agent)' },
      { label: t('audit.detail.duration'), value: '125 秒' },
      { label: t('audit.detail.transfer'), value: '↑ 2.4KB ↓ 28.1KB' },
    ],
  },
])

// ── Filtered records ──
const filteredRecords = computed(() => {
  return records.value.filter(r => {
    if (filters.value.user && r.user !== filters.value.user) return false
    if (filters.value.env && r.envId !== filters.value.env) return false
    if (filters.value.operation && r.operation !== filters.value.operation) return false
    return true
  })
})

// ── Stats ──
const stats = computed(() => {
  const all = filteredRecords.value
  return {
    total: all.length,
    success: all.filter(r => r.result === 'ok').length,
    failed: all.filter(r => r.result === 'fail').length,
    activeUsers: new Set(all.map(r => r.user)).size,
  }
})

// ── Pagination ──
const currentPage = ref(1)
const pageSize = 10

const totalPages = computed(() => Math.max(1, Math.ceil(filteredRecords.value.length / pageSize)))

const paginatedRecords = computed(() => {
  const start = (currentPage.value - 1) * pageSize
  return filteredRecords.value.slice(start, start + pageSize)
})

const paginationText = computed(() => {
  const total = filteredRecords.value.length
  if (total === 0) return '0 条记录'
  const from = (currentPage.value - 1) * pageSize + 1
  const to = Math.min(currentPage.value * pageSize, total)
  return `显示 ${from}-${to} / 共 ${total} 条`
})

// ── CSV Export ──
function exportCsv() {
  const headers = ['时间', '用户', '环境', '操作', '摘要', '结果']
  const rows = filteredRecords.value.map(r => [
    r.time,
    r.user,
    r.envName,
    t(`audit.ops.${r.operation}`),
    r.summary,
    r.result === 'ok' ? '成功' : '失败',
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
.audit-op.upload { background: rgba(139,92,246,0.12); color: #8B5CF6; }
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
  color: #000;
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
