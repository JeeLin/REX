<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRoute, useRouter, onBeforeRouteUpdate } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useEnvironmentsStore } from '@/stores/environments'
import { useWorkspaceStore } from '@/stores/workspace'
import { environmentsApi, type Environment } from '@/api/environments'
import { resourcesApi, type Resource } from '@/api/resources'
import { api } from '@/api/client'
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
const wsStore = useWorkspaceStore()

const envId = ref(route.params.id as string)
const env = ref<Environment | null>(null)
const resources = ref<Resource[]>([])
const loading = ref(true)
const showWizard = ref(false)
const editModal = ref(false)
const editName = ref('')
const editDesc = ref('')
const editMode = ref('direct')
const editError = ref('')
const editLoading = ref(false)

// 编辑资源时的协议特定字段
const editPassword = ref('')
const editDatabaseName = ref('')
const editFilePath = ref('')
const editPrivateKey = ref('')
const editRedisDb = ref(0)
const editS3Endpoint = ref('')
const editS3AccessKey = ref('')
const editS3SecretKey = ref('')
const editS3Bucket = ref('')
const editS3Region = ref('')

// Context menu state
const ctxMenu = ref<{ show: boolean; x: number; y: number; resource: Resource | null }>({ show: false, x: 0, y: 0, resource: null })
const resourceDeleteId = ref<string | null>(null)

// Resource edit modal state
const resEditModal = ref(false)
const resEditName = ref('')
const resEditHost = ref('')
const resEditPort = ref<string>('')
const resEditUsername = ref('')
const resEditProtocol = ref('')
const resEditError = ref('')
const resEditLoading = ref(false)

onMounted(async () => {
  await loadEnvironment(envId.value)
})

// Re-fetch when route param changes (e.g., sidebar right-click navigates to different env)
onBeforeRouteUpdate(async (to) => {
  const newId = to.params.id as string
  if (newId && newId !== envId.value) {
    envId.value = newId
    await loadEnvironment(newId)
  }
})

async function loadEnvironment(id: string) {
  loading.value = true
  try {
    env.value = await environmentsApi.get(id)
    resources.value = await resourcesApi.listByEnv(id)
  } catch {
    router.push('/environments')
  } finally {
    loading.value = false
  }
}

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
    const updated = await store.updateEnvironment(envId.value, {
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
    await store.deleteResource(envId.value, id)
    resources.value = resources.value.filter(r => r.id !== id)
  } catch {
    // ignore
  }
}

async function refreshResources() {
  resources.value = await resourcesApi.listByEnv(envId.value)
  // 刷新环境的 resource_count
  if (env.value) {
    env.value.resource_count = resources.value.length
  }
}

function agentStatus(status: string | null): StatusDotStatus {
  if (status === 'online') return 'online'
  return 'offline'
}
// Context menu
function onContextMenu(e: MouseEvent, resource: Resource) {
  e.preventDefault()
  ctxMenu.value = { show: true, x: e.clientX, y: e.clientY, resource }
}

function closeCtxMenu() {
  ctxMenu.value.show = false
}

function openInWorkspace() {
  const res = ctxMenu.value.resource
  if (!res) return
  wsStore.openResource({
    id: res.id,
    name: res.name,
    protocol: res.protocol,
    host: res.host,
    port: res.port ?? undefined,
    username: res.username || undefined,
    environmentId: res.environment_id,
  })
  router.push('/')
  closeCtxMenu()
}

function openResEdit() {
  const res = ctxMenu.value.resource
  if (!res) return
  resEditName.value = res.name
  resEditHost.value = res.host
  resEditPort.value = res.port != null ? String(res.port) : ''
  resEditUsername.value = res.username || ''
  resEditProtocol.value = res.protocol
  resEditError.value = ''

  // 解析 config_json，填充协议特定字段
  let cfg: Record<string, unknown> = {}
  try {
    cfg = res.config_json ? JSON.parse(res.config_json) : {}
  } catch { /* ignore */ }

  editPassword.value = (cfg.password as string) || ''
  editDatabaseName.value = (cfg.database_name as string) || ''
  editFilePath.value = (cfg.file_path as string) || ''
  editPrivateKey.value = (cfg.private_key as string) || ''
  editRedisDb.value = (cfg.db as number) || 0
  editS3Endpoint.value = (cfg.endpoint as string) || ''
  editS3AccessKey.value = (cfg.access_key as string) || ''
  editS3SecretKey.value = (cfg.secret_key as string) || ''
  editS3Bucket.value = (cfg.bucket as string) || ''
  editS3Region.value = (cfg.region as string) || ''

  resEditModal.value = true
  closeCtxMenu()
}

