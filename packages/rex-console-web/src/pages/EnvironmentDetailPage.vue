<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter, onBeforeRouteUpdate } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useEnvironmentsStore } from '@/stores/environments'
import { useWorkspaceStore } from '@/stores/workspace'
import { environmentsApi, type Environment } from '@/api/environments'
import type { Resource } from '@/api/resources'
import { api } from '@/api/client'
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Modal from '@/components/ui/Modal.vue'
import WizardModal from '@/features/resource/WizardModal.vue'
import { PROTOCOL_ICONS, PROTOCOL_COLORS, PROTOCOL_NAMES, SUBTYPE_META } from '@/features/resource/protocols'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const store = useEnvironmentsStore()
const wsStore = useWorkspaceStore()

const envId = ref(route.params.id as string)
const env = ref<Environment | null>(null)
// 资源列表直接取自共享 store（envResources），保证侧栏新建/删除资源时本页实时刷新
const resources = computed<Resource[]>(() => store.envResources.get(envId.value) || [])
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
const deleteConfirmId = ref<string | ''>('')

// Resource edit modal state
const resEditModal = ref(false)
const resEditName = ref('')
const resEditHost = ref('')
const resEditPort = ref<string>('')
const resEditUsername = ref('')
const resEditProtocol = ref('')
// v0.70.7：SQL 资源的子类（dialect）。编辑弹窗按此分流字段，避免保存时丢失 file_path / 凭据。
const resEditSubtype = ref('')
const resEditError = ref('')
const resEditLoading = ref(false)
const resEditTesting = ref(false)
const resEditTestResult = ref<{ ok: boolean; msg: string } | null>(null)

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
    await store.fetchResources(id)
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

async function deleteEnvironment() {
  const id = deleteConfirmId.value
  deleteConfirmId.value = ''
  if (!id) return
  try {
    await store.deleteEnvironment(id)
    router.push('/environments')
  } catch (e: unknown) {
    editError.value = e instanceof Error ? e.message : String(e)
  }
}

async function deleteResource(id: string) {
  try {
    await store.deleteResource(envId.value, id)
  } catch {
    // ignore
  }
}

