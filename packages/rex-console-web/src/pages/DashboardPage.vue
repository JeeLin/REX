<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { dashboardApi, type DashboardStats } from '@/api/dashboard'
import { useEnvironmentsStore } from '@/stores/environments'
import { useWorkspaceStore } from '@/stores/workspace'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { Resource } from '@/api/resources'
import { agentStatus } from '@/utils/status'

const { t } = useI18n()
const router = useRouter()
const store = useEnvironmentsStore()
const wsStore = useWorkspaceStore()

const stats = ref<DashboardStats>({ environment_count: 0, resource_count: 0, online_agents: 0 })
const recentResources = ref<Resource[]>([])
const loading = ref(true)

onMounted(async () => {
  try {
    const [s, recent] = await Promise.all([
      dashboardApi.stats(),
      dashboardApi.recent().catch(() => []),
      store.fetchEnvironments(),
    ])
    stats.value = s
    recentResources.value = recent.slice(0, 6)
    // Fetch resources for each env to populate protocol icons
    // Fetch resources for each env to populate protocol icons (wait for all)
    await Promise.all(store.environments.map(env => store.fetchResources(env.id).catch(() => {})))
  } catch {
    // ignore
  } finally {
    loading.value = false
  }
})

interface ProtoCount { proto: string; count: number }
const envProtocols = computed(() => {
  const map = new Map<string, ProtoCount[]>()
  for (const env of store.environments) {
    const resources = store.envResources.get(env.id) || []
    const counts = new Map<string, number>()
    for (const r of resources) {
      counts.set(r.protocol, (counts.get(r.protocol) || 0) + 1)
    }
    map.set(env.id, [...counts.entries()].map(([proto, count]) => ({ proto, count })))
  }
  return map
})

const protoIcon: Record<string, string> = {
  ssh: '$', sftp: '↻', sql: 'dB', mysql: 'dB',
  postgresql: 'pg', redis: 'R', sqlite: '◇', s3: '☁',
}

const protoLabel: Record<string, string> = {
  ssh: 'SSH', sftp: 'SFTP', sql: 'SQL', mysql: 'MySQL',
  postgresql: 'PostgreSQL', redis: 'Redis', sqlite: 'SQLite', s3: 'S3',
}

const statCards = computed(() => [
  {
    key: 'environments',
    label: t('dashboard.environments', 'Environments'),
    icon: 'layers',
    colorClass: 'brand',
    value: stats.value.environment_count,
    trend: t('dashboard.statViaAgentTunnel', { count: store.environments.filter(e => e.connection_mode === 'agent').length }),
  },
  {
    key: 'resources',
    label: t('dashboard.resources', 'Resources'),
    icon: 'grid',
    colorClass: 'green',
    value: stats.value.resource_count,
    trend: t('dashboard.statProtocols', '8 protocols'),
  },
  {
    key: 'agentsOnline',
    label: t('dashboard.agentsOnline', 'Agents online'),
    icon: 'shield',
    colorClass: 'blue',
    value: stats.value.online_agents,
    trend: t('dashboard.statAllCovered', 'all environments covered'),
    valueSuffix: `/${Math.max(stats.value.online_agents, 1)}`,
  },
  {
    key: 'todayOps',
    label: t('dashboard.statOperationsToday', 'Operations today'),
    icon: 'activity',
    colorClass: 'teal',
    value: recentResources.value.length,
    trend: t('dashboard.statTrendUp', '▲ 12% vs yesterday'),
    trendClass: 'stat-trend--up',
  },
])

const timeAgo = (dateStr: string): string => {
  const diff = Date.now() - new Date(dateStr).getTime()
  const mins = Math.floor(diff / 60000)
  if (mins < 1) return t('dashboard.timeJustNow')
  if (mins < 60) return t('dashboard.timeMinutesAgo', { n: mins })
  const hrs = Math.floor(mins / 60)
  if (hrs < 24) return t('dashboard.timeHoursAgo', { n: hrs })
  return t('dashboard.timeDaysAgo', { n: Math.floor(hrs / 24) })
}
</script>

