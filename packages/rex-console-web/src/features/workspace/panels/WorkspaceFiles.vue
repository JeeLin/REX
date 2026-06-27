<template>
  <div class="ws-files">
    <!-- Top Bar -->
    <div class="ws-files-topbar">
      <FileBreadcrumb :path="currentPath" @navigate="navigateTo" />
      <div class="ws-files-topbar-spacer"></div>
      <button class="btn btn-ghost btn-xs" @click="loadFiles()">↻</button>
    </div>

    <!-- Toolbar -->
    <div class="ws-files-toolbar">
      <button class="btn btn-ghost btn-xs" @click="showMkdirDialog = true">📁 新建</button>
      <button class="btn btn-ghost btn-xs" @click="showTouchDialog = true">📄 新建文件</button>
      <div class="ws-files-sep"></div>
      <button
        class="btn btn-ghost btn-xs"
        :disabled="selectedPaths.length === 0"
        @click="handleDelete"
      >
        🗑 删除{{ selectedPaths.length > 0 ? ` (${selectedPaths.length})` : '' }}
      </button>
      <div class="ws-files-spacer"></div>
      <span class="ws-files-info">{{ entries.length }} 项</span>
    </div>

    <!-- Main Content -->
    <div class="ws-files-main">
      <FileList
        :entries="entries"
        :current-path="currentPath"
        :selected-paths="selectedPaths"
        :loading="loading"
        @go-up="goUp"
        @open="enterDirectory"
        @select="handleSelect"
        @context-menu="handleContextMenu"
      />
    </div>

    <!-- Status Bar -->
    <div class="ws-files-statusbar">
      <span>{{ resourceName }}</span>
      <span class="spacer"></span>
      <span v-if="loading">加载中...</span>
      <span v-else-if="error" style="color: #000">{{ error }}</span>
    </div>

    <!-- Mkdir Dialog -->
    <div v-if="showMkdirDialog" class="ws-files-modal-overlay" @click.self="showMkdirDialog = false">
      <div class="ws-files-modal">
        <div class="ws-files-modal-title">新建文件夹</div>
        <input
          ref="mkdirInput"
          v-model="newDirName"
          class="ws-files-modal-input"
          placeholder="文件夹名称"
          @keydown.enter="confirmMkdir"
        />
        <div class="ws-files-modal-actions">
          <button class="btn" @click="showMkdirDialog = false">取消</button>
          <button class="btn btn-primary" @click="confirmMkdir">创建</button>
        </div>
      </div>
    </div>

    <!-- Touch Dialog -->
    <div v-if="showTouchDialog" class="ws-files-modal-overlay" @click.self="showTouchDialog = false">
      <div class="ws-files-modal">
        <div class="ws-files-modal-title">新建文件</div>
        <input
          ref="touchInput"
          v-model="newFileName"
          class="ws-files-modal-input"
          placeholder="文件名称"
          @keydown.enter="confirmTouch"
        />
        <div class="ws-files-modal-actions">
          <button class="btn" @click="showTouchDialog = false">取消</button>
          <button class="btn btn-primary" @click="confirmTouch">创建</button>
        </div>
      </div>
    </div>

    <!-- Delete Confirm Dialog -->
    <ConfirmDialog
      :visible="showDeleteDialog"
      :title="t('files.deleteConfirm')"
      :message="t('files.deleteDesc', { count: selectedPaths.length })"
      :confirm-label="t('files.deleteBtn')"
      :danger="true"
      @confirm="confirmDelete"
      @cancel="showDeleteDialog = false"
    />

    <!-- Context Menu -->
    <div
      v-if="showContextMenu"
      class="ws-files-context-menu"
      :style="{ top: contextMenuY + 'px', left: contextMenuX + 'px' }"
    >
      <div v-if="contextMenuEntry" class="ws-files-context-items">
        <div
          v-if="contextMenuEntry.file_type === 'directory'" class="ws-files-context-item"
          @click="enterDirectory(contextMenuEntry.name); showContextMenu = false"
        >
          打开
        </div>
        <div
          class="ws-files-context-item"
          @click="handleCopyPath(contextMenuEntry.path); showContextMenu = false"
        >
          复制路径
        </div>
        <div class="ws-files-context-divider"></div>
        <div
          class="ws-files-context-item danger"
          @click="selectedPaths = [contextMenuEntry.path]; showDeleteDialog = true; showContextMenu = false"
        >
          删除
        </div>
      </div>
      <div v-else class="ws-files-context-items">
        <div class="ws-files-context-item" @click="showMkdirDialog = true; showContextMenu = false">
          新建文件夹
        </div>
        <div class="ws-files-context-item" @click="showTouchDialog = true; showContextMenu = false">
          新建文件
        </div>
        <div class="ws-files-context-divider"></div>
        <div class="ws-files-context-item" @click="loadFiles(); showContextMenu = false">
          刷新
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useFileManager } from '@/features/files/useFileManager'
import { useToast } from '@/composables/useToast'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import FileBreadcrumb from '@/features/files/FileBreadcrumb.vue'
import FileList from '@/features/files/FileList.vue'
import type { FileEntry } from '@/api/files'

