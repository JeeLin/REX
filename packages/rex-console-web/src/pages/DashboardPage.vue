<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { dashboardApi, type DashboardStats } from '@/api/dashboard'
import { useEnvironmentsStore } from '@/stores/environments'
import StatusDot from '@/components/ui/StatusDot.vue'
import Badge from '@/components/ui/Badge.vue'
import type { Resource } from '@/api/resources'
import { agentStatus } from '@/utils/status'

const { t } = useI18n()
const router = useRouter()
const store = useEnvironmentsStore()

const stats = ref<DashboardStats>({ environment_count: 0, resource_count: 0, online_agents: 0 })
const recentResources = ref<Resource[]>([])
const loading = ref(true)
const todayStr = new Date().toLocaleDateString()

onMounted(async () => {
  try {
    const [s, recent] = await Promise.all([
      dashboardApi.stats(),
      dashboardApi.recent().catch(() => []),
      store.fetchEnvironments(),
    ])
    stats.value = s
    recentResources.value = recent.slice(0, 6)
  } catch {
    // ignore
  } finally {
    loading.value = false
  }
})

const protoTone: Record<string, 'success' | 'info' | 'purple' | 'danger' | 'warning' | 'accent'> = {
  ssh: 'success', sftp: 'purple', mysql: 'info',
  postgresql: 'purple', redis: 'danger', sqlite: 'warning', s3: 'accent',
}

const statCards = [
  { key: 'environments', icon: '◈', color: 'var(--info)', bg: 'var(--info-soft)', field: 'environment_count' as const },
  { key: 'resources', icon: '◉', color: 'var(--purple)', bg: 'var(--purple-soft)', field: 'resource_count' as const },
  { key: 'agentsOnline', icon: '⬡', color: 'var(--success)', bg: 'var(--success-soft)', field: 'online_agents' as const },
]
</script>

<template>
  <div class="page-container dashboard">
    <!-- Header -->
    <header class="page-header">
      <div class="page-header-left">
        <h1 class="page-title mono">{{ t('dashboard.title') }}</h1>
        <span class="page-subtitle">{{ t('dashboard.subtitle', 'System overview') }}</span>
      </div>
      <div class="page-header-right mono muted">
        {{ todayStr }}
      </div>
    </header>

    <!-- Loading -->
    <div v-if="loading" class="dash-loading">
      <div class="dash-loading-spinner" />
      <span class="muted">{{ t('common.loadingEllipsis') }}</span>
    </div>

    <template v-else>
      <!-- Stats Row -->
      <div class="stats-row">
        <div
          v-for="(card, idx) in statCards"
          :key="card.key"
          class="stat-tile"
          :style="{ '--stat-color': card.color, '--stat-bg': card.bg }"
        >
          <div class="stat-tile-icon">{{ card.icon }}</div>
          <div class="stat-tile-body">
            <span class="stat-tile-value mono">
              {{ stats[card.field] }}
            </span>
            <span class="stat-tile-label">{{ t(`dashboard.${card.key}`) }}</span>
          </div>
          <div class="stat-tile-glow" />
        </div>
      </div>

      <!-- Quick Connect -->
      <section v-if="recentResources.length" class="dash-section">
        <div class="dash-section-header">
          <h2 class="dash-section-title">
            <span class="section-icon">⚡</span>
            {{ t('dashboard.recentResources', 'Quick Connect') }}
          </h2>
          <span class="dash-section-count muted mono">{{ recentResources.length }}</span>
        </div>
        <div class="recent-grid">
          <button
            v-for="res in recentResources"
            :key="res.id"
            class="recent-tile"
            @click="router.push(`/workspace?resource=${res.id}`)"
          >
            <div class="recent-tile-top">
              <Badge :tone="protoTone[res.protocol] || 'neutral'" size="sm">{{ res.protocol.toUpperCase() }}</Badge>
            </div>
            <div class="recent-tile-name mono">{{ res.name }}</div>
            <div class="recent-tile-host muted">{{ res.host }}{{ res.port ? `:${res.port}` : '' }}</div>
          </button>
        </div>
      </section>

      <!-- Environments -->
      <section class="dash-section">
        <div class="dash-section-header">
          <h2 class="dash-section-title">
            <span class="section-icon">⛁</span>
            {{ t('dashboard.environmentsSection') }}
          </h2>
          <span class="dash-section-count muted mono">{{ store.environments.length }}</span>
        </div>

        <div v-if="store.environments.length === 0" class="dash-empty">
          <span class="dash-empty-icon">⛁</span>
          <span class="dash-empty-text">{{ t('dashboard.noEnvironments') }}</span>
          <button class="dash-empty-action" @click="router.push('/environments')">
            {{ t('dashboard.createFirst', 'Create one →') }}
          </button>
        </div>

        <div v-else class="env-grid">
          <button
            v-for="env in store.environments"
            :key="env.id"
            class="env-tile"
            @click="router.push(`/environments/${env.id}`)"
          >
            <div class="env-tile-header">
              <div class="env-tile-name-row">
                <StatusDot :status="agentStatus(env.agent_status)" />
                <span class="env-tile-name mono">{{ env.name }}</span>
              </div>
              <span class="env-tile-desc muted">{{ env.description || '—' }}</span>
            </div>
            <div class="env-tile-footer">
              <div class="env-tile-stat">
                <span class="env-tile-stat-val mono">{{ env.resource_count }}</span>
                <span class="env-tile-stat-lbl">{{ t('common.resources') }}</span>
              </div>
              <div class="env-tile-stat">
                <span class="env-tile-stat-val mono" :class="{ 'text-success': env.agent_status === 'online', 'text-muted': !env.agent_status }">
                  {{ env.agent_status || '—' }}
                </span>
                <span class="env-tile-stat-lbl">Agent</span>
              </div>
              <span class="env-tile-arrow muted">→</span>
            </div>
          </button>
        </div>
      </section>
    </template>
  </div>
