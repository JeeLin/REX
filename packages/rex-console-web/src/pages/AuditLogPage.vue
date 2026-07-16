<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { auditApi, type AuditEntry } from '@/api/audit'
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import EmptyState from '@/components/ui/EmptyState.vue'

const { t } = useI18n()
const entries = ref<AuditEntry[]>([])
const loading = ref(true)
const actionFilter = ref('')
const resultFilter = ref('')

const actionOptions = [
  { label: 'All', value: '' },
  { label: 'ENV_CREATE', value: 'ENV_CREATE' },
  { label: 'ENV_UPDATE', value: 'ENV_UPDATE' },
  { label: 'ENV_DELETE', value: 'ENV_DELETE' },
  { label: 'RESOURCE_CREATE', value: 'RESOURCE_CREATE' },
  { label: 'RESOURCE_DELETE', value: 'RESOURCE_DELETE' },
  { label: 'AGENT_REGISTER', value: 'AGENT_REGISTER' },
]

async function fetchEntries() {
  loading.value = true
  try {
    entries.value = await auditApi.query({
      action: actionFilter.value || undefined,
      result: resultFilter.value || undefined,
      limit: 100,
    })
  } catch {
    entries.value = []
  } finally {
    loading.value = false
  }
}

onMounted(fetchEntries)

function resultBadge(result: string) {
  return result === 'success' ? 'success' : 'danger'
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
</script>

<template>
  <div class="audit-page">
    <header class="page-header">
      <h1 class="page-title">{{ t('auditLog.title') }}</h1>
      <div class="filters">
        <select v-model="actionFilter" class="filter-select" @change="fetchEntries">
          <option v-for="opt in actionOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
        </select>
        <select v-model="resultFilter" class="filter-select" @change="fetchEntries">
          <option value="">{{ t('auditLog.allResults') }}</option>
          <option value="success">{{ t('auditLog.success') }}</option>
          <option value="failure">{{ t('auditLog.failure') }}</option>
        </select>
        <Button variant="secondary" size="sm" @click="fetchEntries">{{ t('common.refresh') }}</Button>
      </div>
    </header>

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
            <th>{{ t('auditLog.time') }}</th>
            <th>{{ t('auditLog.action') }}</th>
            <th>{{ t('auditLog.target') }}</th>
            <th>{{ t('auditLog.result') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="entry in entries" :key="entry.id">
            <td class="mono">{{ timeAgo(entry.time) }}</td>
            <td>
              <Badge :tone="entry.action.includes('DELETE') ? 'danger' : entry.action.includes('CREATE') ? 'success' : 'info'">
                {{ entry.action }}
              </Badge>
            </td>
            <td>{{ entry.target || '—' }}</td>
            <td><Badge :tone="resultBadge(entry.result)">{{ entry.result }}</Badge></td>
          </tr>
        </tbody>
      </table>
    </Card>
  </div>
</template>

<style scoped>
.audit-page { max-width: 900px; }
.page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--space-6); flex-wrap: wrap; gap: var(--space-3); }
.page-title { font-size: var(--text-xl); font-weight: 600; color: var(--text-primary); }
.filters { display: flex; gap: var(--space-2); align-items: center; }
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
.log-table tr:hover td { background: var(--bg-hover); }
.muted { color: var(--text-muted); }
.mono { font-family: var(--font-mono); }
</style>