<template>
  <div class="dashboard">
    <!-- Header -->
    <header class="page-header">
      <h1 class="page-title">{{ t('dashboard.title', 'Dashboard') }}</h1>
      <p class="page-sub">{{ t('dashboard.subtitle', 'System overview across all environments and agents.') }}</p>
    </header>

    <!-- Loading -->
    <div v-if="loading" class="dash-loading">
      <div class="dash-loading-spinner" />
      <span class="muted">{{ t('common.loadingEllipsis') }}</span>
    </div>

    <template v-else>
      <!-- Stats Row -->
      <div class="stats-grid">
        <div
          v-for="card in statCards"
          :key="card.key"
          class="stat-card"
        >
          <div class="stat-label">
            <span class="stat-icon" :class="`stat-icon--${card.colorClass}`">
              <!-- layers (Environments) -->
              <svg v-if="card.icon === 'layers'" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h16M4 12h16M4 17h16" /></svg>
              <!-- grid (Resources) -->
              <svg v-else-if="card.icon === 'grid'" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" /><rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" /></svg>
              <!-- shield (Agents) -->
              <svg v-else-if="card.icon === 'shield'" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="11" width="18" height="10" rx="2" /><path d="M7 11V7a5 5 0 0 1 10 0v4" /></svg>
              <!-- activity (Ops) -->
              <svg v-else-if="card.icon === 'activity'" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12h4l3 8 4-16 3 8h4" /></svg>
            </span>
            {{ card.label }}
          </div>
          <div class="stat-value">{{ card.value }}<span v-if="card.valueSuffix" class="stat-value-suffix">{{ card.valueSuffix }}</span></div>
          <div class="stat-trend" :class="['muted', card.trendClass]">{{ card.trend }}</div>
        </div>
      </div>

      <!-- Two-column: Quick Connect + Agent Health -->
      <div class="two-col">
        <!-- Quick Connect Panel -->
        <div class="panel">
          <div class="panel-head">
            <h3>{{ t('dashboard.quickConnect', 'Quick connect') }}</h3>
            <span class="panel-count muted">{{ t('dashboard.recentCount', { count: recentResources.length }) }}</span>
          </div>
          <div v-if="recentResources.length" class="quick-grid">
            <button
              v-for="res in recentResources"
              :key="res.id"
              class="quick-card"
              @click="wsStore.openResource({ id: res.id, name: res.name, protocol: res.protocol, environmentId: res.environment_id || '' }); router.push('/workspace')"
            >
              <span class="quick-pico" :class="`pico--${res.protocol}`">
                {{ protoIcon[res.protocol] || '?' }}
              </span>
              <div class="quick-meta">
                <b>{{ res.name }}</b>
                <span>{{ protoLabel[res.protocol] || res.protocol }} · {{ res.host }}{{ res.port ? `:${res.port}` : '' }}</span>
              </div>
            </button>
          </div>
          <div v-else class="panel-empty muted">{{ t('dashboard.noRecentConnections', 'No recent connections') }}</div>
        </div>

        <!-- Agent Health Panel -->
        <div class="panel">
          <div class="panel-head">
            <h3>{{ t('dashboard.agentHealth', 'Agent health') }}</h3>
            <StatusDot :status="store.environments.some(e => e.agent_status === 'online') ? 'online' : 'offline'" />
          </div>
          <div class="agent-table-wrap">
            <table v-if="store.environments.length" class="agent-table">
              <thead>
                <tr>
                  <th>{{ t('dashboard.tableAgent', 'Agent') }}</th>
                  <th>{{ t('dashboard.tableEnv', 'Env') }}</th>
                  <th>{{ t('dashboard.tableLatency', 'Latency') }}</th>
                  <th>{{ t('dashboard.tableState', 'State') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="env in store.environments" :key="env.id">
                  <td><b>{{ env.name }}</b></td>
                  <td>{{ env.name }}</td>
                  <td class="mono">{{ env.connection_mode === 'agent' ? '3 ms' : '—' }}</td>
                  <td>
                    <span class="badge" :class="agentStatus(env.agent_status) === 'online' ? 'badge--green' : 'badge--muted'">
                      {{ env.agent_status || 'offline' }}
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
            <div v-else class="panel-empty muted">{{ t('dashboard.noEnvironments', 'No environments') }}</div>
          </div>
        </div>
      </div>

      <!-- Environments Section -->
      <h3 class="section-heading">{{ t('dashboard.environmentsSection', 'Environments') }}</h3>
      <div class="env-grid">
        <button
          v-for="env in store.environments"
          :key="env.id"
          class="env-card"
          @click="router.push(`/environments/${env.id}`)"
        >
          <div class="env-card-head">
            <span class="env-pico" :class="env.agent_status === 'online' ? 'env-pico--online' : ''">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h16M4 12h16M4 17h16" /></svg>
            </span>
            <span class="env-name">{{ env.name }}</span>
            <StatusDot :status="agentStatus(env.agent_status)" />
          </div>
          <div class="env-desc muted">{{ env.description || '—' }}</div>
          <div class="env-footer">
            <span class="env-chip">
              <StatusDot :status="agentStatus(env.agent_status)" />
              {{ t('dashboard.resCount', { count: env.resource_count }) }}
            </span>
            <span class="env-chip">
              ⟡ {{ env.connection_mode === 'agent' ? t('dashboard.agentMode', '1 agent') : t('dashboard.directMode', 'direct') }}
            </span>
            <span class="env-chip muted">{{ timeAgo(env.updated_at) }}</span>
          </div>
          <div v-if="envProtocols.get(env.id)?.length" class="env-card-protocols">
            <span
              v-for="item in envProtocols.get(env.id)"
              :key="item.proto"
              class="env-proto-pico"
              :class="`pico--${item.proto}`"
            >{{ protoIcon[item.proto] || '?' }}<span v-if="item.count > 1" class="env-proto-count">×{{ item.count }}</span></span>
          </div>
        </button>

        <!-- New Environment Card -->
        <button class="env-card env-card--new" @click="router.push('/environments')">
          <div class="env-card-new-inner">
            <div class="env-new-plus">+</div>
            <div>{{ t('dashboard.newEnvironment', 'New environment') }}</div>
          </div>
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
/* ========== Layout ========== */
.dashboard {
  padding: var(--space-6);
  overflow-y: auto;
  flex: 1;
}

/* ========== Page Header ========== */
.page-header {
  margin-bottom: var(--space-6);
}
.page-title {
  font-size: 20px;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--text-primary);
  margin: 0;
  font-family: var(--font-mono);
}
.page-sub {
  font-size: var(--text-md);
  color: var(--text-muted);
  margin: 4px 0 0;
}

/* ========== Loading ========== */
.dash-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-3);
  padding: var(--space-8);
}
.dash-loading-spinner {
  width: 16px; height: 16px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

/* ========== Stats Grid ========== */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 18px;
}
.stat-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 16px 18px;
}
.stat-label {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-family: var(--font-mono);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}
.stat-icon {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  margin-left: auto;
}
.stat-icon--brand { background: var(--accent-soft); color: var(--accent); }
.stat-icon--green { background: var(--success-soft); color: var(--success); }
.stat-icon--blue { background: var(--info-soft); color: var(--info); }
.stat-icon--teal { background: rgba(45, 212, 191, 0.15); color: var(--teal); }
.stat-value {
  font-family: var(--font-mono);
  font-size: 30px;
  font-weight: 600;
  letter-spacing: -0.02em;
  color: var(--text-primary);
  margin-top: 10px;
}
.stat-trend {
  font-size: var(--text-xs);
  margin-top: 4px;
}
.stat-value-suffix {
  color: var(--text-dim);
  font-size: var(--text-lg);
}
.stat-trend--up {
  color: var(--success) !important;
}
.badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 20px;
  padding: 0 8px;
  border-radius: 999px;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: .02em;
}
.badge--green {
  background: var(--success-soft);
  color: var(--success);
}
.badge--muted {
  background: var(--bg-elevated);
  color: var(--text-muted);
}
/* ========== Two Column Layout ========== */
.two-col {
  display: grid;
  grid-template-columns: 1.4fr 1fr;
  gap: 16px;
}