</template>

<style scoped>
/* ========== Layout ========== */
.dashboard {}

/* ========== Loading ========== */
.dash-loading { display: flex; align-items: center; justify-content: center; gap: var(--space-3); padding: var(--space-8); }
.dash-loading-spinner {
  width: 16px; height: 16px;
  border: 2px solid var(--border); border-top-color: var(--accent);
  border-radius: 50%; animation: spin 0.6s linear infinite;
}

/* ========== Stats Row ========== */
.stats-row { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--space-3); margin-bottom: var(--space-6); }
.stat-tile {
  position: relative; overflow: hidden;
  display: flex; align-items: center; gap: var(--space-3);
  padding: var(--space-4) var(--space-5);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  transition: border-color var(--transition), transform var(--transition);
}
.stat-tile:hover { border-color: var(--stat-color); transform: translateY(-1px); }
.stat-tile-icon {
  font-size: 20px; color: var(--stat-color);
  width: 40px; height: 40px; display: flex; align-items: center; justify-content: center;
  background: var(--stat-bg); border-radius: var(--radius);
  flex-shrink: 0;
}
.stat-tile-body { display: flex; flex-direction: column; gap: 2px; }
.stat-tile-value { font-size: var(--text-2xl); font-weight: 700; color: var(--text-primary); line-height: 1; }
.stat-tile-label { font-size: var(--text-xs); color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; }
.stat-tile-glow {
  position: absolute; top: -20px; right: -20px;
  width: 80px; height: 80px;
  background: radial-gradient(circle, var(--stat-bg) 0%, transparent 70%);
  opacity: 0.6; pointer-events: none;
}

/* ========== Sections ========== */
.dash-section { margin-bottom: var(--space-6); }
.dash-section-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--space-3); }
.dash-section-title {
  display: flex; align-items: center; gap: var(--space-2);
  font-size: var(--text-md); font-weight: 600; color: var(--text-primary); margin: 0;
}
.section-icon { font-size: var(--text-sm); opacity: 0.7; }
.dash-section-count {
  font-size: var(--text-xs); padding: 2px 8px;
  background: var(--bg-elevated); border-radius: var(--radius-pill);
}

/* ========== Quick Connect ========== */
.recent-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: var(--space-2); }
.recent-tile {
  display: flex; flex-direction: column; gap: var(--space-2);
  padding: var(--space-3);
  background: var(--bg-surface); border: 1px solid var(--border);
  border-radius: var(--radius); cursor: pointer; text-align: left;
  transition: border-color var(--transition), background var(--transition), transform var(--transition);
}
.recent-tile:hover { border-color: var(--border-strong); background: var(--bg-hover); transform: translateY(-1px); }
.recent-tile-top { display: flex; align-items: center; }
.recent-tile-name { font-size: var(--text-sm); font-weight: 600; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.recent-tile-host { font-size: var(--text-xs); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

/* ========== Environments ========== */
.env-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: var(--space-3); }
.env-tile {
  display: flex; flex-direction: column; justify-content: space-between;
  padding: var(--space-4);
  background: var(--bg-surface); border: 1px solid var(--border);
  border-radius: var(--radius-lg); cursor: pointer; text-align: left;
  transition: border-color var(--transition), transform var(--transition);
  min-height: 100px;
}
.env-tile:hover { border-color: var(--accent); transform: translateY(-1px); }
.env-tile-header { display: flex; flex-direction: column; gap: var(--space-2); }
.env-tile-name-row { display: flex; align-items: center; gap: var(--space-2); }
.env-tile-name { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); }
.env-tile-desc { font-size: var(--text-xs); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.env-tile-footer {
  display: flex; align-items: center; gap: var(--space-4);
  padding-top: var(--space-3); margin-top: var(--space-3);
  border-top: 1px solid var(--border-subtle);
}
.env-tile-stat { display: flex; flex-direction: column; gap: 1px; }
.env-tile-stat-val { font-size: var(--text-sm); font-weight: 600; color: var(--text-secondary); }
.env-tile-stat-lbl { font-size: 10px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; }
.env-tile-arrow { margin-left: auto; font-size: var(--text-sm); transition: transform var(--transition); }
.env-tile:hover .env-tile-arrow { transform: translateX(3px); }

/* ========== Empty State ========== */
.dash-empty {
  display: flex; flex-direction: column; align-items: center; gap: var(--space-3);
  padding: var(--space-8); background: var(--bg-surface); border: 1px dashed var(--border);
  border-radius: var(--radius-lg); text-align: center;
}
.dash-empty-icon { font-size: 32px; opacity: 0.4; }
.dash-empty-text { font-size: var(--text-sm); color: var(--text-muted); }
.dash-empty-action {
  font-size: var(--text-sm); color: var(--accent); background: none; border: none;
  cursor: pointer; font-family: var(--font-mono);
}
.dash-empty-action:hover { text-decoration: underline; }

/* ========== Utils ========== */
.text-success { color: var(--success); }
.text-muted { color: var(--text-muted); }

@keyframes spin { to { transform: rotate(360deg); } }

/* ========== Responsive ========== */
@media (max-width: 768px) {
  .dashboard { padding: var(--space-4); }
  .stats-row { grid-template-columns: 1fr; }
  .stat-tile { padding: var(--space-3) var(--space-4); }
  .stat-tile-value { font-size: var(--text-xl); }
  .env-grid { grid-template-columns: 1fr; }
  .recent-grid { grid-template-columns: repeat(2, 1fr); }
}
@media (max-width: 375px) {
  .recent-grid { grid-template-columns: 1fr; }
}
</style>
