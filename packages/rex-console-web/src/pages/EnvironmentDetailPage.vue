<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useEnvironmentsStore } from '@/stores/environments'
import { environmentsApi, type Environment } from '@/api/environments'
import { resourcesApi, type Resource } from '@/api/resources'
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Modal from '@/components/ui/Modal.vue'
import WizardModal from '@/features/resource/WizardModal.vue'
import { PROTOCOL_ICONS, PROTOCOL_COLORS } from '@/features/resource/protocols'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const store = useEnvironmentsStore()

const envId = route.params.id as string
const env = ref<Environment | null>(null)
const resources = ref<Resource[]>([])
const loading = ref(true)
const showWizard = ref(false)
const deleteConfirmId = ref<string | null>(null)
const editModal = ref(false)
const editName = ref('')
const editDesc = ref('')
const editMode = ref('direct')
const editError = ref('')
const editLoading = ref(false)

onMounted(async () => {
  try {
    env.value = await environmentsApi.get(envId)
    resources.value = await resourcesApi.listByEnv(envId)
  } catch {
    router.push('/environments')
  } finally {
    loading.value = false
  }
})

function openEdit() {
  if (!env.value) return
  editName.value = env.value.name
  editDesc.value = env.value.description
  editMode.value = env.value.connection_mode
  editError.value = ''
  editModal.value = true
}

async function submitEdit() {
  if (!editName.value.trim()) {
    editError.value = t('common.nameRequired')
    return
  }
  editLoading.value = true
  try {
    const updated = await store.updateEnvironment(envId, {
      name: editName.value.trim(),
      description: editDesc.value.trim(),
      connection_mode: editMode.value,
    })
    env.value = { ...env.value!, ...updated }
    editModal.value = false
  } catch (e: unknown) {
    editError.value = e instanceof Error ? e.message : String(e)
  } finally {
    editLoading.value = false
  }
}

async function deleteResource(id: string) {
  try {
    await resourcesApi.delete(envId, id)
    resources.value = resources.value.filter(r => r.id !== id)
    if (env.value) env.value.resource_count--
  } catch {
    // ignore
  }
}

async function refreshResources() {
  resources.value = await resourcesApi.listByEnv(envId)
  // 刷新环境的 resource_count
  if (env.value) {
    env.value.resource_count = resources.value.length
  }
}

function agentStatus(status: string | null): StatusDotStatus {
  if (status === 'online') return 'online'
  return 'offline'
}
</script>

