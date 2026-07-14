<script setup lang="ts">
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Table from '@/components/ui/Table.vue'
import EmptyState from '@/components/ui/EmptyState.vue'

interface LogEntry {
  id: string
  time: string
  action: string
  target: string
  result: 'success' | 'failure' | 'warning'
}

const logs: LogEntry[] = [
  { id: '1', time: '2026-07-14 19:42:15', action: 'SSH Connect', target: '10.0.1.5', result: 'success' },
  { id: '2', time: '2026-07-14 19:40:03', action: 'MySQL Query', target: 'db.internal', result: 'success' },
  { id: '3', time: '2026-07-14 19:38:22', action: 'Redis Connect', target: 'cache.local', result: 'failure' },
  { id: '4', time: '2026-07-14 19:35:11', action: 'SFTP Upload', target: 'nas.home', result: 'warning' },
]

const columns = [
  { key: 'time', label: 'Time', width: '180px' },
  { key: 'action', label: 'Action' },
  { key: 'target', label: 'Target', width: '140px' },
  { key: 'result', label: 'Result', width: '100px', align: 'right' as const },
]

const hasLogs = logs.length > 0
</script>

<template>
  <div class="audit-log">
    <header class="page-header">
      <h1 class="page-title">Audit Log</h1>
    </header>

    <EmptyState
      v-if="!hasLogs"
      icon="☰"
      title="No audit entries"
      description="Connection attempts, queries, and file transfers will appear here."
    />

    <Card v-else :padded="false" title="Recent Activity">
      <Table :columns="columns" :rows="logs" :row-key="(r: LogEntry) => r.id">
        <template #cell-action="{ row }">
          <span class="mono">{{ row.action }}</span>
        </template>
        <template #cell-result="{ row }">
          <Badge :tone="row.result === 'success' ? 'success' : row.result === 'failure' ? 'danger' : 'warning'">
            {{ row.result }}
          </Badge>
        </template>
      </Table>
    </Card>
  </div>
</template>

<style scoped>
.audit-log {
  max-width: 1000px;
}
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-6);
}
.page-title {
  font-size: var(--text-xl);
  font-weight: 600;
  color: var(--text-primary);
}
</style>
