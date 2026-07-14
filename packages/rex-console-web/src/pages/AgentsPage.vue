<script setup lang="ts">
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import Table from '@/components/ui/Table.vue'

interface Agent {
  id: string
  name: string
  host: string
  status: StatusDotStatus
  version: string
  lastSeen: string
}

const agents: Agent[] = [
  { id: '1', name: 'Agent-US-East', host: 'agent-1.internal', status: 'online', version: '0.1.0', lastSeen: '12s ago' },
  { id: '2', name: 'Agent-EU-West', host: 'agent-2.internal', status: 'connecting', version: '0.1.0', lastSeen: '—' },
  { id: '3', name: 'Agent-AP-South', host: 'agent-3.internal', status: 'offline', version: '0.0.9', lastSeen: '2h ago' },
]

const columns = [
  { key: 'name', label: 'Agent' },
  { key: 'host', label: 'Host', width: '180px' },
  { key: 'status', label: 'Status', width: '110px', align: 'left' as const },
  { key: 'version', label: 'Version', width: '90px' },
  { key: 'lastSeen', label: 'Last Seen', width: '100px' },
]
</script>

<template>
  <div class="agents">
    <header class="page-header">
      <h1 class="page-title">Agents</h1>
      <div class="page-actions">
        <Button variant="secondary" size="sm">Deployment Guide</Button>
        <Button variant="primary" size="sm">+ Register Agent</Button>
      </div>
    </header>

    <Card :padded="false" title="Registered Agents">
      <Table :columns="columns" :rows="agents" :row-key="(r: Agent) => r.id">
        <template #cell-name="{ row }">
          <span class="agent-name">{{ row.name }}</span>
        </template>
        <template #cell-status="{ row }">
          <Badge :tone="row.status === 'online' ? 'success' : row.status === 'connecting' ? 'warning' : 'danger'">
            <StatusDot :status="row.status" />
            {{ row.status }}
          </Badge>
        </template>
        <template #cell-version="{ row }">
          <span class="mono">{{ row.version }}</span>
        </template>
      </Table>
    </Card>
  </div>
</template>

<style scoped>
.agents {
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
.page-actions {
  display: flex;
  gap: var(--space-2);
}
.agent-name {
  color: var(--text-primary);
  font-weight: 500;
}
</style>