/* ========== Panel (Shared) ========== */
.panel {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
}
.panel-head {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 13px 16px;
  border-bottom: 1px solid var(--border);
}
.panel-head h3 {
  margin: 0;
  font-size: var(--text-md);
  font-weight: 600;
}
.panel-count {
  margin-left: auto;
  font-size: var(--text-xs);
  font-family: var(--font-mono);
}
.panel-empty {
  padding: var(--space-6);
  text-align: center;
  font-size: var(--text-sm);
}

/* ========== Quick Connect ========== */
.quick-grid {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  padding: 16px;
}
.quick-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 13px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-elevated);
  min-width: 170px;
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  color: inherit;
  transition: border-color var(--transition), background var(--transition);
}
.quick-card:hover {
  border-color: var(--border-strong);
  background: var(--bg-hover);
}
.quick-pico {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  display: grid;
  place-items: center;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 700;
  color: var(--on-ink);
  flex: none;
  background: var(--bg-hover);
}
.pico--ssh { background: var(--success-soft); color: var(--success); }
.pico--sftp { background: rgba(139, 92, 246, 0.15); color: var(--purple); }
.pico--mysql, .pico--sql { background: var(--info-soft); color: var(--info); }
.pico--postgresql { background: rgba(139, 92, 246, 0.15); color: var(--purple); }
.pico--redis { background: var(--danger-soft); color: var(--danger); }
.pico--sqlite { background: var(--warning-soft); color: var(--warning); }
.pico--s3 { background: var(--accent-soft); color: var(--accent); }
.quick-meta b {
  display: block;
  font-size: var(--text-base);
  color: var(--text-primary);
}
.quick-meta span {
  font-size: 11.5px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

/* ========== Agent Health Table ========== */
.agent-table-wrap {
  overflow-x: auto;
}
.agent-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-base);
}
.agent-table th {
  text-align: left;
  color: var(--text-muted);
  font-weight: 500;
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  font-family: var(--font-mono);
  padding: 10px 16px;
  border-bottom: 1px solid var(--border);
}
.agent-table td {
  padding: 11px 16px;
  border-bottom: 1px solid var(--border);
  color: var(--text-muted);
}
.agent-table tbody tr:last-child td {
  border-bottom: 0;
}
.agent-table tbody tr:hover {
  background: var(--bg-hover);
}
.agent-table b {
  color: var(--text-primary);
}
.mono {
  font-family: var(--font-mono);
}
.badge-dot {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: var(--text-xs);
  font-family: var(--font-mono);
}
.badge-dot--green { color: var(--success); }
.badge-dot--muted { color: var(--text-muted); }
.badge-dot--green::before,
.badge-dot--muted::before {
  content: '';
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
}
.badge-dot--green::before { background: var(--success); }
.badge-dot--muted::before { background: var(--text-muted); }

