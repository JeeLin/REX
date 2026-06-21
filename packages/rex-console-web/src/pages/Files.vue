<template>
  <div class="files-layout">
    <!-- Top Bar -->
    <div class="files-topbar">
      <button class="btn btn-ghost btn-sm" @click="handleBack">← 返回</button>
      <div class="topbar-spacer"></div>
      <FileBreadcrumb :path="currentPath" @navigate="navigateTo" />
      <div class="topbar-spacer"></div>
      <button class="btn btn-ghost btn-sm" @click="loadFiles()">↻</button>
    </div>

    <!-- Toolbar -->
    <div class="files-toolbar">
      <button class="btn btn-ghost btn-sm" @click="showMkdirDialog = true">📁 新建</button>
      <button class="btn btn-ghost btn-sm" @click="showTouchDialog = true">📄 新建文件</button>
      <div class="toolbar-sep"></div>
      <button class="btn btn-ghost btn-sm" @click="triggerUpload">⬆ 上传</button>
      <button
        class="btn btn-ghost btn-sm"
        :disabled="selectedPaths.length !== 1 || isDirectorySelected"
        @click="handleDownload"
      >
        ⬇ 下载
      </button>
      <div class="toolbar-sep"></div>
      <button
        class="btn btn-ghost btn-sm"
        :disabled="selectedPaths.length === 0"
        @click="handleDelete"
      >
        🗑 删除{{ selectedPaths.length > 0 ? ` (${selectedPaths.length})` : '' }}
      </button>
      <div class="toolbar-spacer"></div>
      <span class="toolbar-info">{{ entries.length }} 项</span>
    </div>

    <!-- Main Content -->
    <div
      class="files-main"
      @dragover.prevent="onDragOver"
      @dragleave.prevent="onDragLeave"
      @drop.prevent="onDrop"
    >
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
      <TransferQueuePanel
        :tasks="transferTasks"
        @cancel="cancelTransfer"
        @remove="removeTransfer"
      />
      <!-- Drag-drop overlay -->
      <div v-if="isDragging" class="drop-overlay">
        <div class="drop-overlay-content">
          <span class="drop-icon">⬆</span>
          <span>拖放文件到此处上传</span>
        </div>
      </div>
    </div>

    <!-- Hidden file input for upload -->
    <input
      ref="fileInput"
      type="file"
      multiple
      style="display: none"
      @change="onFileSelect"
    />

    <!-- Status Bar -->
    <div class="files-statusbar">
      <span>{{ resourceName }}</span>
      <span class="spacer"></span>
      <span v-if="loading">加载中...</span>
      <span v-else-if="error" style="color: var(--danger)">{{ error }}</span>
    </div>

    <!-- Mkdir Dialog -->
    <div v-if="showMkdirDialog" class="modal-overlay" @click.self="showMkdirDialog = false">
      <div class="modal">
        <div class="modal-title">新建文件夹</div>
        <input
          ref="mkdirInput"
          v-model="newDirName"
          class="modal-input"
          placeholder="文件夹名称"
          @keydown.enter="confirmMkdir"
        />
        <div class="modal-actions">
          <button class="btn" @click="showMkdirDialog = false">取消</button>
          <button class="btn btn-primary" @click="confirmMkdir">创建</button>
        </div>
      </div>
    </div>

    <!-- Touch Dialog -->
    <div v-if="showTouchDialog" class="modal-overlay" @click.self="showTouchDialog = false">
      <div class="modal">
        <div class="modal-title">新建文件</div>
        <input
          ref="touchInput"
          v-model="newFileName"
          class="modal-input"
          placeholder="文件名称"
          @keydown.enter="confirmTouch"
        />
        <div class="modal-actions">
          <button class="btn" @click="showTouchDialog = false">取消</button>
          <button class="btn btn-primary" @click="confirmTouch">创建</button>
        </div>
      </div>
    </div>

    <!-- Delete Confirm Dialog -->
    <div v-if="showDeleteDialog" class="modal-overlay" @click.self="showDeleteDialog = false">
      <div class="modal">
        <div class="modal-title">确认删除？</div>
        <p class="modal-desc">将删除 {{ selectedPaths.length }} 个项目，此操作不可撤销。</p>
        <div class="modal-actions">
          <button class="btn" @click="showDeleteDialog = false">取消</button>
          <button class="btn btn-danger" @click="confirmDelete">删除</button>
        </div>
      </div>
    </div>

    <!-- Context Menu -->
    <div
      v-if="showContextMenu"
      class="context-menu"
      :style="{ top: contextMenuY + 'px', left: contextMenuX + 'px' }"
      @click.self="showContextMenu = false"
    >
      <div v-if="contextMenuEntry" class="context-menu-items">
        <div v-if="contextMenuEntry.file_type === 'directory'" class="context-menu-item" @click="enterDirectory(contextMenuEntry.name); showContextMenu = false">
          打开
        </div>
        <div v-if="contextMenuEntry.file_type !== 'directory'" class="context-menu-item" @click="downloadFile(resourceId, contextMenuEntry.path); showContextMenu = false">
          ⬇ 下载
        </div>
        <div class="context-menu-item" @click="handleCopyPath(contextMenuEntry.path); showContextMenu = false">
          复制路径
        </div>
        <div class="context-menu-item" @click="handleRename(contextMenuEntry); showContextMenu = false">
          重命名
        </div>
        <div v-if="sendToTargets.length > 0" class="context-menu-item" @click="handleSendTo(contextMenuEntry); showContextMenu = false">
          发送到…
        </div>
        <div class="context-menu-divider"></div>
        <div class="context-menu-item danger" @click="selectedPaths = [contextMenuEntry.path]; showDeleteDialog = true; showContextMenu = false">
          删除
        </div>
      </div>
      <div v-else class="context-menu-items">
        <div class="context-menu-item" @click="showMkdirDialog = true; showContextMenu = false">
          新建文件夹
        </div>
        <div class="context-menu-item" @click="showTouchDialog = true; showContextMenu = false">
          新建文件
        </div>
        <div class="context-menu-item" @click="triggerUpload(); showContextMenu = false">
          ⬆ 上传文件
        </div>
        <div class="context-menu-divider"></div>
        <div class="context-menu-item" @click="loadFiles(); showContextMenu = false">
          刷新
        </div>
      </div>
    </div>

    <!-- Rename Dialog -->
    <div v-if="showRenameDialog" class="modal-overlay" @click.self="showRenameDialog = false">
      <div class="modal">
        <div class="modal-title">重命名</div>
        <input
          ref="renameInput"
          v-model="renameNewName"
          class="modal-input"
          placeholder="新名称"
          @keydown.enter="confirmRename"
        />
        <div class="modal-actions">
          <button class="btn" @click="showRenameDialog = false">取消</button>
          <button class="btn btn-primary" @click="confirmRename">确认</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useFileManager } from '@/features/files/useFileManager'