<template>
  <div class="env-detail">
    <!-- Breadcrumb -->
    <div class="breadcrumb">
      <router-link to="/environments" class="breadcrumb-link">{{ t('nav.environments') }}</router-link>
      <span class="breadcrumb-sep">›</span>
      <span class="breadcrumb-current">{{ env?.name || '...' }}</span>
    </div>

    <div v-if="loading" class="loading">{{ t('common.loadingEllipsis') }}</div>

    <template v-else-if="env">
      <!-- Header -->
      <div class="env-header">
        <div class="env-header-info">
          <h1 class="page-title">{{ env.name }}</h1>
          <p class="env-description muted">{{ env.description || t('common.noDescription') }}</p>
        </div>
        <div class="env-header-actions">
          <Button variant="secondary" size="sm" @click="openEdit">{{ t('common.edit') }}</Button>
        </div>
      </div>

      <!-- Meta info -->
      <div class="env-meta">
        <Badge :tone="env.connection_mode === 'agent' ? 'warning' : 'info'">
          {{ env.connection_mode }}
        </Badge>
        <Badge tone="accent">{{ env.resource_count }} {{ t('common.resources') }}</Badge>
        <span class="muted" style="font-size: var(--text-xs)">
          {{ t('environmentDetail.created', { date: new Date(env.created_at).toLocaleDateString() }) }}
        </span>
      </div>

      <!-- Agent Panel (placeholder for M12) -->
      <Card class="section-card">
        <h2 class="section-title">{{ t('environmentDetail.agentSection') }}</h2>
        <div v-if="env.agent_status" class="agent-info">
          <StatusDot :status="agentStatus(env.agent_status)" />
          <span>{{ t('environments.agentStatus', { status: env.agent_status }) }}</span>
        </div>
        <div v-else class="agent-empty muted">
          {{ t('environmentDetail.noAgent') }}
        </div>
      </Card>

      <!-- Resources Table -->
      <Card class="section-card">
        <div class="section-header">
          <h2 class="section-title">{{ t('environmentDetail.resourcesSection') }}</h2>
          <Button variant="primary" size="sm" @click="showWizard = true">+ {{ t('environmentDetail.addResource') }}</Button>
        </div>

        <EmptyState
          v-if="resources.length === 0"
          icon="⊕"
          :title="t('environmentDetail.noResources')"
          :description="t('environmentDetail.addResourceDesc')"
        >
          <Button variant="primary" size="sm" @click="showWizard = true">{{ t('environmentDetail.addResource') }}</Button>
        </EmptyState>

        <table v-else class="resource-table">
          <thead>
            <tr>
              <th>{{ t('common.name') }}</th>
              <th>{{ t('wizard.protocol') }}</th>
              <th>{{ t('wizard.host') }}</th>
              <th>{{ t('wizard.port') }}</th>
              <th>{{ t('wizard.username') }}</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="res in resources" :key="res.id">
              <td>
                <span class="res-name">
                  <span class="res-icon" :style="{ color: PROTOCOL_COLORS[res.protocol] || 'var(--text-secondary)' }">
                    {{ PROTOCOL_ICONS[res.protocol] || '?' }}
                  </span>
                  {{ res.name }}
                </span>
              </td>
              <td>
                <Badge :tone="res.protocol === 'redis' ? 'danger' : res.protocol === 'ssh' ? 'success' : 'info'">
                  {{ res.protocol }}
                </Badge>
              </td>
              <td class="mono">{{ res.host }}</td>
              <td class="mono">{{ res.port || '—' }}</td>
              <td>{{ res.username || '—' }}</td>
              <td>
                <button class="icon-btn danger" :title="t('common.delete')" @click="deleteResource(res.id)">✕</button>
              </td>
            </tr>
          </tbody>
        </table>
      </Card>
    </template>

    <!-- Resource Creation Wizard -->
    <WizardModal
      :visible="showWizard"
      :environment-id="envId"
      @close="showWizard = false"
      @created="showWizard = false; refreshResources()"
    />

    <!-- Edit Modal -->
    <Modal v-model="editModal">
      <template #title>{{ t('environments.editEnvironment') }}</template>
      <form class="env-form" @submit.prevent="submitEdit">
        <label class="form-label">
          <span>{{ t('common.name') }}</span>
          <input v-model="editName" type="text" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('common.description') }}</span>
          <input v-model="editDesc" type="text" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('environments.connectionMode') }}</span>
          <select v-model="editMode" class="form-input">
            <option value="direct">{{ t('environments.direct') }}</option>
            <option value="agent">{{ t('environments.agent') }}</option>
          </select>
        </label>
        <div v-if="editError" class="form-error">{{ editError }}</div>
        <div class="form-actions">
          <Button type="button" variant="secondary" @click="editModal = false">{{ t('common.cancel') }}</Button>
          <Button type="submit" variant="primary" :loading="editLoading">{{ t('common.save') }}</Button>
        </div>
      </form>
    </Modal>
  </div>
</template>

<style scoped>
.env-detail {
  max-width: 900px;
}
.breadcrumb {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-4);
  font-size: var(--text-sm);
}
.breadcrumb-link {
  color: var(--text-secondary);
  text-decoration: none;
}
.breadcrumb-link:hover {
  color: var(--accent);
}
.breadcrumb-sep {
  color: var(--text-muted);
}
.breadcrumb-current {
  color: var(--text-primary);
  font-weight: 500;
}
.loading {
  color: var(--text-muted);
  padding: var(--space-8) 0;
  text-align: center;
}
.env-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: var(--space-4);
}
.page-title {
  font-size: var(--text-xl);
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}
.env-description {
  margin-top: var(--space-1);
  font-size: var(--text-sm);
}
.env-meta {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-6);
}
.section-card {
  margin-bottom: var(--space-4);
}
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-3);
}
.section-title {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}
.agent-info {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-sm);
  color: var(--text-secondary);
}
.agent-empty {
  font-size: var(--text-sm);
  padding: var(--space-4) 0;
}
.resource-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-sm);
}
.resource-table th {
  text-align: left;
  padding: var(--space-2) var(--space-3);
  color: var(--text-muted);
  font-weight: 500;
  border-bottom: 1px solid var(--border);
  font-size: var(--text-xs);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.resource-table td {
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
  color: var(--text-secondary);
}
.resource-table tr:hover td {
  background: var(--bg-hover);
}
.res-name {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-primary);
  font-weight: 500;
}
.res-icon {
  font-family: var(--font-mono);
  font-size: 14px;
  width: 20px;
  text-align: center;
}
.mono {
  font-family: var(--font-mono);
}
.muted {
  color: var(--text-muted);
}
.icon-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: 4px;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.icon-btn:hover {
  background: var(--bg-hover);
}
.icon-btn.danger:hover {
  color: var(--danger);
}
.env-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.form-label {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  font-size: var(--text-sm);
  color: var(--text-secondary);
}
.form-input {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}
.form-input:focus {
  border-color: var(--accent);
}
.form-error {
  color: var(--danger);
  font-size: var(--text-sm);
}
.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  margin-top: var(--space-4);
}
</style>
