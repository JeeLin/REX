<script setup lang="ts">
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'

const stats = [
  { label: 'Connections', value: '12', icon: '⬡', color: 'var(--accent)' },
  { label: 'Online', value: '8', icon: '●', color: 'var(--success)' },
  { label: 'Offline', value: '4', icon: '●', color: 'var(--danger)' },
  { label: 'Active Sessions', value: '3', icon: '▸', color: 'var(--info)' },
]

const recent = [
  { name: 'Web Server', host: '10.0.1.5', protocol: 'SSH', status: 'online' as StatusDotStatus, time: '2m ago' },
  { name: 'DB Primary', host: 'db.internal', protocol: 'MySQL', status: 'online' as StatusDotStatus, time: '5m ago' },
  { name: 'Cache', host: 'cache.local', protocol: 'Redis', status: 'offline' as StatusDotStatus, time: '1h ago' },
]
</script>

<template>
  <div class="dashboard">
    <header class="dash-header">
      <h1 class="dash-title">Dashboard</h1>
      <Button variant="primary" size="sm">+ New Connection</Button>
    </header>

    <!-- Stats -->
    <div class="stats-grid">
      <div v-for="s in stats" :key="s.label" class="stat-card">
        <div class="stat-icon" :style="{ color: s.color }">{{ s.icon }}</div>
        <div class="stat-value mono">{{ s.value }}</div>
        <div class="stat-label">{{ s.label }}</div>
      </div>
    </div>

    <!-- Recent -->
    <Card title="Recent Connections">
      <div class="recent-list">
        <div v-for="conn in recent" :key="conn.host" class="recent-item">
          <StatusDot :status="conn.status" />
          <span class="recent-name">{{ conn.name }}</span>
          <span class="recent-proto mono muted">{{ conn.protocol }}</span>
          <span class="recent-host mono muted">{{ conn.host }}</span>
          <span class="recent-time muted">{{ conn.time }}</span>
        </div>
      </div>
    </Card>
  </div>
</template>

<style scoped>
.dashboard {
  max-width: 900px;
}
.dash-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-6);
}
.dash-title {
  font-size: var(--text-xl);
  font-weight: 600;
  color: var(--text-primary);
}

/* Stats */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--space-4);
  margin-bottom: var(--space-6);
}
.stat-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
.stat-icon {
  font-size: var(--text-lg);
  margin-bottom: var(--space-1);
}
.stat-value {
  font-size: var(--text-2xl);
  font-weight: 700;
  color: var(--text-primary);
}
.stat-label {
  font-size: var(--text-sm);
  color: var(--text-muted);
}

/* Recent */
.recent-list {
  display: flex;
  flex-direction: column;
}
.recent-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) 0;
  border-bottom: 1px solid var(--border-subtle);
  font-size: var(--text-sm);
}
.recent-item:last-child {
  border-bottom: none;
}
.recent-name {
  color: var(--text-primary);
  min-width: 100px;
}
.recent-proto {
  font-size: var(--text-xs);
  width: 60px;
}
.recent-host {
  font-size: var(--text-xs);
}
.recent-time {
  margin-left: auto;
  font-size: var(--text-xs);
}
</style>