async function refreshResources() {
  await store.fetchResources(envId.value)
  // 刷新环境的 resource_count
  if (env.value) {
    env.value.resource_count = (store.envResources.get(envId.value) || []).length
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

// v0.70.7：SQL 资源合并为单一「SQL」协议，展示时按探测出的子类（dialect）着色 / 命名。
// 旧 mysql/postgresql/sqlite 资源已在 DB 迁移为 sql+subtype，这里统一按 subtype 解析。
function resProtoIcon(res: { protocol: string; subtype?: string | null }): string {
  if (res.protocol === 'sql') return SUBTYPE_META[res.subtype ?? '']?.icon ?? 'dB'
  return PROTOCOL_ICONS[res.protocol] || '?'
}
function resProtoColor(res: { protocol: string; subtype?: string | null }): string {
  if (res.protocol === 'sql') return SUBTYPE_META[res.subtype ?? '']?.color ?? 'var(--info)'
  return PROTOCOL_COLORS[res.protocol] || 'var(--text-secondary)'
}
function resProtoName(res: { protocol: string; subtype?: string | null }): string {
  if (res.protocol === 'sql') return SUBTYPE_META[res.subtype ?? '']?.name ?? 'SQL'
  return PROTOCOL_NAMES[res.protocol] || res.protocol
}
function resProtoTone(res: { protocol: string; subtype?: string | null }): 'success' | 'info' | 'purple' | 'danger' | 'warning' | 'accent' {
  if (res.protocol === 'sql') {
    const sub = res.subtype ?? ''
    if (sub === 'postgresql') return 'purple'
    if (sub === 'sqlite') return 'warning'
    return 'info'
  }
  if (res.protocol === 'redis') return 'danger'
  if (res.protocol === 'ssh') return 'success'
  return 'info'
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
    subtype: res.subtype ?? undefined,
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
  resEditSubtype.value = (res.subtype as string) || ''
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
    } else if (res.protocol === 'sql') {
      // v0.70.7：SQL 资源按子类（dialect）分流；file_path 仅在 SQLite 子类下保留。
      if (res.subtype === 'sqlite') {
        cfg.file_path = editFilePath.value
      } else {
        if (editPassword.value) cfg.password = editPassword.value
        if (editDatabaseName.value) cfg.database_name = editDatabaseName.value
      }
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
    const list = store.envResources.get(envId.value)
    if (list) {
      const idx = list.findIndex(r => r.id === res.id)
      if (idx >= 0) {
        list[idx] = updated
        store.envResources.set(envId.value, [...list])
      }
    }
    resEditModal.value = false
  } catch (e: unknown) {
    resEditError.value = e instanceof Error ? e.message : String(e)
  } finally {
    resEditLoading.value = false
  }
}

async function testResConnection() {
  if (!resEditName.value.trim() || !resEditHost.value.trim()) {
    resEditTestResult.value = { ok: false, msg: t('wizard.hostRequired') }
    return
  }
  const cfg: Record<string, unknown> = {}
  if (['ssh', 'sftp'].includes(resEditProtocol.value)) {
    if (editPassword.value) cfg.password = editPassword.value
    if (editPrivateKey.value) cfg.private_key = editPrivateKey.value
  } else if (resEditProtocol.value === 'sql') {
    // v0.70.7：SQL 资源按子类（dialect）分流；SQLite 子类仅用 file_path 测试连接。
    if (resEditSubtype.value === 'sqlite') {
      cfg.file_path = editFilePath.value
    } else {
      if (editPassword.value) cfg.password = editPassword.value
      if (editDatabaseName.value) cfg.database_name = editDatabaseName.value
    }
  } else if (['mysql', 'postgresql'].includes(resEditProtocol.value)) {
    if (editPassword.value) cfg.password = editPassword.value
    if (editDatabaseName.value) cfg.database_name = editDatabaseName.value
  } else if (resEditProtocol.value === 'redis') {
    if (editPassword.value) cfg.password = editPassword.value
    cfg.db = editRedisDb.value
  } else if (resEditProtocol.value === 'sqlite') {
    cfg.file_path = editFilePath.value
  } else if (resEditProtocol.value === 's3') {
    cfg.endpoint = editS3Endpoint.value
    cfg.access_key = editS3AccessKey.value
    cfg.secret_key = editS3SecretKey.value
    cfg.bucket = editS3Bucket.value
    cfg.region = editS3Region.value || 'us-east-1'
  }
  resEditTesting.value = true
  resEditTestResult.value = null
  try {
    const res = await store.testConnection({
      protocol: resEditProtocol.value,
      host: resEditHost.value.trim(),
      port: resEditPort.value ? Number(resEditPort.value) : null,
      username: resEditUsername.value.trim() || undefined,
      config_json: JSON.stringify(cfg),
      environment_id: envId.value,
    })
    resEditTestResult.value = res.ok
      ? { ok: true, msg: t('wizard.testSuccess') }
      : { ok: false, msg: res.error || t('wizard.testFailed') }
  } catch (e: unknown) {
    resEditTestResult.value = { ok: false, msg: e instanceof Error ? e.message : t('wizard.testFailed') }
  } finally {
    resEditTesting.value = false
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
      <router-link to="/environments" class="breadcrumb-link">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h16M4 12h16M4 17h16"/></svg>
        Environments
      </router-link>
      <svg class="breadcrumb-sep" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18l6-6-6-6"/></svg>
      <span class="breadcrumb-current">{{ env?.name || '...' }}</span>
    </div>

    <div v-if="loading" class="loading">{{ t('common.loadingEllipsis') }}</div>

    <template v-else-if="env">
      <!-- Detail Head -->
      <div class="detail-head">
        <div class="detail-head-icon" :class="`detail-head-icon--${env.connection_mode}`">
          {{ env.connection_mode === 'agent' ? '⬡' : '◉' }}
        </div>
        <div class="detail-head-text">
          <div class="detail-head-name">{{ env.name }}</div>
          <div class="detail-head-sub">{{ env.description || t('common.noDescription') }}</div>
        </div>
        <div class="detail-head-spacer"></div>
        <Button variant="ghost" size="sm" @click="openEdit">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
          Edit
        </Button>
        <Button variant="ghost" size="sm" style="color: var(--danger);" @click="deleteConfirmId = env.id ?? ''">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
          Delete
        </Button>
      </div>

      <!-- Meta -->
      <div class="detail-meta">
        <Badge :tone="env.connection_mode === 'agent' ? 'warning' : 'info'">
          {{ env.connection_mode === 'agent' ? 'Agent Tunnel' : 'Direct' }}
        </Badge>
        <Badge :tone="env.agent_status === 'online' ? 'success' : 'warning'">
          {{ env.agent_status || 'no agent' }}
        </Badge>
        <Badge>{{ env.resource_count }} resources</Badge>
        <span class="detail-meta-updated">
          Updated {{ new Date(env.updated_at).toLocaleDateString() }}
        </span>
      </div>

      <!-- Agents Section -->
      <div class="section">
        <div class="section-head">
          <h2 class="section-title">Agents</h2>
          <span class="section-head-spacer"></span>
          <Badge>{{ env.agent_status === 'online' ? '1 online' : '0 online' }}</Badge>
        </div>
        <div class="section-body">
          <div v-if="env.agent_status" class="agent-row">
            <span class="agent-icon">⟡</span>
            <div class="agent-info">
              <span class="agent-name">{{ env.agent_status }}</span>
              <span class="agent-meta">Registered agent</span>
            </div>
            <span class="agent-version mono">1.0.0</span>
            <StatusDot :status="agentStatus(env.agent_status)" />
          </div>
          <div v-else class="agent-empty">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
            No agents registered for this environment.
          </div>
        </div>
        <!-- Registration Token -->
        <div v-if="env.connection_mode === 'agent'" class="agent-token-section">
          <label class="form-label" style="margin-bottom: var(--space-2)">
            <span>{{ t('environments.agentToken') }}</span>
          </label>
          <div v-if="env.registration_token" class="agent-token-row">
            <code class="agent-token-value mono">{{ env.registration_token }}</code>
            <Button variant="ghost" size="sm" :aria-label="t('common.copy')" @click="copyToken">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
              {{ t('common.copy') }}
            </Button>
            <Button variant="ghost" size="sm" :aria-label="t('common.reset')" @click="resetToken">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
              {{ t('common.reset') }}
            </Button>
          </div>
          <div v-else class="agent-token-empty muted" style="font-size: var(--text-sm)">
            {{ t('environmentDetail.noAgentToken') }}
          </div>
        </div>
        <div style="margin-top: var(--space-3); font-size: var(--text-sm);">
          <router-link to="/agents" class="section-link">
            {{ t('environmentDetail.viewDeployGuide') }}
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14M12 5l7 7-7 7"/></svg>
          </router-link>
        </div>
      </div>

      <!-- Resources Section -->
      <div class="section">
        <div class="section-head">
          <h2 class="section-title">Resources</h2>
          <span class="section-head-spacer"></span>
          <Button variant="primary" size="sm" @click="showWizard = true">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 5v14M5 12h14"/></svg>
            Add Resource
          </Button>
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
                  <span class="res-icon" :style="{ color: resProtoColor(res) }">
                    {{ resProtoIcon(res) }}
                  </span>
                  {{ res.name }}
                </span>
              </td>
              <td>
                <Badge :tone="resProtoTone(res)">
                  {{ resProtoName(res) }}
                </Badge>
              </td>
              <td class="mono">{{ res.host }}</td>
              <td class="mono">{{ res.port || '—' }}</td>
              <td>{{ res.username || '—' }}</td>
              <td>
                <button class="icon-btn danger" :title="t('common.delete')" @click="resourceDeleteId = res.id">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
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
          <Button type="button" variant="ghost" @click="editModal = false">{{ t('common.cancel') }}</Button>
          <Button type="submit" variant="primary" :loading="editLoading">{{ t('common.save') }}</Button>
        </div>
      </form>
    </Modal>

    <!-- Environment Delete Confirmation -->
    <Modal :model-value="!!deleteConfirmId" @update:model-value="deleteConfirmId = ''">
      <template #title>{{ t('environments.deleteEnvironment') }}</template>
      <p style="color: var(--text-secondary); margin-bottom: 16px">
        {{ t('environments.deleteConfirm') }}
      </p>
      <div class="form-actions">
        <Button variant="ghost" @click="deleteConfirmId = ''">{{ t('common.cancel') }}</Button>
        <Button variant="danger" :loading="false" @click="deleteEnvironment">{{ t('common.delete') }}</Button>
      </div>
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
        <Button variant="ghost" @click="resourceDeleteId = null">{{ t('common.cancel') }}</Button>
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
        <!-- SQL (unified) fields：非 SQLite 子类时显示密码 / 数据库 -->
        <template v-if="resEditProtocol === 'sql' && resEditSubtype !== 'sqlite'">
          <label class="form-label">
            <span>{{ t('wizard.password') }}</span>
            <input v-model="editPassword" type="password" class="form-input" />
          </label>
          <label class="form-label">
            <span>{{ t('wizard.database') }}</span>
            <input v-model="editDatabaseName" type="text" class="form-input" />
          </label>
        </template>
        <!-- SQL / SQLite 子类：文件路径 -->
        <template v-if="resEditProtocol === 'sql' && resEditSubtype === 'sqlite'">
          <label class="form-label">
            <span>{{ t('wizard.filePath') }}</span>
            <input v-model="editFilePath" type="text" class="form-input" />
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
          <Button type="button" variant="ghost" :loading="resEditTesting" @click="testResConnection">{{ t('wizard.testConnection') }}</Button>
          <Button type="button" variant="ghost" @click="resEditModal = false">{{ t('common.cancel') }}</Button>
          <Button type="submit" variant="primary" :loading="resEditLoading">{{ t('common.save') }}</Button>
        </div>
        <div v-if="resEditTestResult" class="form-error" :style="{ color: resEditTestResult.ok ? 'var(--success)' : 'var(--danger)' }">
          {{ resEditTestResult.msg }}
        </div>
      </form>
    </Modal>
  </div>
</template>

<style scoped>
/* ========== Page Layout ========== */
.env-detail {
  padding: var(--space-6);
}

/* ========== Breadcrumb ========== */
.breadcrumb {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-6);
  font-size: var(--text-sm);
}

.breadcrumb-link {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--text-muted);
  text-decoration: none;
  transition: color var(--transition);
}

.breadcrumb-link:hover {
  color: var(--accent);
}

.breadcrumb-sep {
  color: var(--text-muted);
  flex-shrink: 0;
}

.breadcrumb-current {
  color: var(--text-primary);
  font-weight: 600;
}

/* ========== Loading ========== */
.loading {
  color: var(--text-muted);
  padding: var(--space-8) 0;
  text-align: center;
}

/* ========== Detail Head ========== */
.detail-head {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  margin-bottom: 8px;
}

.detail-head-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-family: var(--font-mono);
  font-size: 18px;
  font-weight: 700;
  color: var(--on-ink);
  flex-shrink: 0;
}

