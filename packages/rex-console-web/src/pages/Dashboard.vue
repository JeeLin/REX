<template>
  <div>
    <!-- Stats -->
    <div class="stats-row">
      <div class="stat-card">
        <div class="stat-label">{{ t('dashboard.envCount') }}</div>
        <div class="stat-value" style="color: var(--accent)">{{ envCount }}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">{{ t('dashboard.resCount') }}</div>
        <div class="stat-value" style="color: var(--info)">{{ resourceCount }}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">{{ t('dashboard.agentOnline') }}</div>
        <div class="stat-value" style="color: var(--success)">{{ agentOnlineCount }}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">{{ t('dashboard.todayOps') }}</div>
        <div class="stat-value">{{ todayOps }}</div>
      </div>
    </div>

    <!-- Environments -->
    <div class="section-header">
      <h2 class="section-title">{{ t('dashboard.environments') }}</h2>
      <span class="text-sm text-secondary">{{ envCount }} {{ t('dashboard.envCountLabel') }}</span>
    </div>
    <div class="env-grid">
      <router-link
        v-for="env in environments"
        :key="env.id"
        :to="`/environments/${env.id}`"
        class="env-card"
      >
        <div class="env-card-header">
          <span class="env-card-name">{{ env.name }}</span>
          <span class="badge" :class="env.connection_mode === 'direct' ? 'badge-info' : 'badge-success'">
            {{ env.connection_mode === 'direct' ? t('env.connectionModeLabel') : t('env.agentOnline') }}
          </span>
        </div>
        <div class="env-card-desc">{{ env.description || '—' }}</div>
        <div class="env-card-footer">
          <span>{{ env.connection_mode === 'direct' ? t('env.direct') : t('env.agentProxy') }}</span>
        </div>
      </router-link>

      <router-link to="/environments/new" class="add-env-card">
        <div class="add-icon">+</div>
        <div>{{ t('dashboard.createEnv') }}</div>
      </router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import client from '@/api/client'

const { t } = useI18n()

interface Environment {
  id: string
  name: string
  description: string | null
  connection_mode: string
  created_at: string
  updated_at: string
}

const environments = ref<Environment[]>([])
const envCount = ref(0)
const resourceCount = ref(0)
const agentOnlineCount = ref(0)
const todayOps = ref(0)

onMounted(async () => {
  try {
    const { data } = await client.get<{ data: Environment[] }>('/environments')
    environments.value = data.data
    envCount.value = data.data.length
  } catch {
    // 静默处理
  }
})
</script>

<style scoped>
.env-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  cursor: pointer;
  transition: all var(--transition-base);
  text-decoration: none;
  color: inherit;
  display: block;
}

.env-card:hover {
  border-color: rgba(232, 145, 45, 0.2);
  transform: translateY(-2px);
  box-shadow: var(--shadow-glow);
  text-decoration: none;
}

.env-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--sp-md);
}

.env-card-name {
  font-family: var(--font-mono);
  font-weight: 600;
  font-size: var(--fs-lg);
}

.env-card-desc {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  margin-bottom: var(--sp-lg);
}

.env-card-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: var(--sp-md);
  border-top: 1px solid var(--border);
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.add-env-card {
  background: var(--bg-surface);
  border: 2px dashed var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-md);
  cursor: pointer;
  transition: all var(--transition-base);
  min-height: 200px;
  color: var(--text-muted);
  text-decoration: none;
}

.add-env-card:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: rgba(232, 145, 45, 0.03);
  text-decoration: none;
  box-shadow: 0 0 20px rgba(232, 145, 45, 0.06);
}

.add-icon {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  border: 2px solid currentColor;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  transition: box-shadow var(--transition-base);
}

.add-env-card:hover .add-icon {
  box-shadow: 0 0 16px var(--accent-glow);
}

.badge-info {
  color: var(--info);
}

.text-sm {
  font-size: var(--fs-sm);
}

.text-secondary {
  color: var(--text-secondary);
}
</style>
