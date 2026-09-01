<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useEnvironmentsStore } from '@/stores/environments'
import { environmentsApi } from '@/api/environments'
import type { ExportData, Environment } from '@/api/environments'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import ContextMenu from '@/components/ui/ContextMenu.vue'
import Modal from '@/components/ui/Modal.vue'
import Input from '@/components/ui/Input.vue'
import Select from '@/components/ui/Select.vue'
import { agentStatus } from '@/utils/status'

const { t } = useI18n()
const router = useRouter()
const store = useEnvironmentsStore()

const showCreateModal = ref(false)
const editingEnv = ref<{ id: string; name: string; description: string; connection_mode: string } | null>(null)
const formName = ref('')
const formDesc = ref('')
const formMode = ref('direct')
const formError = ref('')
const formLoading = ref(false)
const deleteConfirmId = ref<string | null>(null)
const ctxMenu = ref<{ show: boolean; x: number; y: number; env: Environment | null }>({ show: false, x: 0, y: 0, env: null })

const activeView = ref<'list' | 'topology'>('list')

function onContextMenu(e: MouseEvent, env: Environment) {
  e.preventDefault()
  ctxMenu.value = { show: true, x: e.clientX, y: e.clientY, env }
}

onMounted(async () => {
  await store.fetchEnvironments()
  for (const env of store.environments) {
    await store.fetchResources(env.id)
  }
})

const envProtocols = computed(() => {
  const map = new Map<string, string[]>()
  for (const env of store.environments) {
    const resources = store.envResources.get(env.id) || []
    map.set(env.id, [...new Set(resources.map((r: { protocol: string }) => r.protocol))])
  }
  return map
})

const hasEnvironments = computed(() => store.environments.length > 0)

function openCreate() {
  editingEnv.value = null
  formName.value = ''
  formDesc.value = ''
  formMode.value = 'direct'
  formError.value = ''
  showCreateModal.value = true
}

function openEdit(env: typeof editingEnv.value extends null ? never : NonNullable<typeof editingEnv.value>) {
  editingEnv.value = env
  formName.value = env.name
  formDesc.value = env.description
  formMode.value = env.connection_mode
  formError.value = ''
  showCreateModal.value = true
}

async function submitForm() {
  if (!formName.value.trim()) {
    formError.value = t('common.nameRequired')
    return
  }
  formLoading.value = true
  formError.value = ''
  try {
    if (editingEnv.value) {
      await store.updateEnvironment(editingEnv.value.id, {
        name: formName.value.trim(),
        description: formDesc.value.trim(),
        connection_mode: formMode.value,
      })
    } else {
      await store.createEnvironment({
        name: formName.value.trim(),
        description: formDesc.value.trim(),
        connection_mode: formMode.value,
      })
    }
    showCreateModal.value = false
    // Signal sidebar to refresh
    window.dispatchEvent(new CustomEvent('rex:env-changed'))
  } catch (e: unknown) {
    formError.value = e instanceof Error ? e.message : String(e)
  } finally {
    formLoading.value = false
  }
}

async function confirmDelete() {
  if (!deleteConfirmId.value) return
  try {
    await store.deleteEnvironment(deleteConfirmId.value)
  } catch {
    // ignore
  }
  deleteConfirmId.value = null
}

const protoIcon: Record<string, string> = {
  ssh: '$', sftp: '📁', sql: 'dB', mysql: 'dB', postgresql: 'pg', redis: 'R', sqlite: '◇', s3: '☁', sip: '☎',
}
function envIcon(mode: string): string {
  return mode === 'agent' ? '⬡' : '◉'
}

// Export / Import
const importFileInput = ref<HTMLInputElement | null>(null)
const importLoading = ref(false)

async function exportConfig() {
  try {
    const data = await environmentsApi.export()
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `rex-config-${new Date().toISOString().slice(0, 10)}.json`
    a.click()
    URL.revokeObjectURL(url)
  } catch (e) {
    console.error('Export failed:', e)
  }
}

function triggerImport() {
  importFileInput.value?.click()
}

async function handleImport(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  importLoading.value = true
  try {
    const text = await file.text()
    const data: ExportData = JSON.parse(text)
    if (!data.environments || !Array.isArray(data.environments)) {
      throw new Error(t('environments.importInvalidFile'))
    }
    const result = await environmentsApi.import(data)
    await store.fetchEnvironments()
    alert(t('environments.importSuccess', { imported: result.imported, skipped: result.skipped }))
  } catch (e) {
    console.error('Import failed:', e)
    alert(t('environments.importFailed', { error: e instanceof Error ? e.message : String(e) }))
  } finally {
    importLoading.value = false
    input.value = ''
  }
}
</script>

