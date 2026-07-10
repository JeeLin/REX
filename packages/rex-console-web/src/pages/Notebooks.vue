<template>
  <div class="notebooks-page">
    <!-- Header -->
    <div class="notebooks-header">
      <div class="notebooks-header-left">
        <h1 class="notebooks-title">📓 {{ t('notebooks.title') }}</h1>
        <span class="notebooks-count">{{ notebooks.length }}</span>
      </div>
      <div class="notebooks-header-actions">
        <button class="btn btn-ghost btn-sm" @click="fileInputRef?.click()">
          📥 {{ t('notebooks.import') }}
        </button>
        <input
          ref="fileInputRef"
          type="file"
          accept=".json"
          class="file-input-hidden"
          @change="handleFileImport"
        />
        <button class="btn btn-primary btn-sm" @click="showCreateDialog = true">
          + {{ t('notebooks.create') }}
        </button>
      </div>
    </div>

    <!-- Search -->
    <div class="notebooks-search">
      <input
        v-model="searchQuery"
        type="text"
        :placeholder="t('notebooks.searchPlaceholder')"
        class="search-input"
      />
    </div>

    <!-- List -->
    <div class="notebooks-list">
      <div v-if="loading" class="notebooks-loading">
        <LoadingSpinner />
      </div>
      <div v-else-if="filteredNotebooks.length === 0" class="notebooks-empty">
        <EmptyState :title="t('notebooks.empty')" />
      </div>
      <div
        v-for="notebook in filteredNotebooks"
        :key="notebook.id"
        class="notebook-card"
        @click="openNotebook(notebook.id)"
        @contextmenu.prevent="showContextMenu($event, notebook)"
      >
        <div class="notebook-card-header">
          <h3 class="notebook-card-title">{{ notebook.title }}</h3>
          <div class="notebook-card-actions">
            <button
              class="btn btn-ghost btn-xs"
              :title="t('common.delete')"
              @click.stop="confirmDelete(notebook)"
            >
              🗑
            </button>
          </div>
        </div>
        <p v-if="notebook.description" class="notebook-card-desc">
          {{ notebook.description }}
        </p>
        <div class="notebook-card-meta">
          <span class="notebook-card-time">
            {{ formatDate(notebook.updated_at) }}
          </span>
        </div>
      </div>
    </div>

    <!-- Create Dialog -->
    <ConfirmDialog
      :visible="showCreateDialog"
      :title="t('notebooks.createDialog.title')"
      :message="t('notebooks.createDialog.message')"
      :confirm-label="t('notebooks.createDialog.confirm')"
      @confirm="handleCreate"
      @cancel="showCreateDialog = false"
    >
      <div class="form-group">
        <label>{{ t('notebooks.createDialog.titleLabel') }}</label>
        <input
          v-model="newNotebookTitle"
          type="text"
          class="form-input"
          :placeholder="t('notebooks.createDialog.titlePlaceholder')"
          @keyup.enter="handleCreate"
        />
      </div>
      <div class="form-group">
        <label>{{ t('notebooks.createDialog.descLabel') }}</label>
        <textarea
          v-model="newNotebookDesc"
          class="form-input form-textarea"
          :placeholder="t('notebooks.createDialog.descPlaceholder')"
          rows="2"
        ></textarea>
      </div>
    </ConfirmDialog>

    <!-- Delete Confirm -->
    <ConfirmDialog
      :visible="notebookToDelete !== null"
      :title="t('notebooks.deleteDialog.title')"
      :message="t('notebooks.deleteDialog.message', { title: notebookToDelete?.title ?? '' })"
      :confirm-label="t('notebooks.deleteDialog.confirm')"
      :danger="true"
      @confirm="handleDelete"
      @cancel="notebookToDelete = null"
    />

    <!-- Context Menu -->
    <ContextMenu
      v-if="contextMenu"
      :x="contextMenu.x"
      :y="contextMenu.y"
      :items="contextMenuItems"
      @close="contextMenu = null"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, useTemplateRef } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { listNotebooks, createNotebook, deleteNotebook, importNotebook } from '../api/notebook'
import type { Notebook } from '../api/notebook'
import { importNotebookFromFile, exportNotebookToFile } from '../utils/notebook-io'
import { useToast } from '../composables/useToast'
import LoadingSpinner from '../components/LoadingSpinner.vue'
import EmptyState from '../components/EmptyState.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import ContextMenu from '../components/ContextMenu.vue'

const { t } = useI18n()
const router = useRouter()
const { success: toastSuccess, error: toastError } = useToast()
const fileInputRef = useTemplateRef<HTMLInputElement>('fileInputRef')

const notebooks = ref<Notebook[]>([])
const loading = ref(true)
const searchQuery = ref('')
const showCreateDialog = ref(false)
const newNotebookTitle = ref('')
const newNotebookDesc = ref('')
const notebookToDelete = ref<Notebook | null>(null)
const contextMenu = ref<{ x: number; y: number; notebook: Notebook } | null>(null)