async function submitResEdit() {
  const res = ctxMenu.value.resource
  if (!res || !resEditName.value.trim()) {
    resEditError.value = t('common.nameRequired')
    return
  }
  resEditLoading.value = true
  try {
    // 根据协议类型构建 config_json
    const cfg: Record<string, unknown> = {}
    if (['ssh', 'sftp'].includes(res.protocol)) {
      if (editPassword.value) cfg.password = editPassword.value
      if (editPrivateKey.value) cfg.private_key = editPrivateKey.value
    } else if (['mysql', 'postgresql'].includes(res.protocol)) {
      if (editPassword.value) cfg.password = editPassword.value
      if (editDatabaseName.value) cfg.database_name = editDatabaseName.value
    } else if (res.protocol === 'redis') {
      if (editPassword.value) cfg.password = editPassword.value
      cfg.db = editRedisDb.value
    } else if (res.protocol === 'sqlite') {
      cfg.file_path = editFilePath.value
    } else if (res.protocol === 's3') {
      cfg.endpoint = editS3Endpoint.value
      cfg.access_key = editS3AccessKey.value
      cfg.secret_key = editS3SecretKey.value
      cfg.bucket = editS3Bucket.value
      cfg.region = editS3Region.value || 'us-east-1'
    }

    const updated = await store.updateResource(envId.value, res.id, {
      name: resEditName.value.trim(),
      protocol: res.protocol,
      host: resEditHost.value.trim(),
      port: resEditPort.value ? Number(resEditPort.value) : null,
      username: resEditUsername.value.trim() || undefined,
      config_json: JSON.stringify(cfg),
    })
    const idx = resources.value.findIndex(r => r.id === res.id)
    if (idx >= 0) resources.value[idx] = updated
    resEditModal.value = false
  } catch (e: unknown) {
    resEditError.value = e instanceof Error ? e.message : String(e)
  } finally {
    resEditLoading.value = false
  }
}

function confirmDeleteResource() {
  if (!resourceDeleteId.value) return
  deleteResource(resourceDeleteId.value)
  resourceDeleteId.value = null
}

async function copyToken() {
  if (env.value?.registration_token) {
    try {
      await navigator.clipboard.writeText(env.value.registration_token)
    } catch {
      // fallback
      const el = document.createElement('textarea')
      el.value = env.value.registration_token!
      document.body.appendChild(el)
      el.select()
      document.execCommand('copy')
      document.body.removeChild(el)
    }
  }
}