<template>
  <div class="env-page">
    <!-- Page Header -->
    <div class="page-header">
      <div class="page-header-text">
        <h1 class="page-title">Environments</h1>
        <p class="page-sub">Logical groupings of resources by network or purpose. Each owns its connection mode, agents and resources — agents are managed inside their environment. Switch to <b>Topology</b> to see how everything is wired.</p>
      </div>
    </div>

    <!-- Toolbar -->
    <div class="toolbar">
      <span class="badge-item">
        <Badge>{{ store.environments.length }} environments</Badge>
      </span>
      <span class="badge-item">
        <Badge tone="success">{{ store.environments.filter(e => e.agent_status === 'online').length }} agents online</Badge>
      </span>
      <span class="badge-item">
        <Badge>{{ store.environments.reduce((sum, e) => sum + e.resource_count, 0) }} resources</Badge>
      </span>
      <div class="toolbar-spacer"></div>
      <Button variant="ghost" size="sm" :loading="importLoading" @click="triggerImport">Import</Button>
      <input ref="importFileInput" type="file" accept=".json" style="display:none" @change="handleImport" />
      <Button variant="primary" size="sm" @click="openCreate">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 5v14M5 12h14"/></svg>
        New environment
      </Button>
    </div>

    <!-- Tab Bar -->
    <div class="tab-bar">
      <div class="tab" :class="{ active: activeView === 'list' }" @click="activeView = 'list'">List</div>
      <div class="tab" :class="{ active: activeView === 'topology' }" @click="activeView = 'topology'">Topology</div>
    </div>

    <!-- Empty State -->
    <EmptyState
      v-if="!store.loading && !hasEnvironments"
      icon="⛁"
      :title="t('environments.noEnvironments')"
      :description="t('environments.emptyDescription')"
    >
      <Button variant="primary" @click="openCreate">{{ t('environments.createEnvironment') }}</Button>
    </EmptyState>

    <!-- Environment Grid -->
    <div v-else-if="activeView === 'list'" class="env-grid">
      <div
        v-for="env in store.environments"
        :key="env.id"
        class="env-card"
        role="button"
        tabindex="0"
        @click="router.push(`/environments/${env.id}`)"
        @keydown.enter="router.push(`/environments/${env.id}`)"
        @contextmenu.prevent="onContextMenu($event, env)"
      >
        <!-- Card Header -->
        <div class="env-card-header">
          <div class="env-card-icon" :class="`env-card-icon--${env.connection_mode}`">
            {{ envIcon(env.connection_mode) }}
          </div>
          <div class="env-card-header-text">
            <span class="env-card-name">{{ env.name }}</span>
            <span class="env-card-mode">{{ env.connection_mode }}</span>
          </div>
          <StatusDot v-if="env.agent_status" :status="agentStatus(env.agent_status)" />
        </div>

        <!-- Card Body -->
        <div class="env-card-body">
          <div class="env-card-desc">{{ env.description || t('common.noDescription') }}</div>

          <div class="env-card-chips">
            <span class="chip">
              <span class="chip-dot" :class="env.agent_status === 'online' ? 'chip-dot--success' : 'chip-dot--warning'"></span>
              {{ env.resource_count }} resources
            </span>
          </div>
        </div>

        <!-- Protocol Icons -->
        <div v-if="envProtocols.get(env.id)?.length" class="env-card-protocols">
          <span
            v-for="proto in envProtocols.get(env.id)"
            :key="proto"
            class="env-proto-pico"
            :class="`pico--${proto}`"
          >{{ protoIcon[proto] || '?' }}</span>
        </div>
        <!-- Agents Section -->
        <div v-if="env.connection_mode === 'agent'" class="env-card-agents">
          <div class="env-card-agents-header">
            Agents
            <div class="env-card-agents-spacer"></div>
            <span v-if="env.agent_status" class="badge-sm badge-green">1 online</span>
          </div>
          <div v-if="env.agent_status" class="env-card-agent-row">
            <span class="env-card-agent-icon">⟡</span>
            <div class="env-card-agent-info">
              <span class="env-card-agent-name">{{ env.agent_status }}</span>
            </div>
            <StatusDot :status="agentStatus(env.agent_status)" />
          </div>
          <div v-else class="env-card-no-agent">No agents registered</div>
        </div>

        <!-- Card Footer Actions -->
        <div class="env-card-actions">
          <button class="env-card-action" @click.stop="router.push(`/environments/${env.id}`)">Open</button>
          <button class="env-card-action env-card-action--primary" @click.stop="router.push(`/environments/${env.id}?action=newResource`)">Add Resource</button>
          <button class="env-card-action" @click.stop="openEdit(env)">Edit</button>
          <button class="env-card-action env-card-action--danger" @click.stop="deleteConfirmId = env.id">Delete</button>
        </div>
      </div>

      <!-- New Environment Card -->
      <div class="env-card env-card--new" @click="openCreate">
        <div class="env-card-new-content">
          <div class="env-card-new-icon">+</div>
          <div class="env-card-new-text">New environment</div>
        </div>
      </div>
    </div>

    <!-- Topology placeholder -->
    <div v-else-if="activeView === 'topology'" class="env-grid">
      <div class="env-card" style="grid-column: 1 / -1; min-height: 200px; display: flex; align-items: center; justify-content: center;">
        <div style="text-align: center; color: var(--text-muted);">
          <div style="font-size: 32px; margin-bottom: 12px;">⛁</div>
          <div style="font-size: 14px; font-weight: 600; margin-bottom: 4px;">Topology View</div>
          <div style="font-size: 12px;">Coming soon — network topology visualization</div>
        </div>
      </div>
    </div>

    <!-- Create / Edit Modal -->
    <Modal v-model="showCreateModal">
      <template #title>{{ editingEnv ? t('environments.editEnvironment') : t('environments.newEnvironment') }}</template>
      <form class="env-form" @submit.prevent="submitForm">
        <label class="form-label">
          <span class="form-label-text">{{ t('common.name') }}</span>
          <Input v-model="formName" :placeholder="t('environments.placeholderName')" autofocus />
        </label>
        <label class="form-label">
          <span class="form-label-text">{{ t('common.description') }}</span>
          <Input v-model="formDesc" :placeholder="t('environments.placeholderDescription')" />
        </label>
        <label class="form-label" @click.stop>
          <span class="form-label-text">{{ t('environments.connectionMode') }}</span>
          <Select v-model="formMode" :options="[
            { label: t('environments.direct'), value: 'direct' },
            { label: t('environments.agent'), value: 'agent' },
          ]" />
        </label>
        <div v-if="formError" class="form-error">{{ formError }}</div>
        <div class="form-actions">
          <Button type="button" variant="ghost" @click="showCreateModal = false">{{ t('common.cancel') }}</Button>
          <Button type="submit" variant="primary" :loading="formLoading">
            {{ editingEnv ? t('common.save') : t('common.create') }}
          </Button>
        </div>
      </form>
    </Modal>

    <!-- Delete Confirmation -->
    <Modal :model-value="!!deleteConfirmId" @update:model-value="deleteConfirmId = null">
      <template #title>{{ t('environments.deleteEnvironment') }}</template>
      <p class="delete-msg">{{ t('environments.deleteConfirm') }}</p>
      <div class="form-actions">
        <Button variant="ghost" @click="deleteConfirmId = null">{{ t('common.cancel') }}</Button>
        <Button variant="danger" @click="confirmDelete">{{ t('common.delete') }}</Button>
      </div>
    </Modal>

    <!-- Context Menu -->
    <ContextMenu
      v-model="ctxMenu.show"
      :x="ctxMenu.x"
      :y="ctxMenu.y"
      @select="(key: string) => {
        if (key === 'edit') router.push(`/environments/${ctxMenu.env!.id}`)
        else if (key === 'newResource') router.push(`/environments/${ctxMenu.env!.id}?action=newResource`)
        else if (key === 'delete') deleteConfirmId = ctxMenu.env!.id
      }"
    >
      <template #default="{ choose }">
        <div class="ctx-item" @click="choose('edit')">✏ {{ t('environments.edit') }}</div>
        <div class="ctx-item" @click="choose('newResource')">➕ {{ t('environments.newResource') }}</div>
        <div class="ctx-item ctx-item--danger" @click="choose('delete')">🗑 {{ t('environments.delete') }}</div>
      </template>
    </ContextMenu>
  </div>
