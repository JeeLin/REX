<template>
  <div>
    <div class="env-detail-header">
      <router-link to="/environments" class="btn btn-ghost btn-sm">
        ← {{ t('common.back') }}
      </router-link>
      <h2 class="page-title">{{ env?.name || '...' }}</h2>
      <span v-if="env" class="badge" :class="env.connection_mode === 'direct' ? 'badge-info' : 'badge-success'">
        {{ env.connection_mode === 'direct' ? t('env.direct') : t('env.agentProxy') }}
      </span>
    </div>

    <div v-if="env?.description" class="env-desc">{{ env.description }}</div>

    <LoadingSpinner v-if="loading" :text="t('common.loading')" />

    <ErrorState v-else-if="loadError" :message="loadError" :retry="loadEnv" />

    <template v-else>
      <!-- Resources -->
      <div class="section-header">
        <h3 class="section-title">{{ t('env.resources') }}</h3>
        <router-link
          v-if="env"
          :to="`/environments/${env.id}/resources/new`"
          class="btn btn-primary btn-sm"
        >
          {{ t('env.addResource') }}
        </router-link>
      </div>

      <EmptyState
        v-if="resources.length === 0"
        icon="📦"
        :title="t('env.noResources')"
      />

      <div v-else class="resource-list">
        <button
          v-for="res in resources"
          :key="res.id"
          class="resource-item resource-clickable"
          @click="connectToResource(res)"
          @contextmenu.prevent="onResourceCtx($event, res)"
        >
          <div class="resource-icon" :style="{ background: getProtocolIcon(res.protocol).color + '15', color: getProtocolIcon(res.protocol).color }">
            {{ getProtocolIcon(res.protocol).icon }}
          </div>
          <div class="resource-info">
            <div class="resource-name">{{ res.name }}</div>
            <div class="resource-proto">{{ res.protocol.toUpperCase() }}</div>
          </div>
          <span class="resource-status badge badge-success">{{ t('status.online') }}</span>
        </button>
      </div>

      <!-- Agent Status -->
      <AgentStatusPanel v-if="env" :env-id="env.id" />
    </template>

    <!-- Edit Modal -->
    <EnvironmentEditModal
      v-model:visible="editModalVisible"
      :env-id="editingEnvId"
    />

    <!-- Resource Edit Modal -->
    <ResourceEditModal
      v-model:visible="resEditModalVisible"
      :env-id="resEditEnvId"
      :resource-id="resEditResId"
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

    <!-- Resource Delete Confirm -->
    <ConfirmDialog
      :visible="showResDeleteConfirm"
      :title="t('resource.deleteTitle')"
      :message="t('resource.deleteConfirm')"
      :danger="true"
      @confirm="confirmDeleteRes"
      @cancel="showResDeleteConfirm = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import ErrorState from '@/components/ErrorState.vue'
import EmptyState from '@/components/EmptyState.vue'
import EnvironmentEditModal from '@/components/EnvironmentEditModal.vue'
import ResourceEditModal from '@/components/ResourceEditModal.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import client from '@/api/client'
import type { Environment, Resource } from '@/api/env'
import { deleteEnvironment, deleteResource } from '@/api/env'
import { getProtocolIcon } from '@/composables/useProtocol'
import { useProtocol } from '@/composables/useProtocol'
import AgentStatusPanel from '@/features/agents/AgentStatusPanel.vue'

const { t } = useI18n()
const route = useRoute()
const { connectToResource: connect } = useProtocol()
const { show: showMenu } = useContextMenu()

const env = ref<Environment | null>(null)
const resources = ref<Resource[]>([])
const loading = ref(true)
const loadError = ref('')
const editModalVisible = ref(false)
const editingEnvId = ref('')
const showDeleteConfirm = ref(false)
const showResDeleteConfirm = ref(false)
const deletingResId = ref('')
const resEditModalVisible = ref(false)
const resEditEnvId = ref('')
const resEditResId = ref('')

function connectToResource(res: Resource) {
  connect(res, env.value?.name || '')
}

function onResourceCtx(e: MouseEvent, res: Resource) {
  showMenu(e, [
    { label: t('ctx.connect'), action: () => connectToResource(res) },
    { label: t('ctx.connectNewTab'), action: () => connectToResource(res) },
    { separator: true },
    { label: t('ctx.editResource'), action: () => openResEditModal(res) },
    { label: t('ctx.deleteResource'), danger: true, action: () => requestDeleteRes(res) },
    { separator: true },
    { label: t('ctx.copyAddress'), action: () => navigator.clipboard?.writeText(res.name) },
  ])
}

function openResEditModal(res: Resource) {
  resEditEnvId.value = env.value?.id || ''
  resEditResId.value = res.id
  resEditModalVisible.value = true
}

function requestDeleteRes(res: Resource) {
  deletingResId.value = res.id
  showResDeleteConfirm.value = true
}

async function confirmDeleteRes() {
  if (!env.value || !deletingResId.value) return
  try {
    await deleteResource(env.value.id, deletingResId.value)
    resources.value = resources.value.filter(r => r.id !== deletingResId.value)
  } catch {
    // silent
  } finally {
    showResDeleteConfirm.value = false
    deletingResId.value = ''
  }
}

function openEditEnvModal() {
  if (env.value) {
    editingEnvId.value = env.value.id
    editModalVisible.value = true
  }
}

async function confirmDeleteEnv() {
  if (!env.value) return
  try {
    await deleteEnvironment(env.value.id)
    window.history.back()
  } catch {
    // silent
  } finally {
    showDeleteConfirm.value = false
  }
}

onMounted(() => loadEnv())

async function loadEnv() {
  loading.value = true
  loadError.value = ''
  const id = route.params.id as string
  try {
    const { data } = await client.get<{ data: Environment }>(`/environments/${id}`)
    env.value = data.data
    const resResp = await client.get<{ data: Resource[] }>(`/environments/${id}/resources`)
    resources.value = resResp.data.data
  } catch {
    loadError.value = t('env.detailLoadFailed')
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.env-detail-header {
  display: flex;
  align-items: center;
  gap: var(--sp-lg);
  margin-bottom: var(--sp-md);
}

.env-desc {
  color: var(--text-secondary);
  font-size: var(--fs-sm);
  margin-bottom: var(--sp-xl);
}

.empty-state {
  text-align: center;
  padding: var(--sp-3xl);
  color: var(--text-secondary);
}

.resource-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
}

.resource-item {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
  padding: var(--sp-md) var(--sp-lg);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
  color: inherit;
  text-align: left;
  width: 100%;
}

.resource-clickable {
  cursor: pointer;
}

.resource-clickable:hover {
  border-color: rgba(232, 145, 45, 0.2);
  background: var(--bg-elevated);
}

.resource-icon {
  width: 36px;
  height: 36px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: var(--fs-sm);
  flex-shrink: 0;
}

.resource-info {
  flex: 1;
  min-width: 0;
}

.resource-name {
  font-family: var(--font-mono);
  font-weight: 500;
  font-size: var(--fs-base);
}

.resource-proto {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.badge-info {
  color: var(--info);
}
</style>