import { useTransferQueue } from '@/features/files/useTransferQueue'
import { downloadFile, uploadFile } from '@/api/files'
import { createTransfer } from '@/api/transfer'
import { useTabs } from '@/features/workspace/useTabs'
import FileBreadcrumb from '@/features/files/FileBreadcrumb.vue'
import FileList from '@/features/files/FileList.vue'
import TransferQueuePanel from '@/features/files/TransferQueuePanel.vue'
import type { FileEntry } from '@/api/files'
import type { TransferEndpoint } from '@/api/transfer'

const route = useRoute()
const router = useRouter()
const resourceId = route.params.resourceId as string
const resourceName = ref(resourceId)

const {
  currentPath,
  entries,
  loading,
  error,
  loadFiles,
  navigateTo,
  enterDirectory,
  goUp,
  createDir,
  createFile,
  deleteEntries,
  renameEntry,
} = useFileManager(resourceId)

const selectedPaths = ref<string[]>([])
const isDirectorySelected = computed(() => {
  if (selectedPaths.value.length !== 1) return false
  const entry = entries.value.find(e => e.path === selectedPaths.value[0])
  return entry?.file_type === 'directory'
})

const { tasks: transferTasks, cancel: cancelTransfer, remove: removeTransfer } = useTransferQueue()

// Download
async function handleDownload() {
  if (selectedPaths.value.length !== 1) return
  await downloadFile(resourceId, selectedPaths.value[0])
}

// Upload
const fileInput = ref<HTMLInputElement>()
const uploading = ref(false)

function triggerUpload() {
  fileInput.value?.click()
}

async function uploadFiles(fileList: FileList | File[]) {
  uploading.value = true
  try {
    for (const file of Array.from(fileList)) {
      await uploadFile(resourceId, currentPath.value, file)
    }
    await loadFiles()
  } finally {
    uploading.value = false
  }
}

async function onFileSelect() {
  const input = fileInput.value
  if (!input?.files?.length) return
  await uploadFiles(input.files)
  input.value = ''
}

// Drag-drop
const isDragging = ref(false)
let dragCounter = 0

function onDragOver(e: DragEvent) {
  if (e.dataTransfer?.types.includes('Files')) {
    isDragging.value = true
    dragCounter++
  }
}

function onDragLeave() {
  dragCounter--
  if (dragCounter <= 0) {
    isDragging.value = false
    dragCounter = 0
  }
}

async function onDrop(e: DragEvent) {
  isDragging.value = false
  dragCounter = 0
  const files = e.dataTransfer?.files
  if (!files?.length) return
  await uploadFiles(files)
}

// Send-to (cross-connection transfer)
const { tabs } = useTabs()

const sendToTargets = computed(() => {
  return tabs.value.filter(t =>
    t.id !== route.params.resourceId as string &&
    (t.proto === 'ssh' || t.proto === 'sftp')
  )
})