const filteredNotebooks = computed(() => {
  if (!searchQuery.value) return notebooks.value
  const q = searchQuery.value.toLowerCase()
  return notebooks.value.filter(
    (n) =>
      n.title.toLowerCase().includes(q) ||
      (n.description && n.description.toLowerCase().includes(q))
  )
})

const contextMenuItems = computed(() => {
  if (!contextMenu.value) return []
  const notebook = contextMenu.value.notebook
  return [
    { label: t('notebooks.contextOpen'), action: () => openNotebook(notebook.id) },
    { label: t('notebooks.contextRename'), action: () => startRename(notebook) },
    { label: t('notebooks.contextDelete'), action: () => confirmDelete(notebook), danger: true },
    { label: t('notebooks.contextExport'), action: () => handleExport(notebook) },
  ]
})

onMounted(async () => {
  await loadNotebooks()
})

async function loadNotebooks() {
  loading.value = true
  try {
    notebooks.value = await listNotebooks()
  } catch (e) {
    console.error('Failed to load notebooks:', e)
  } finally {
    loading.value = false
  }
}

function openNotebook(id: string) {
  router.push(`/notebooks/${id}`)
}

async function handleCreate() {
  if (!newNotebookTitle.value.trim()) return
  try {
    const notebook = await createNotebook({
      title: newNotebookTitle.value.trim(),
      description: newNotebookDesc.value.trim() || undefined,
    })
    notebooks.value.unshift(notebook)
    showCreateDialog.value = false
    newNotebookTitle.value = ''
    newNotebookDesc.value = ''
  } catch (e) {
    console.error('Failed to create notebook:', e)
  }
}

function confirmDelete(notebook: Notebook) {
  notebookToDelete.value = notebook
  contextMenu.value = null
}

async function handleDelete() {
  if (!notebookToDelete.value) return
  try {
    await deleteNotebook(notebookToDelete.value.id)
    notebooks.value = notebooks.value.filter((n) => n.id !== notebookToDelete.value!.id)
    notebookToDelete.value = null
  } catch (e) {
    console.error('Failed to delete notebook:', e)
  }
}

function startRename(notebook: Notebook) {
  // TODO: implement inline rename
  contextMenu.value = null
}

async function handleExport(notebook: Notebook) {
  contextMenu.value = null
  try {
    await exportNotebookToFile(notebook.id, notebook.title)
    toastSuccess(t('notebooks.editor.exportSuccess'))
  } catch {
    toastError(t('notebooks.editor.exportError'))
  }
}

async function handleFileImport(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  input.value = ''
  try {
    const data = await importNotebookFromFile(file)
    const notebook = await importNotebook({
      title: data.title,
      description: data.description || undefined,
      blocks: data.blocks,
    })
    notebooks.value.unshift(notebook)
    toastSuccess(t('notebooks.importSuccess'))
    router.push(`/notebooks/${notebook.id}`)
  } catch (err) {
    toastError(err instanceof Error ? err.message : t('notebooks.importError'))
  }
}

function showContextMenu(event: MouseEvent, notebook: Notebook) {
  contextMenu.value = { x: event.clientX, y: event.clientY, notebook }
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr)
  return d.toLocaleDateString()
}
</script>

<style scoped>
.notebooks-page {
  padding: 24px;
  max-width: 960px;
  margin: 0 auto;
}

.notebooks-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.notebooks-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.notebooks-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.file-input-hidden {
  display: none;
}

.notebooks-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.notebooks-count {
  font-size: 13px;
  color: var(--text-secondary);
}

.notebooks-search {
  margin-bottom: 16px;
}

.search-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 14px;
}

.notebooks-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.notebooks-loading,
.notebooks-empty {
  padding: 48px 0;
  text-align: center;
}

.notebook-card {
  padding: 12px 16px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  cursor: pointer;
  transition: border-color 0.15s;
}

.notebook-card:hover {
  border-color: var(--accent);
}

.notebook-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.notebook-card-title {
  font-size: 15px;
  font-weight: 500;
  margin: 0;
}

.notebook-card-actions {
  opacity: 0;
  transition: opacity 0.15s;
}

.notebook-card:hover .notebook-card-actions {
  opacity: 1;
}

.notebook-card-desc {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 4px 0 0;
  line-height: 1.4;
}

.notebook-card-meta {
  margin-top: 6px;
}

.notebook-card-time {
  font-size: 12px;
  color: var(--text-tertiary);
}

.form-group {
  margin-bottom: 12px;
}

.form-group label {
  display: block;
  font-size: 13px;
  font-weight: 500;
  margin-bottom: 4px;
}

.form-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 14px;
  box-sizing: border-box;
}

.form-textarea {
  resize: vertical;
  min-height: 60px;
}
</style>