.detail-head-icon--direct {
  background: linear-gradient(140deg, var(--info), #2a6cb8);
}

.detail-head-icon--agent {
  background: linear-gradient(140deg, var(--accent), var(--brand-deep));
}

.detail-head-text {
  flex: 1;
  min-width: 0;
}

.detail-head-name {
  font-size: var(--text-xl);
  font-weight: 700;
  color: var(--text-primary);
  line-height: 1.3;
}

.detail-head-sub {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin-top: 2px;
}

.detail-head-spacer {
  flex: 1;
}

/* ========== Meta ========== */
.detail-meta {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 6px 0 22px;
  flex-wrap: wrap;
}

.detail-meta-updated {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-muted);
  margin-left: var(--space-2);
}

/* ========== Section ========== */
.section {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--space-4) var(--space-5);
  margin-bottom: var(--space-5);
}

.section-head {
  display: flex;
  align-items: center;
  margin-bottom: var(--space-4);
}

.section-head-spacer {
  flex: 1;
}

.section-title {
  font-size: 13px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
  font-family: var(--font-mono);
  margin: 0;
  font-weight: 500;
}

.section-body {
  /* body wrapper */
}

.section-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--accent);
  text-decoration: none;
  font-size: var(--text-sm);
  transition: color var(--transition);
}