</template>

<style scoped>
/* ========== Page Layout ========== */
.env-page {
  padding: var(--space-6);
}

.page-header {
  margin-bottom: var(--space-4);
}

.page-header-text {
  max-width: 720px;
}

.page-title {
  font-size: var(--text-xl);
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 var(--space-2) 0;
}

.page-sub {
  font-size: var(--text-sm);
  color: var(--text-muted);
  line-height: 1.5;
  margin: 0;
}

.page-sub b {
  color: var(--text-secondary);
}

/* ========== Toolbar ========== */
.toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: var(--space-4);
}

.toolbar-spacer {
  flex: 1;
}

.badge-item {
  display: inline-flex;
}

/* ========== Tab Bar ========== */
.tab-bar {
  display: flex;
  gap: 2px;
  margin-bottom: var(--space-5);
  border-bottom: 1px solid var(--border);
}

.tab {
  padding: 9px 18px;
  font-size: var(--text-base);
  color: var(--text-muted);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
  font-weight: 500;
  transition: color var(--transition);
}

.tab:hover {
  color: var(--text-primary);
}

.tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
  font-weight: 600;
}

/* ========== Environment Grid ========== */
.env-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(330px, 1fr));
  gap: 16px;
}

/* ========== Environment Card ========== */
.env-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
  transition: border-color 0.15s, transform 0.08s;
  display: flex;
  flex-direction: column;
}

