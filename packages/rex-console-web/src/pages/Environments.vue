<template>
  <div>
    <div class="section-header">
      <h2 class="section-title">{{ t('env.title') }}</h2>
      <router-link to="/environments/new" class="btn btn-primary btn-sm">
        {{ t('env.create') }}
      </router-link>
    </div>

    <LoadingSpinner v-if="loading" :text="t('common.loading')" />

    <ErrorState v-else-if="loadError" :message="loadError" :retry="loadEnvs" />

    <EmptyState
      v-else-if="environments.length === 0"
      icon="📋"
      :title="t('common.noData')"
      :action="{ label: t('env.create'), handler: () => router.push('/environments/new') }"
    />

    <div v-else class="env-grid">
      <router-link
        v-for="env in environments"
        :key="env.id"
        :to="`/environments/${env.id}`"
        class="env-card"
        @contextmenu.prevent="onEnvCardCtx($event, env)"
      >
        <div class="env-card-header">
          <span class="env-card-name">{{ env.name }}</span>
          <span class="badge" :class="env.connection_mode === 'direct' ? 'badge-info' : 'badge-success'">
            {{ env.connection_mode === 'direct' ? t('env.connectionModeLabel') : t('env.agentOnline') }}
          </span>
        </div>
        <div class="env-card-desc">{{ env.description || '—' }}</div>
        <div v-if="env.resource_types && Object.keys(env.resource_types).length > 0" class="env-card-badges">
          <span
            v-for="(count, proto) in env.resource_types"
            :key="proto"
            class="res-badge"
          >
            {{ String(proto).toUpperCase() }} ×{{ count }}
          </span>
        </div>
        <div class="env-card-footer">
          <span>{{ env.connection_mode === 'direct' ? t('env.direct') : t('env.agentProxy') }}</span>
          <span v-if="env.resource_count !== undefined">{{ env.resource_count }} {{ t('env.resources') }}</span>
        </div>
      </router-link>

      <router-link to="/environments/new" class="add-env-card">
        <div class="add-icon">+</div>
        <div>{{ t('dashboard.createEnv') }}</div>
      </router-link>
    </div>

    <!-- Edit Modal -->
    <EnvironmentEditModal
      v-model:visible="editModalVisible"
      :env-id="editingEnvId"
    />

    <!-- Delete Confirm -->
    <ConfirmDialog
      :visible="showDeleteConfirm"
      :title="t('env.deleteTitle')"
      :message="t('env.deleteConfirm')"
      :danger="true"
      @confirm="confirmDeleteEnv"
      @cancel="showDeleteConfirm = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'
import { useProtocol } from '@/composables/useProtocol'
import { listResources } from '@/api/env'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import ErrorState from '@/components/ErrorState.vue'
import EmptyState from '@/components/EmptyState.vue'
import EnvironmentEditModal from '@/components/EnvironmentEditModal.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import client from '@/api/client'
import { deleteEnvironment } from '@/api/env'
import type { Environment } from '@/api/env'

const router = useRouter()
const { t } = useI18n()
const { show: showMenu } = useContextMenu()
const { connectToResource } = useProtocol()

const environments = ref<Environment[]>([])
const loading = ref(true)
const loadError = ref('')
const editModalVisible = ref(false)
const editingEnvId = ref('')
const showDeleteConfirm = ref(false)
const deletingEnvId = ref('')

function onEnvCardCtx(e: MouseEvent, env: Environment) {
  showMenu(e, [
    { label: t('ctx.openDetail'), action: () => router.push(`/environments/${env.id}`) },
    { label: t('ctx.newResource'), action: () => router.push(`/environments/${env.id}/resources/new`) },
    { label: t('ctx.addAgent'), action: () => router.push('/agents') },
    { separator: true },
    { label: t('ctx.openAllWorkspace'), action: () => openAllInWorkspace(env) },
    { separator: true },
    { label: t('ctx.editEnv'), action: () => openEditModal(env) },
    { label: t('ctx.deleteEnv'), danger: true, action: () => requestDeleteEnv(env) },
  ])
}

async function openAllInWorkspace(env: Environment) {
  try {
    const resources = await listResources(env.id)
    for (const res of resources) {
      connectToResource(res, env.name)
    }
  } catch {
    // silent
  }
}

function openEditModal(env: Environment) {
  editingEnvId.value = env.id
  editModalVisible.value = true
}

function requestDeleteEnv(env: Environment) {
  deletingEnvId.value = env.id
  showDeleteConfirm.value = true
}

async function confirmDeleteEnv() {
  try {
    await deleteEnvironment(deletingEnvId.value)
    environments.value = environments.value.filter(e => e.id !== deletingEnvId.value)
  } catch {
    // silent
  } finally {
    showDeleteConfirm.value = false
    deletingEnvId.value = ''
  }
}

onMounted(() => loadEnvs())

async function loadEnvs() {
  loading.value = true
  loadError.value = ''
  try {
    const { data } = await client.get<{ data: Environment[] }>('/environments')
    environments.value = data.data
  } catch {
    loadError.value = t('env.loadFailed')
  } finally {
    loading.value = false
  }
}
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

.env-card-badges {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-xs);
  margin-bottom: var(--sp-md);
}

.res-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  font-size: 11px;
  font-family: var(--font-mono);
  font-weight: 500;
  white-space: nowrap;
  background: var(--bg-elevated);
  color: var(--text-secondary);
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
}

.add-env-card:hover .add-icon {
  box-shadow: 0 0 16px var(--accent-glow);
}

.badge-info {
  color: var(--info);
}
</style>