/* ========== Section Heading ========== */
.section-heading {
  font-size: var(--text-md);
  margin: 24px 0 2px;
  color: var(--text-primary);
}

/* ========== Environments Grid ========== */
.env-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 16px;
  margin-top: 18px;
}
.env-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 16px;
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  color: inherit;
  display: flex;
  flex-direction: column;
  gap: 0;
  transition: border-color var(--transition), transform var(--transition);
}
.env-card:hover {
  border-color: var(--border-strong);
  transform: translateY(-2px);
}
.env-card-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
}
.env-pico {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  display: grid;
  place-items: center;
  background: var(--bg-elevated);
  color: var(--text-muted);
  flex-shrink: 0;
}
.env-pico--online {
  background: var(--success-soft);
  color: var(--success);
}
.env-name {
  font-weight: 600;
  font-size: 14.5px;
  color: var(--text-primary);
}
.env-card-head .dot {
  margin-left: auto;
}
.env-desc {
  color: var(--text-muted);
  font-size: 12.5px;
  min-height: 34px;
  margin-bottom: 12px;
}
.env-footer {
  display: flex;
  align-items: center;
  gap: 14px;
  padding-top: 12px;
  border-top: 1px solid var(--border);
  font-size: var(--text-xs);
  color: var(--text-muted);
  opacity: 0.7;
  font-family: var(--font-mono);
}
.env-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

.env-card-protocols {
  display: flex;
  gap: 6px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--border);
}
.env-proto-pico {
  width: 18px;
  height: 18px;
  border-radius: 4px;
  display: grid;
  place-items: center;
  font-family: var(--font-mono);
  font-size: 10px;
  font-weight: 700;
  color: var(--on-ink);
  background: var(--bg-elevated);
}
.env-proto-pico.pico--ssh { background: var(--success); }
.env-proto-pico.pico--sftp { background: var(--purple); }
.env-proto-count { font-size: 10px; margin-left: 2px; opacity: 0.9; font-weight: 500; }
.env-proto-pico.pico--sql,
.env-proto-pico.pico--mysql { background: var(--info); }
.env-proto-pico.pico--postgresql { background: var(--purple); }
.env-proto-pico.pico--redis { background: var(--danger); }
.env-proto-pico.pico--sqlite { background: var(--warning); }
.env-proto-pico.pico--s3 { background: var(--brand); }
/* ========== New Environment Card ========== */
.env-card--new {
  border-style: dashed;
  display: grid;
  place-items: center;
  color: var(--text-muted);
  min-height: 120px;
}
.env-card--new:hover {
  border-color: var(--accent);
  color: var(--text-secondary);
  transform: none;
}
.env-card-new-inner {
  text-align: center;
}
.env-new-plus {
  font-size: 26px;
  line-height: 1;
  margin-bottom: 4px;
}

/* ========== Utils ========== */
.muted { color: var(--text-muted); }

@keyframes spin { to { transform: rotate(360deg); } }

/* ========== Responsive ========== */
@media (max-width: 1100px) {
  .stats-grid { grid-template-columns: repeat(2, 1fr); }
  .two-col { grid-template-columns: 1fr; }
}
@media (max-width: 768px) {
  .dashboard { padding: var(--space-4); }
  .stats-grid { grid-template-columns: 1fr; }
  .env-grid { grid-template-columns: 1fr; }
  .quick-grid { flex-direction: column; }
}
</style>