const { t } = useI18n()
const { success } = useToast()

const props = defineProps<{
  resourceId: string
  resourceName: string
}>()

defineEmits<{
  (e: 'disconnect'): void
  (e: 'error', msg: string): void
}>()

const {
  currentPath, entries, loading, error,
  loadFiles, navigateTo, enterDirectory, goUp,
  createDir, createFile, deleteEntries,
} = useFileManager(props.resourceId)

const selectedPaths = ref<string[]>([])

// Context menu
const showContextMenu = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextMenuEntry = ref<FileEntry | null>(null)

// Dialogs
const showMkdirDialog = ref(false)
const showTouchDialog = ref(false)
const showDeleteDialog = ref(false)
const newDirName = ref('')
const newFileName = ref('')

const mkdirInput = ref<HTMLInputElement>()
const touchInput = ref<HTMLInputElement>()

watch(showMkdirDialog, (v) => { if (v) { newDirName.value = ''; nextTick(() => mkdirInput.value?.focus()) } })
watch(showTouchDialog, (v) => { if (v) { newFileName.value = ''; nextTick(() => touchInput.value?.focus()) } })

function handleSelect(entry: FileEntry, event: MouseEvent) {
  if (event.ctrlKey || event.metaKey) {
    const idx = selectedPaths.value.indexOf(entry.path)
    if (idx >= 0) selectedPaths.value.splice(idx, 1)
    else selectedPaths.value.push(entry.path)
  } else {
    selectedPaths.value = [entry.path]
  }
}

function handleContextMenu(event: MouseEvent, entry: FileEntry | null) {
  contextMenuX.value = event.clientX
  contextMenuY.value = event.clientY
  contextMenuEntry.value = entry
  showContextMenu.value = true
  if (entry && !selectedPaths.value.includes(entry.path)) {
    selectedPaths.value = [entry.path]
  }
}

function handleDelete() {
  if (selectedPaths.value.length > 0) showDeleteDialog.value = true
}

function handleCopyPath(path: string) {
  navigator.clipboard.writeText(path)
}

async function confirmMkdir() {
  if (!newDirName.value.trim()) return
  await createDir(newDirName.value.trim())
  showMkdirDialog.value = false
  success(t('files.folderCreated'))
}

async function confirmTouch() {
  if (!newFileName.value.trim()) return
  await createFile(newFileName.value.trim())
  showTouchDialog.value = false
  success(t('files.fileCreated'))
}

async function confirmDelete() {
  const paths = [...selectedPaths.value]
  selectedPaths.value = []
  showDeleteDialog.value = false
  await deleteEntries(paths)
  success(t('files.deleted'))
}

function closeContextMenu() {
  showContextMenu.value = false
}

onMounted(async () => {
  document.addEventListener('click', closeContextMenu)
  await loadFiles()
})

onBeforeUnmount(() => {
  document.removeEventListener('click', closeContextMenu)
})
</script>

<style scoped>
.ws-files {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-deep);
  position: relative;
}

.ws-files-topbar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-sm);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 32px;
  flex-shrink: 0;
  gap: var(--sp-sm);
}

.ws-files-topbar-spacer { flex: 1; }

.ws-files-toolbar {
  display: flex;
  align-items: center;
  padding: 2px var(--sp-sm);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 28px;
  flex-shrink: 0;
  gap: var(--sp-xs);
}

.ws-files-sep {
  width: 1px;
  height: 16px;
  background: var(--border);
}

.ws-files-spacer { flex: 1; }

.ws-files-info {
  font-size: 11px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.ws-files-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
}

.ws-files-statusbar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-sm);
  background: var(--accent);
  color: #000;
  height: 22px;
  flex-shrink: 0;
  font-size: 11px;
  font-family: var(--font-mono);
  font-weight: 500;
  gap: var(--sp-sm);
}

.ws-files-statusbar .spacer { flex: 1; }

/* Modal */
.ws-files-modal-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20;
}

.ws-files-modal {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  max-width: 320px;
  width: 90%;
}

.ws-files-modal-title {
  font-size: var(--fs-md);
  font-weight: 600;
  margin-bottom: var(--sp-md);
}

.ws-files-modal-desc {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  margin-bottom: var(--sp-lg);
}

.ws-files-modal-input {
  width: 100%;
  padding: var(--sp-sm) var(--sp-md);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  outline: none;
  margin-bottom: var(--sp-lg);
}

.ws-files-modal-input:focus {
  border-color: var(--accent);
}

.ws-files-modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
}

/* Context Menu */
.ws-files-context-menu {
  position: fixed;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--sp-xs);
  z-index: 1000;
  min-width: 140px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.ws-files-context-items {
  display: flex;
  flex-direction: column;
}

.ws-files-context-item {
  padding: var(--sp-xs) var(--sp-md);
  border-radius: var(--radius-sm);
  font-size: var(--fs-sm);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.ws-files-context-item:hover {
  background: var(--bg-hover);
}

.ws-files-context-item.danger {
  color: var(--danger);
}

.ws-files-context-divider {
  height: 1px;
  background: var(--border);
  margin: var(--sp-xs) 0;
}
</style>