async function resetToken() {
  if (!env.value?.id) return
  if (!confirm(t('environmentDetail.resetTokenConfirm'))) return
  try {
    // 重新生成注册令牌
    const newToken = crypto.randomUUID()
    await api.put(`/environments/${env.value.id}`, { registration_token: newToken } as Record<string, unknown>)
    env.value = await environmentsApi.get(envId.value)
  } catch (e) {
    console.error('Failed to reset token:', e)
  }
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

      <!-- Agent Panel (仅 Agent 模式显示) -->
      <Card v-if="env.connection_mode === 'agent'" class="section-card">
        <h2 class="section-title">{{ t('environmentDetail.agentSection') }}</h2>
        <div v-if="env.agent_status" class="agent-info">
          <StatusDot :status="agentStatus(env.agent_status)" />
          <span>{{ t('environments.agentStatus', { status: env.agent_status }) }}</span>
        </div>
        <div v-else class="agent-empty muted">
          {{ t('environmentDetail.noAgent') }}
        </div>
        <!-- Registration Token (仅 Agent 模式显示) -->
        <div v-if="env.connection_mode === 'agent'" class="agent-token-section">
          <label class="form-label" style="margin-bottom: var(--space-2)">
            <span>{{ t('environments.agentToken') }}</span>
          </label>
          <div v-if="env.registration_token" class="agent-token-row">
            <code class="agent-token-value mono">{{ env.registration_token }}</code>
            <Button variant="secondary" size="sm" :aria-label="t('common.copy')" @click="copyToken">{{ t('common.copy') }}</Button>
            <Button variant="secondary" size="sm" :aria-label="t('common.reset')" @click="resetToken">{{ t('common.reset') }}</Button>
          </div>
          <div v-else class="agent-token-empty muted" style="font-size: var(--text-sm)">
            {{ t('environmentDetail.noAgentToken') }}
          </div>
        </div>
        <div style="margin-top: var(--space-3); font-size: var(--text-sm);">
          <router-link to="/agents" style="color: var(--accent); text-decoration: none;">
            {{ t('environmentDetail.viewDeployGuide') }} →
          </router-link>
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
            <tr v-for="res in resources" :key="res.id" @contextmenu.prevent="onContextMenu($event, res)">
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
                <button class="icon-btn danger" :title="t('common.delete')" @click="resourceDeleteId = res.id">✕</button>
              </td>
            </tr>
          </tbody>
        </table>
      </Card>
    </template>

    <!-- Resource Creation Wizard -->
    <WizardModal
      :visible="showWizard"
      :environment-id="envId.value"
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
    <!-- Resource Context Menu -->
    <div v-if="ctxMenu.show" class="ctx-overlay" @click="closeCtxMenu" @contextmenu.prevent="closeCtxMenu" />
    <div v-if="ctxMenu.show" class="res-ctx-menu" :style="{ top: ctxMenu.y + 'px', left: ctxMenu.x + 'px' }">
      <div class="ctx-item" @click="openInWorkspace()">🚀 {{ t('resources.open') }}</div>
      <div class="ctx-item" @click="openResEdit()">✏ {{ t('resources.edit') }}</div>
      <div class="ctx-item ctx-item--danger" @click="resourceDeleteId = ctxMenu.resource?.id ?? null; closeCtxMenu()">🗑 {{ t('resources.delete') }}</div>
    </div>

    <!-- Resource Delete Confirmation -->
    <Modal :model-value="!!resourceDeleteId" @update:model-value="resourceDeleteId = null">
      <template #title>{{ t('resources.delete') }}</template>
      <p style="color: var(--text-secondary); margin-bottom: 16px">
        {{ t('environments.deleteConfirm') }}
      </p>
      <div class="form-actions">
        <Button variant="secondary" @click="resourceDeleteId = null">{{ t('common.cancel') }}</Button>
        <Button variant="danger" @click="confirmDeleteResource">{{ t('common.delete') }}</Button>
      </div>
    </Modal>

    <!-- Resource Edit Modal -->
    <Modal v-model="resEditModal">
      <template #title>{{ t('resources.edit') }}</template>
      <form class="env-form" @submit.prevent="submitResEdit">
        <label class="form-label">
          <span>{{ t('common.name') }}</span>
          <input v-model="resEditName" type="text" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.host') }}</span>
          <input v-model="resEditHost" type="text" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.port') }}</span>
          <input v-model="resEditPort" type="number" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.username') }}</span>
          <input v-model="resEditUsername" type="text" class="form-input" />
        </label>
        <!-- SSH / SFTP fields -->
        <template v-if="['ssh', 'sftp'].includes(resEditProtocol)">
          <label class="form-label">
            <span>{{ t('wizard.password') }}</span>
            <input v-model="editPassword" type="password" class="form-input" />
          </label>
          <label class="form-label">
            <span>{{ t('wizard.privateKey') }}</span>
            <textarea v-model="editPrivateKey" class="form-input" rows="3" />
          </label>
        </template>
        <!-- MySQL / PostgreSQL fields -->
        <template v-if="['mysql', 'postgresql'].includes(resEditProtocol)">
          <label class="form-label">
            <span>{{ t('wizard.password') }}</span>
            <input v-model="editPassword" type="password" class="form-input" />
          </label>
          <label class="form-label">
            <span>{{ t('wizard.database') }}</span>
            <input v-model="editDatabaseName" type="text" class="form-input" />
          </label>
        </template>
        <!-- Redis fields -->
        <template v-if="resEditProtocol === 'redis'">
          <label class="form-label">
            <span>{{ t('wizard.password') }}</span>
            <input v-model="editPassword" type="password" class="form-input" />
          </label>
          <label class="form-label">
            <span>{{ t('wizard.redisDb') }}</span>
            <input v-model.number="editRedisDb" type="number" class="form-input" min="0" />
          </label>
        </template>
        <!-- SQLite fields -->
        <template v-if="resEditProtocol === 'sqlite'">
          <label class="form-label">
            <span>{{ t('wizard.filePath') }}</span>
            <input v-model="editFilePath" type="text" class="form-input" />
          </label>
        </template>
        <!-- S3 fields -->
        <template v-if="resEditProtocol === 's3'">
          <label class="form-label">
            <span>{{ t('wizard.s3Endpoint') }}</span>
            <input v-model="editS3Endpoint" type="text" class="form-input" />
          </label>
          <label class="form-label">
            <span>{{ t('wizard.s3AccessKey') }}</span>
            <input v-model="editS3AccessKey" type="text" class="form-input" />
          </label>
          <label class="form-label">
            <span>{{ t('wizard.s3SecretKey') }}</span>
            <input v-model="editS3SecretKey" type="password" class="form-input" />
          </label>
          <label class="form-label">
            <span>{{ t('wizard.s3Bucket') }}</span>
            <input v-model="editS3Bucket" type="text" class="form-input" />
          </label>
          <label class="form-label">
            <span>{{ t('wizard.s3Region') }}</span>
            <input v-model="editS3Region" type="text" class="form-input" />
          </label>
        </template>
        <div v-if="resEditError" class="form-error">{{ resEditError }}</div>
        <div class="form-actions">
          <Button type="button" variant="secondary" @click="resEditModal = false">{{ t('common.cancel') }}</Button>
          <Button type="submit" variant="primary" :loading="resEditLoading">{{ t('common.save') }}</Button>
        </div>
      </form>
    </Modal>
  </div>
</template>

<style scoped>
.env-detail {
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
.agent-token-section {
  margin-top: var(--space-4);
  padding-top: var(--space-4);
  border-top: 1px solid var(--border);
}
.agent-token-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.agent-token-value {
  flex: 1;
  font-size: var(--text-xs);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--space-2);
  word-break: break-all;
  color: var(--text-secondary);
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
/* ---- context menu ---- */
.ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 200;
}
.res-ctx-menu {
  position: fixed;
  z-index: 210;
  min-width: 160px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  padding: var(--space-1) 0;
}
.ctx-item {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  cursor: pointer;
  color: var(--text-primary);
}
.ctx-item:hover {
  background: var(--bg-hover);
}
.ctx-item--danger {
  color: var(--danger);
}

</style>