.env-card:hover {
  border-color: var(--border-strong);
  transform: translateY(-2px);
}

.env-card[role="button"] {
  cursor: pointer;
}

/* Card Header */
.env-card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
  border-bottom: 1px solid var(--border);
}

.env-card-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: 700;
  color: var(--on-ink);
  flex-shrink: 0;
}

.env-card-icon--direct {
  background: linear-gradient(140deg, var(--info), #2a6cb8);
}

.env-card-icon--agent {
  background: linear-gradient(140deg, var(--accent), var(--brand-deep));
}

.env-card-header-text {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
}

.env-card-name {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-primary);
}

.env-card-mode {
  font-size: 12px;
  font-family: var(--font-mono);
  color: var(--text-muted);
}

/* Card Body */
.env-card-body {
  padding: 14px 16px;
}

.env-card-desc {
  color: var(--text-muted);
  font-size: 12.5px;
  min-height: 32px;
  margin-bottom: 14px;
  line-height: 1.4;
}

.env-card-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-muted);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
  padding: 3px 9px;
}

.chip-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
}

.chip-dot--success {
  background: var(--success);
}

.chip-dot--warning {
  background: var(--warning);
}

.env-card-protocols {
  display: flex;
  gap: 6px;
  margin-top: 8px;
  padding: 0 16px;
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
.env-proto-pico.pico--sql,
.env-proto-pico.pico--mysql { background: var(--info); }
.env-proto-pico.pico--postgresql { background: var(--purple); }
.env-proto-pico.pico--redis { background: var(--danger); }
.env-proto-pico.pico--sqlite { background: var(--warning); }
.env-proto-pico.pico--s3 { background: var(--brand); }
.env-proto-pico.pico--sip { background: var(--teal); }

/* Agents Section */
.env-card-agents {
  border-top: 1px solid var(--border);
  padding: 12px 16px;
  background: var(--bg-page);
}

.env-card-agents-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  font-family: var(--font-mono);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.env-card-agents-spacer {
  flex: 1;
}

.badge-sm {
  display: inline-flex;
  align-items: center;
  font-size: 10px;
  font-family: var(--font-mono);
  padding: 2px 7px;
  border-radius: var(--radius-pill);
}

.badge-green {
  color: var(--success);
  background: var(--success-soft);
}

.env-card-agent-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
}

.env-card-agent-icon {
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

.env-card-agent-info {
  flex: 1;
  min-width: 0;
}

.env-card-agent-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.env-card-no-agent {
  font-size: 12px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  padding: 4px 0;
}

/* Card Actions */
.env-card-actions {
  display: flex;
  gap: 6px;
  padding: 12px 16px;
  border-top: 1px solid var(--border);
}

.env-card-action {
  flex: 1;
  height: 30px;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  border: 1px solid var(--border-strong);
  border-radius: 7px;
  background: var(--bg-surface);
  color: var(--text-primary);
  cursor: pointer;
  font-family: var(--font-sans);
  transition: background var(--transition);
}

.env-card-action:hover {
  background: var(--bg-hover);
}

.env-card-action--primary {
  background: var(--accent);
  color: var(--on-brand);
  border-color: var(--accent);
  font-weight: 600;
}

.env-card-action--primary:hover {
  background: var(--accent-hover);
}

.env-card-action--danger {
  color: var(--danger);
}

.env-card-action--danger:hover {
  background: var(--danger-soft);
}

/* New Environment Card */
.env-card--new {
  border: 1px dashed var(--border-strong);
  display: grid;
  place-items: center;
  color: var(--text-muted);
  cursor: pointer;
  min-height: 220px;
}

.env-card--new:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.env-card--new:hover {
  transform: none;
}

.env-card-new-content {
  text-align: center;
}

.env-card-new-icon {
  font-size: 28px;
}

.env-card-new-text {
  font-size: 13px;
  margin-top: 6px;
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
}

.form-label-text {
  font-size: var(--text-sm);
  color: var(--text-secondary);
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

.delete-msg {
  color: var(--text-secondary);
  margin-bottom: var(--space-4);
  font-size: var(--text-sm);
}

/* ========== Context Menu ========== */
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

/* ========== Responsive ========== */
@media (max-width: 1100px) {
  .env-grid {
    grid-template-columns: 1fr 1fr;
  }
}

@media (max-width: 760px) {
  .env-page {
    padding: var(--space-4);
  }
  .env-grid {
    grid-template-columns: 1fr;
  }
}
</style>