.section-link:hover {
  color: var(--accent-hover);
}

/* ========== Agent Row ========== */
.agent-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
}

.agent-row + .agent-row {
  border-top: 1px dashed var(--border);
}

.agent-icon {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  display: grid;
  place-items: center;
  background: linear-gradient(140deg, var(--accent), var(--brand-deep));
  color: var(--on-brand);
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: 11px;
  flex-shrink: 0;
}

.agent-info {
  flex: 1;
  min-width: 0;
}

.agent-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.agent-meta {
  display: block;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-muted);
}

.agent-version {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-muted);
}

.agent-empty {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: var(--text-sm);
  padding: var(--space-3) 0;
}

/* ========== Agent Token ========== */
.agent-token-section {
  margin-top: var(--space-4);
  padding-top: var(--space-4);
  border-top: 1px solid var(--border);
}

.agent-token-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.agent-token-value {
  flex: 1;
  min-width: 200px;
  font-size: var(--text-xs);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--space-2);
  word-break: break-all;
  color: var(--text-muted);
  display: inline-block;
}

.agent-token-empty {
  color: var(--text-muted);
}

/* ========== Resource Table ========== */
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
  flex-shrink: 0;
}

/* ========== Icon Button ========== */
.icon-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background var(--transition), color var(--transition);
}

.icon-btn:hover {
  background: var(--bg-hover);
}

.icon-btn.danger:hover {
  color: var(--danger);
}

/* ========== Form ========== */
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
  transition: border-color var(--transition);
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

/* ========== Context Menu ========== */
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
  transition: background var(--transition);
}

.ctx-item:hover {
  background: var(--bg-hover);
}

.ctx-item--danger {
  color: var(--danger);
}

/* ========== Utilities ========== */
.mono {
  font-family: var(--font-mono);
}

.muted {
  color: var(--text-muted);
}
</style>
