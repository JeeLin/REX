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

function onContextMenu(e: MouseEvent, env: Environment) {
  e.preventDefault()
  ctxMenu.value = { show: true, x: e.clientX, y: e.clientY, env }
}

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
  <div class="page-container env-page">
    <!-- Header -->
    <header class="page-header">
      <div class="page-header-left">
        <h1 class="page-title mono">{{ t('environments.title') }}</h1>
        <span class="page-subtitle">{{ t('environments.subtitle', 'Manage connection targets') }}</span>
      </div>
      <div class="page-header-actions">
        <Button variant="ghost" size="sm" @click="exportConfig">
          <span class="action-icon">↓</span> {{ t('environments.export') }}
        </Button>
        <Button variant="ghost" size="sm" :loading="importLoading" @click="triggerImport">
          <span class="action-icon">↑</span> {{ t('environments.import') }}
        </Button>
        <input ref="importFileInput" type="file" accept=".json" style="display:none" @change="handleImport" />
        <Button variant="primary" size="sm" @click="openCreate">
          <span class="action-icon">+</span> {{ t('environments.newEnvironment') }}
        </Button>
      </div>
    </header>

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
    <div v-else class="env-grid">
      <button
        v-for="env in store.environments"
        :key="env.id"
        class="env-tile"
        @click="router.push(`/environments/${env.id}`)"
        @contextmenu.prevent="onContextMenu($event, env)"
      >
        <div class="env-tile-top">
          <div class="env-tile-icon" :class="`env-tile-icon--${env.connection_mode}`">
            {{ envIcon(env.connection_mode) }}
          </div>
          <div class="env-tile-actions" @click.stop>
            <button class="env-tile-action" :title="t('common.edit')" @click="openEdit(env)">✎</button>
            <button class="env-tile-action env-tile-action--danger" :title="t('common.delete')" @click="deleteConfirmId = env.id">✕</button>
          </div>
        </div>
        <div class="env-tile-body">
          <span class="env-tile-name mono">{{ env.name }}</span>
          <span class="env-tile-desc muted">{{ env.description || t('common.noDescription') }}</span>
        </div>
        <div class="env-tile-agent">
          <template v-if="env.agent_status">
            <StatusDot :status="agentStatus(env.agent_status)" />
            <span class="mono env-tile-agent-text">Agent {{ env.agent_status }}</span>
          </template>
          <template v-else>
            <span class="muted env-tile-agent-text">{{ t('dashboard.noAgent') }}</span>
          </template>
        </div>
        <div class="env-tile-footer">
          <Badge tone="accent" size="sm">{{ env.resource_count }} {{ t('common.resources') }}</Badge>
          <Badge :tone="env.connection_mode === 'agent' ? 'warning' : 'info'" size="sm">
            {{ env.connection_mode }}
          </Badge>
        </div>
      </button>
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
/* ========== Layout ========== */
.env-page {}
.page-header-actions { display: flex; gap: var(--space-2); align-items: center; }
.action-icon { font-size: var(--text-sm); }

/* ========== Environment Grid ========== */
.env-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: var(--space-3); }

.env-tile {
  display: flex; flex-direction: column; gap: var(--space-3);
  padding: var(--space-4);
  background: var(--bg-surface); border: 1px solid var(--border);
  border-radius: var(--radius-lg); cursor: pointer; text-align: left;
  transition: border-color var(--transition), transform var(--transition);
}
.env-tile:hover { border-color: var(--accent); transform: translateY(-1px); }

.env-tile-top { display: flex; align-items: flex-start; justify-content: space-between; }
.env-tile-icon {
  width: 36px; height: 36px; display: flex; align-items: center; justify-content: center;
  border-radius: var(--radius); font-size: 18px; flex-shrink: 0;
}
.env-tile-icon--direct { color: var(--info); background: var(--info-soft); }
.env-tile-icon--agent { color: var(--accent); background: var(--accent-soft); }

.env-tile-actions { display: flex; gap: 2px; opacity: 0; transition: opacity var(--transition); }
.env-tile:hover .env-tile-actions { opacity: 1; }
.env-tile-action {
  width: 26px; height: 26px; border: none; background: transparent;
  color: var(--text-muted); cursor: pointer; border-radius: var(--radius-sm);
  font-size: 13px; display: flex; align-items: center; justify-content: center;
  transition: background var(--transition), color var(--transition);
}
.env-tile-action:hover { background: var(--bg-hover); color: var(--text-primary); }
.env-tile-action--danger:hover { color: var(--danger); }

.env-tile-body { display: flex; flex-direction: column; gap: var(--space-1); }
.env-tile-name { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); }
.env-tile-desc { font-size: var(--text-xs); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.env-tile-agent {
  display: flex; align-items: center; gap: var(--space-2);
  padding-top: var(--space-2); border-top: 1px solid var(--border-subtle);
  font-size: var(--text-xs);
}
.env-tile-agent-text { color: var(--text-secondary); }

.env-tile-footer { display: flex; gap: var(--space-2); justify-content: flex-end; }

/* ========== Form ========== */
.env-form { display: flex; flex-direction: column; gap: var(--space-4); }
.form-label { display: flex; flex-direction: column; gap: var(--space-1); }
.form-label-text { font-size: var(--text-sm); color: var(--text-secondary); }
.form-error { color: var(--danger); font-size: var(--text-sm); }
.form-actions { display: flex; justify-content: flex-end; gap: var(--space-2); margin-top: var(--space-4); }
.delete-msg { color: var(--text-secondary); margin-bottom: var(--space-4); font-size: var(--text-sm); }

/* ========== Context Menu ========== */
.ctx-item {
  padding: var(--space-2) var(--space-3); font-size: var(--text-sm);
  cursor: pointer; color: var(--text-primary);
}
.ctx-item:hover { background: var(--bg-hover); }
.ctx-item--danger { color: var(--danger); }

/* ========== Responsive ========== */
@media (max-width: 768px) {
  .env-page { padding: var(--space-4); }
  .env-header { flex-direction: column; align-items: flex-start; gap: var(--space-3); }
  .env-header-actions { width: 100%; justify-content: flex-end; }
  .env-grid { grid-template-columns: 1fr; }
}
</style>
