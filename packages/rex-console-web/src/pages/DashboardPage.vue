<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { dashboardApi, type DashboardStats } from '@/api/dashboard'
import { useEnvironmentsStore } from '@/stores/environments'
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'

const router = useRouter()
const store = useEnvironmentsStore()

const stats = ref<DashboardStats>({ environment_count: 0, resource_count: 0, online_agents: 0 })
const loading = ref(true)

onMounted(async () => {
  try {
    const [s] = await Promise.all([
      dashboardApi.stats(),
      store.fetchEnvironments(),
    ])
    stats.value = s
  } catch {
    // ignore
  } finally {
    loading.value = false
  }
})

function agentStatus(status: string | null): StatusDotStatus {
  if (status === 'online') return 'online'
  return 'offline'
}
</script>

<template>
  <div class="dashboard">
    <h1 class="page-title">Dashboard</h1>

    <div class="stats-grid">
      <Card class="stat-card">
        <div class="stat-value mono">{{ stats.environment_count }}</div>
        <div class="stat-label">Environments</div>
      </Card>
      <Card class="stat-card">
        <div class="stat-value mono">{{ stats.resource_count }}</div>
        <div class="stat-label">Resources</div>
      </Card>
      <Card class="stat-card">
        <div class="stat-value mono">{{ stats.online_agents }}</div>
        <div class="stat-label">Agents Online</div>
      </Card>
    </div>

    <h2 class="section-title">Environments</h2>
    <div v-if="store.environments.length === 0" class="muted">No environments yet</div>
    <div v-else class="env-grid">
      <Card
        v-for="env in store.environments"
        :key="env.id"
        class="env-card"
        @click="router.push(`/environments/${env.id}`)"
      >
        <div class="env-name">{{ env.name }}</div>
        <div class="env-meta">
          <StatusDot :status="agentStatus(env.agent_status)" />
          <span class="muted">{{ env.agent_status || 'no agent' }}</span>
          <Badge tone="accent" style="margin-left: auto">{{ env.resource_count }} resources</Badge>
        </div>
      </Card>
    </div>
  </div>
</template>

<style scoped>
.dashboard { max-width: 900px; }
.page-title { font-size: var(--text-xl); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-6); }
.stats-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--space-4); margin-bottom: var(--space-6); }
.stat-card { text-align: center; padding: var(--space-4); }
.stat-value { font-size: var(--text-2xl); font-weight: 700; color: var(--accent); }
.stat-label { font-size: var(--text-sm); color: var(--text-muted); margin-top: var(--space-1); }
.section-title { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-3); }
.env-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: var(--space-3); }
.env-card { cursor: pointer; transition: border-color var(--transition); }
.env-card:hover { border-color: var(--accent); }
.env-name { font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-2); }
.env-meta { display: flex; align-items: center; gap: var(--space-2); font-size: var(--text-sm); }
.muted { color: var(--text-muted); }
.mono { font-family: var(--font-mono); }
</style>
