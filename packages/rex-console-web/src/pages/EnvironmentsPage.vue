<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useEnvironmentsStore } from '@/stores/environments'
import { environmentsApi } from '@/api/environments'
import type { ExportData } from '@/api/environments'
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Modal from '@/components/ui/Modal.vue'

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

onMounted(() => {
  store.fetchEnvironments()
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

function agentStatus(status: string | null): StatusDotStatus {
  if (status === 'online') return 'online'
  if (status === 'offline') return 'offline'
  return 'offline'
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
      throw new Error('Invalid config file')
    }
    const result = await environmentsApi.import(data)
    await store.fetchEnvironments()
    alert(`Imported: ${result.imported}, Skipped: ${result.skipped}`)
  } catch (e) {
    console.error('Import failed:', e)
    alert(`Import failed: ${e instanceof Error ? e.message : String(e)}`)
  } finally {
    importLoading.value = false
    input.value = ''
  }
}
</script>

<template>
  <div class="environments">
    <header class="page-header">
      <h1 class="page-title">{{ t('environments.title') }}</h1>
      <div class="header-actions">
        <Button variant="secondary" size="sm" @click="exportConfig">Export</Button>
        <Button variant="secondary" size="sm" :loading="importLoading" @click="triggerImport">Import</Button>
        <input ref="importFileInput" type="file" accept=".json" style="display:none" @change="handleImport" />
        <Button variant="primary" size="sm" @click="openCreate">+ {{ t('environments.newEnvironment') }}</Button>
      </div>
    </header>

    <EmptyState
      v-if="!store.loading && !hasEnvironments"
      icon="⛁"
      :title="t('environments.noEnvironments')"
      :description="t('environments.emptyDescription')"
    >
      <Button variant="primary" @click="openCreate">{{ t('environments.createEnvironment') }}</Button>
    </EmptyState>

    <div v-else class="env-grid">
      <Card
        v-for="env in store.environments"
        :key="env.id"
        class="env-card"
        @click="router.push(`/environments/${env.id}`)"
      >
        <div class="env-card-header">
          <span class="env-icon" :class="env.connection_mode">{{ envIcon(env.connection_mode) }}</span>
          <div class="env-info">
            <div class="env-name">{{ env.name }}</div>
            <div class="env-desc muted">{{ env.description || t('common.noDescription') }}</div>
          </div>
          <div class="env-actions" @click.stop>
            <button class="icon-btn" :title="t('common.edit')" @click="openEdit(env)">✎</button>
            <button class="icon-btn danger" :title="t('common.delete')" @click="deleteConfirmId = env.id">✕</button>
          </div>
        </div>
        <div class="env-agent">
          <template v-if="env.agent_status">
            <StatusDot :status="agentStatus(env.agent_status)" />
            <span class="mono env-agent-status">Agent {{ env.agent_status }}</span>
          </template>
          <template v-else>
            <span class="muted">{{ t('dashboard.noAgent') }}</span>
          </template>
        </div>
        <div class="env-footer">
          <Badge tone="accent">{{ env.resource_count }} {{ t('common.resources') }}</Badge>
          <Badge :tone="env.connection_mode === 'agent' ? 'warning' : 'info'" style="margin-left: 8px">
            {{ env.connection_mode }}
          </Badge>
        </div>
      </Card>
    </div>

    <!-- Create / Edit Modal -->
    <Modal v-model="showCreateModal">
      <template #title>{{ editingEnv ? t('environments.editEnvironment') : t('environments.newEnvironment') }}</template>
      <form class="env-form" @submit.prevent="submitForm">
        <label class="form-label">
          <span>{{ t('common.name') }}</span>
          <input v-model="formName" type="text" class="form-input" placeholder="e.g. Production" autofocus />
        </label>
        <label class="form-label">
          <span>{{ t('common.description') }}</span>
          <input v-model="formDesc" type="text" class="form-input" placeholder="Optional description" />
        </label>
        <label class="form-label">
          <span>{{ t('environments.connectionMode') }}</span>
          <select v-model="formMode" class="form-input">
            <option value="direct">{{ t('environments.direct') }}</option>
            <option value="agent">{{ t('environments.agent') }}</option>
          </select>
        </label>
        <div v-if="formError" class="form-error">{{ formError }}</div>
        <div class="form-actions">
          <Button type="button" variant="secondary" @click="showCreateModal = false">{{ t('common.cancel') }}</Button>
          <Button type="submit" variant="primary" :loading="formLoading">
            {{ editingEnv ? t('common.save') : t('common.create') }}
          </Button>
        </div>
      </form>
    </Modal>

    <!-- Delete Confirmation -->
    <Modal :model-value="!!deleteConfirmId" @update:model-value="deleteConfirmId = null">
      <template #title>{{ t('environments.deleteEnvironment') }}</template>
      <p style="color: var(--text-secondary); margin-bottom: 16px">
        {{ t('environments.deleteConfirm') }}
      </p>
      <div class="form-actions">
        <Button variant="secondary" @click="deleteConfirmId = null">{{ t('common.cancel') }}</Button>
        <Button variant="danger" @click="confirmDelete">{{ t('common.delete') }}</Button>
      </div>
    </Modal>
  </div>
</template>

<style scoped>
.environments {
  max-width: 900px;
}
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-6);
}
.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
.page-title {
  font-size: var(--text-xl);
  font-weight: 600;
  color: var(--text-primary);
}
.env-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: var(--space-4);
}
.env-card {
  cursor: pointer;
  transition: border-color var(--transition);
}
.env-card:hover {
  border-color: var(--accent);
}
.env-card-header {
  display: flex;
  gap: var(--space-3);
  margin-bottom: var(--space-3);
}
.env-icon {
  font-size: 20px;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  flex-shrink: 0;
}
.env-icon.direct {
  color: var(--info);
  background: rgba(88, 166, 255, 0.1);
}
.env-icon.agent {
  color: var(--accent);
  background: rgba(232, 145, 45, 0.1);
}
.env-info {
  flex: 1;
  min-width: 0;
}
.env-name {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
}
.env-desc {
  font-size: var(--text-sm);
  margin-top: var(--space-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.env-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity var(--transition);
}
.env-card:hover .env-actions {
  opacity: 1;
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
  color: var(--text-primary);
}
.icon-btn.danger:hover {
  color: var(--danger);
}
.env-agent {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) 0;
  border-top: 1px solid var(--border);
  margin-top: var(--space-2);
  font-size: var(--text-xs);
}
.env-agent-status {
  color: var(--text-secondary);
}
.env-footer {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  margin-top: var(--space-2);
}
.muted {
  color: var(--text-muted);
}
.mono {
  font-family: var(--font-mono);
}

/* Form styles */
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