async function handleSendTo(entry: FileEntry) {
  if (sendToTargets.value.length === 0) return
  // Use first available target (TODO: show target selection dialog)
  const target = sendToTargets.value[0]
  const source: TransferEndpoint = {
    connector_type: 'sftp',
    resource_id: resourceId,
    path: entry.path,
  }
  const dest: TransferEndpoint = {
    connector_type: 'sftp',
    resource_id: target.resourceId,
    path: currentPath.value,
  }
  await createTransfer(source, dest)
}

// Context menu
const showContextMenu = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextMenuEntry = ref<FileEntry | null>(null)

// Dialogs
const showMkdirDialog = ref(false)
const showTouchDialog = ref(false)
const showDeleteDialog = ref(false)
const showRenameDialog = ref(false)
const newDirName = ref('')
const newFileName = ref('')
const renameNewName = ref('')
const renameTarget = ref<FileEntry | null>(null)

const mkdirInput = ref<HTMLInputElement>()
const touchInput = ref<HTMLInputElement>()
const renameInput = ref<HTMLInputElement>()

watch(showMkdirDialog, (v) => { if (v) { newDirName.value = ''; nextTick(() => mkdirInput.value?.focus()) } })
watch(showTouchDialog, (v) => { if (v) { newFileName.value = ''; nextTick(() => touchInput.value?.focus()) } })
watch(showRenameDialog, (v) => { if (v) { nextTick(() => renameInput.value?.focus()) } })

function handleBack() {
  router.back()
}

function handleSelect(entry: FileEntry, event: MouseEvent) {
  if (event.ctrlKey || event.metaKey) {
    const idx = selectedPaths.value.indexOf(entry.path)
    if (idx >= 0) {
      selectedPaths.value.splice(idx, 1)
    } else {
      selectedPaths.value.push(entry.path)
    }
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

function handleCopyPath(path: string) {
  navigator.clipboard.writeText(path)
}

function handleRename(entry: FileEntry) {
  renameTarget.value = entry
  renameNewName.value = entry.name
  showRenameDialog.value = true
}

function handleDelete() {
  if (selectedPaths.value.length > 0) {
    showDeleteDialog.value = true
  }
}

async function confirmMkdir() {
  if (!newDirName.value.trim()) return
  await createDir(newDirName.value.trim())
  showMkdirDialog.value = false
}

async function confirmTouch() {
  if (!newFileName.value.trim()) return
  await createFile(newFileName.value.trim())
  showTouchDialog.value = false
}

async function confirmDelete() {
  const paths = [...selectedPaths.value]
  selectedPaths.value = []
  showDeleteDialog.value = false
  await deleteEntries(paths)
}

async function confirmRename() {
  if (!renameNewName.value.trim() || !renameTarget.value) return
  await renameEntry(renameTarget.value.path, renameNewName.value.trim())
  showRenameDialog.value = false
}

// Close context menu on click outside
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
.files-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg-deep);
}

.files-topbar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-md);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 40px;
  flex-shrink: 0;
  gap: var(--sp-md);
}

.topbar-spacer {
  flex: 1;
}

.files-toolbar {
  display: flex;
  align-items: center;
  padding: var(--sp-xs) var(--sp-md);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 36px;
  flex-shrink: 0;
  gap: var(--sp-sm);
}

.toolbar-sep {
  width: 1px;
  height: 20px;
  background: var(--border);
  margin: 0 var(--sp-xs);
}

.toolbar-spacer {
  flex: 1;
}

.toolbar-info {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.files-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.files-statusbar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-md);
  background: var(--accent);
  color: #000;
  height: 22px;
  flex-shrink: 0;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  font-weight: 500;
  gap: var(--sp-md);
}

.files-statusbar .spacer {
  flex: 1;
}

/* Modal */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: var(--z-modal);
}

.modal {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  max-width: 400px;
  width: 90%;
}

.modal-title {
  font-size: var(--fs-md);
  font-weight: 600;
  margin-bottom: var(--sp-md);
}

.modal-desc {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  margin-bottom: var(--sp-lg);
}

.modal-input {
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

.modal-input:focus {
  border-color: var(--accent);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
}

/* Context Menu */
.context-menu {
  position: fixed;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--sp-xs);
  z-index: var(--z-dropdown, 1000);
  min-width: 160px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.context-menu-items {
  display: flex;
  flex-direction: column;
}

.context-menu-item {
  padding: var(--sp-sm) var(--sp-md);
  border-radius: var(--radius-sm);
  font-size: var(--fs-sm);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.context-menu-item:hover {
  background: var(--bg-hover);
}

.context-menu-item.danger {
  color: var(--danger);
}

.context-menu-divider {
  height: 1px;
  background: var(--border);
  margin: var(--sp-xs) 0;
}

/* Drop overlay */
.files-main {
  position: relative;
}

.drop-overlay {
  position: absolute;
  inset: 0;
  background: rgba(232, 145, 45, 0.08);
  border: 2px dashed var(--accent);
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
  pointer-events: none;
}

.drop-overlay-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--sp-sm);
  color: var(--accent);
  font-size: var(--fs-md);
  font-weight: 600;
}

.drop-icon {
  font-size: 32px;
}
</style>
