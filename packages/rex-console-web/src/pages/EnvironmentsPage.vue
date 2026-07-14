<script setup lang="ts">
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import EmptyState from '@/components/ui/EmptyState.vue'

interface Env {
  name: string
  description: string
  connections: number
  icon: string
}

const environments: Env[] = [
  { name: 'Production', description: '生产环境服务器和数据库', connections: 8, icon: '🚀' },
  { name: 'Staging', description: '预发布环境', connections: 4, icon: '🔬' },
  { name: 'Development', description: '开发和测试环境', connections: 6, icon: '🔧' },
]

const hasEnvironments = environments.length > 0
</script>

<template>
  <div class="environments">
    <header class="page-header">
      <h1 class="page-title">Environments</h1>
      <Button variant="primary" size="sm">+ New Environment</Button>
    </header>

    <EmptyState
      v-if="!hasEnvironments"
      icon="⛁"
      title="No environments yet"
      description="Environments group your connections by context — production, staging, or development."
    >
      <Button variant="primary">Create Environment</Button>
    </EmptyState>

    <div v-else class="env-grid">
      <Card v-for="env in environments" :key="env.name" class="env-card">
        <div class="env-card-header">
          <span class="env-icon">{{ env.icon }}</span>
          <div class="env-info">
            <div class="env-name">{{ env.name }}</div>
            <div class="env-desc muted">{{ env.description }}</div>
          </div>
        </div>
        <div class="env-footer">
          <Badge tone="accent">{{ env.connections }} connections</Badge>
        </div>
      </Card>
    </div>
  </div>
</template>

<style scoped>
.environments {
  max-width: 900px;
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
.env-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: var(--space-4);
}
.env-card {
  cursor: pointer;
  transition: border-color var(--transition);
}
.env-card:hover {
  border-color: var(--accent);
}
.env-card-header {
  display: flex;
  gap: var(--space-3);
  margin-bottom: var(--space-4);
}
.env-icon {
  font-size: 24px;
}
.env-info {
  flex: 1;
}
.env-name {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
}
.env-desc {
  font-size: var(--text-sm);
  margin-top: var(--space-1);
}
.env-footer {
  display: flex;
  justify-content: flex-end;
}
</style>
