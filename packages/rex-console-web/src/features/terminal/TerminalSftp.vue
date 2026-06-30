<template>
  <div class="sftp-panel-inline">
    <!-- Header -->
    <div class="sftp-inline-header">
      <span class="sftp-icon">📁</span>
      <span class="panel-title">SFTP</span>
      <button class="btn btn-ghost btn-sm btn-icon" :title="t('files.sftp.close')" @click="$emit('close')">✕</button>
    </div>

    <!-- Breadcrumb -->
    <div class="sftp-inline-breadcrumb">
      <template v-for="(part, idx) in breadcrumbParts" :key="idx">
        <span
          class="sftp-bc-seg"
          :class="{ 'sftp-bc-current': idx === breadcrumbParts.length - 1 }"
          @click="navigateToBreadcrumb(idx)"
        >{{ part }}</span>
        <span v-if="idx < breadcrumbParts.length - 1" class="sftp-bc-sep">/</span>
      </template>
    </div>

    <!-- Toolbar -->
    <div class="sftp-inline-toolbar">
      <button class="btn btn-ghost btn-sm" @click="triggerUpload">⬆ {{ t('files.upload') }}</button>
      <button
        class="btn btn-ghost btn-sm"
        :disabled="!selectedEntry || selectedEntry.file_type === 'directory'"
        @click="handleDownload"
      >
        ⬇ {{ t('files.download') }}
      </button>
      <button class="btn btn-ghost btn-sm" @click="showMkdirInput = true">📁 {{ t('files.newFolder') }}</button>
      <button class="btn btn-ghost btn-sm" @click="() => loadDir()">↻</button>
    </div>

    <!-- Mkdir input -->
    <div v-if="showMkdirInput" class="sftp-inline-mkdir">
      <input
        ref="mkdirInputRef"
        v-model="newDirName"
        class="sftp-mkdir-input"
        :placeholder="t('files.folderName')"
        @keydown.enter="confirmMkdir"
        @keydown.esc="showMkdirInput = false"
      />
      <button class="btn btn-ghost btn-xs" @click="confirmMkdir">✓</button>
      <button class="btn btn-ghost btn-xs" @click="showMkdirInput = false">✕</button>
    </div>

    <!-- File list -->
    <div class="sftp-inline-files" @contextmenu.prevent="showBgCtxMenu">
      <div v-if="loading" class="sftp-loading">{{ t('files.loading') }}</div>
      <template v-else>
        <div
          v-if="currentPath !== '/'"
          class="sfile-row"
          @click="goUp"
          @dblclick="goUp"
        >
          <span class="sfile-icon folder">📁</span>
          <span class="sfile-name parent">..</span>
          <span class="sfile-size"></span>
        </div>
        <div
          v-for="entry in sortedEntries"
          :key="entry.path"
          class="sfile-row"
          :class="{ selected: selectedEntry?.path === entry.path, renaming: renamingEntry?.path === entry.path }"
          draggable="true"
          @click="onEntryClick(entry)"
          @dblclick="handleDblClick(entry)"
          @contextmenu.prevent.stop="showEntryCtxMenu($event, entry)"
          @dragstart="onDragStart($event, entry)"
        >
          <span class="sfile-icon" :class="getIconClass(entry)">{{ getIcon(entry) }}</span>
          <template v-if="renamingEntry?.path === entry.path">
            <input
              ref="renameInputRef"
              v-model="renameValue"
              class="sftp-rename-input"
              @keydown.enter="confirmRename"
              @keydown.esc="cancelRename"
              @blur="confirmRename"
            />
          </template>
          <template v-else>
            <span class="sfile-name" :class="{ parent: false }">{{ entry.name }}</span>
          </template>
          <span class="sfile-size">{{ entry.file_type === 'directory' ? '' : formatSize(entry.size) }}</span>
        </div>
        <div v-if="entries.length === 0 && !loading" class="sftp-empty">{{ t('files.emptyDir') }}</div>
      </template>
    </div>

    <!-- Status -->
    <div class="sftp-inline-status">
      {{ currentPath }} · {{ t('files.items', { count: entries.length }) }}
    </div>

    <!-- Context Menu -->
    <div
      v-if="ctxMenu.visible"
      class="sftp-ctx-menu"
      :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
    >
      <template v-if="ctxMenu.entry">
        <button v-if="ctxMenu.entry.file_type === 'directory'" class="ctx-menu-item" @click="handleDblClick(ctxMenu.entry); closeCtxMenu()">{{ t('files.open') }}</button>
        <button v-else class="ctx-menu-item" @click="downloadEntry(ctxMenu.entry); closeCtxMenu()">{{ t('files.download') }}</button>
        <button class="ctx-menu-item" @click="copyPath(ctxMenu.entry.path); closeCtxMenu()">{{ t('files.copyPath') }}</button>
        <button class="ctx-menu-item" @click="copyFileName(ctxMenu.entry.name); closeCtxMenu()">{{ t('files.copyFileName') }}</button>
        <button class="ctx-menu-item" @click="startRename(ctxMenu.entry); closeCtxMenu()">{{ t('files.rename') }}</button>
        <button v-if="ctxMenu.entry.file_type !== 'directory' && sendToTargets.length > 0" class="ctx-menu-item" @click="openSendTo(ctxMenu.entry); closeCtxMenu()">{{ t('files.sendTo') }}</button>
        <div class="ctx-menu-divider"></div>
        <button class="ctx-menu-item danger" @click="deleteEntry(ctxMenu.entry); closeCtxMenu()">{{ t('files.delete') }}</button>
      </template>
      <template v-else>
        <button class="ctx-menu-item" @click="showMkdirInput = true; closeCtxMenu()">{{ t('files.newFolder') }}</button>
        <button class="ctx-menu-item" @click="triggerUpload(); closeCtxMenu()">{{ t('files.upload') }}</button>
        <div class="ctx-menu-divider"></div>
        <button class="ctx-menu-item" @click="loadDir(); closeCtxMenu()">{{ t('files.refresh') }}</button>
      </template>
    </div>

    <!-- SendTo modal -->
    <div v-if="showSendTo" class="sftp-sendto-overlay" @click.self="showSendTo = false">
      <div class="sftp-sendto-modal">
        <div class="sftp-sendto-title">{{ t('files.sendToTitle') }}</div>
        <p class="sftp-sendto-desc">{{ t('files.sendToDesc') }}</p>
        <div class="sftp-sendto-list">
          <div
            v-for="target in sendToTargets"
            :key="target.resourceId"
            class="sftp-sendto-item"
            :class="{ active: sendToTargetId === target.resourceId }"
            @click="sendToTargetId = target.resourceId"
          >
            <span class="sftp-sendto-name">{{ target.name }}</span>
            <span class="sftp-sendto-proto">{{ target.proto }}</span>
          </div>
        </div>
        <div class="sftp-sendto-actions">
          <button class="btn btn-ghost btn-sm" @click="showSendTo = false">{{ t('common.cancel') }}</button>
          <button class="btn btn-primary btn-sm" :disabled="!sendToTargetId" @click="confirmSendTo">{{ t('files.send') }}</button>
        </div>
      </div>
    </div>

    <!-- Hidden upload input -->
    <input ref="fileInputRef" type="file" multiple style="display: none" @change="onFileSelect" />

    <!-- Transfer Queue -->
    <TransferQueuePanel
      :tasks="transferTasks"
      :speeds="speeds"
      :etas="etas"
      @cancel="cancelTransferTask"
      @remove="removeTransferTask"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { listFiles, uploadFile, deleteFile as apiDeleteFile, mkdirFile, downloadFile, renameFile } from '@/api/files'
import type { FileEntry } from '@/api/files'
import { createTransfer } from '@/api/transfer'
import type { TransferEndpoint } from '@/api/transfer'
import { useTabs } from '@/features/workspace/useTabs'
import { useToast } from '@/composables/useToast'
import { useTransferQueue } from '@/features/files/useTransferQueue'
import { useTransferToast } from '@/features/files/useTransferToast'
import TransferQueuePanel from '@/features/files/TransferQueuePanel.vue'

const props = defineProps<{
  resourceId: string
}>()

defineEmits<{
  close: []
  dragPath: [path: string]
}>()

const { t } = useI18n()
const { tabs } = useTabs()
const { success: toastSuccess, error: toastError } = useToast()
const { tasks: transferTasks, cancel: cancelTransferTask, remove: removeTransferTask, speeds, etas, prevTasks } = useTransferQueue()

// Toast notifications for transfer completion/failure
useTransferToast(transferTasks, prevTasks)

const currentPath = ref('/')
const entries = ref<FileEntry[]>([])
const loading = ref(false)
const selectedEntry = ref<FileEntry | null>(null)
const showMkdirInput = ref(false)
const newDirName = ref('')
const mkdirInputRef = ref<HTMLInputElement>()
const fileInputRef = ref<HTMLInputElement>()

// Rename state
const renamingEntry = ref<FileEntry | null>(null)
const renameValue = ref('')
const renameInputRef = ref<HTMLInputElement>()

// SendTo state
const showSendTo = ref(false)
const sendToTargetId = ref<string>('')
const sendToFile = ref<FileEntry | null>(null)

const sendToTargets = computed(() => {
  return tabs.value
    .filter(tab => (tab.component === 'files' || tab.component === 'terminal' || tab.component === 's3'))
    .filter(tab => tab.resourceId !== props.resourceId)
    .map(tab => ({ resourceId: tab.resourceId, name: tab.name, proto: tab.proto }))
})

const ctxMenu = ref<{ visible: boolean; x: number; y: number; entry: FileEntry | null }>({
  visible: false, x: 0, y: 0, entry: null,
})

// 移动端检测
const isTouchDevice: boolean = typeof window !== 'undefined' && ('ontouchstart' in window || navigator.maxTouchPoints > 0)

const sortedEntries = computed(() => {
  return [...entries.value].sort((a, b) => {
    if (a.file_type !== b.file_type) return a.file_type === 'directory' ? -1 : 1
    return a.name.localeCompare(b.name)
  })
})

const breadcrumbParts = computed(() => {
  if (currentPath.value === '/') return ['/']
  return currentPath.value.split('/')
})

async function loadDir(path?: string) {
  if (path !== undefined) currentPath.value = path
  loading.value = true
  selectedEntry.value = null
  try {
    const data = await listFiles(props.resourceId, currentPath.value)
    currentPath.value = data.path
    entries.value = data.entries
  } catch {
    entries.value = []
  } finally {
    loading.value = false
  }
}

function navigateToBreadcrumb(idx: number) {
  if (idx === 0) {
    loadDir('/')
  } else {
    const parts = currentPath.value.split('/')
    loadDir(parts.slice(0, idx + 1).join('/'))
  }
}

function goUp() {
  if (currentPath.value === '/') return
  const parent = currentPath.value.replace(/\/[^/]+\/?$/, '') || '/'
  loadDir(parent)
}

function onEntryClick(entry: FileEntry) {
  selectedEntry.value = entry
  // 移动端单击进入目录
  if (isTouchDevice && entry.file_type === 'directory') {
    loadDir(entry.path)
  }
}

function handleDblClick(entry: FileEntry) {
  if (entry.file_type === 'directory') {
    loadDir(entry.path)
  }
}

function handleDownload() {
  if (selectedEntry.value && selectedEntry.value.file_type !== 'directory') {
    downloadEntry(selectedEntry.value)
  }
}

async function downloadEntry(entry: FileEntry) {
  await downloadFile(props.resourceId, entry.path)
}

async function deleteEntry(entry: FileEntry) {
  await apiDeleteFile(props.resourceId, entry.path)
  await loadDir()
}

function triggerUpload() {
  fileInputRef.value?.click()
}

async function onFileSelect() {
  const input = fileInputRef.value
  if (!input?.files?.length) return
  for (const file of Array.from(input.files)) {
    await uploadFile(props.resourceId, currentPath.value, file)
  }
  input.value = ''
  await loadDir()
}

async function confirmMkdir() {
  if (!newDirName.value.trim()) return
  const base = currentPath.value === '/' ? '/' : currentPath.value + '/'
  await mkdirFile(props.resourceId, base + newDirName.value.trim())
  newDirName.value = ''
  showMkdirInput.value = false
  await loadDir()
}

function startRename(entry: FileEntry) {
  renamingEntry.value = entry
  renameValue.value = entry.name
  nextTick(() => {
    const input = renameInputRef.value
    if (input) {
      input.focus()
      // Select name without extension for files
      if (entry.file_type !== 'directory') {
        const dotIdx = entry.name.lastIndexOf('.')
        input.setSelectionRange(0, dotIdx > 0 ? dotIdx : entry.name.length)
      } else {
        input.select()
      }
    }
  })
}

async function confirmRename() {
  const entry = renamingEntry.value
  if (!entry) return
  const newName = renameValue.value.trim()
  if (!newName || newName === entry.name) {
    renamingEntry.value = null
    return
  }
  const basePath = currentPath.value === '/' ? '/' : currentPath.value + '/'
  try {
    await renameFile(props.resourceId, entry.path, basePath + newName)
    await loadDir()
  } catch {
    toastError(t('files.renameFailed'))
  } finally {
    renamingEntry.value = null
  }
}

function cancelRename() {
  renamingEntry.value = null
}

function openSendTo(entry: FileEntry) {
  sendToFile.value = entry
  sendToTargetId.value = ''
  showSendTo.value = true
}

async function confirmSendTo() {
  const entry = sendToFile.value
  const targetId = sendToTargetId.value
  if (!entry || !targetId) return

  const source: TransferEndpoint = { connector_type: 'sftp', resource_id: props.resourceId, path: entry.path }
  const target: TransferEndpoint = { connector_type: 'sftp', resource_id: targetId, path: '/' + entry.name }

  try {
    await createTransfer(source, target)
    toastSuccess(t('files.transferStarted'))
  } catch {
    toastError(t('files.transferFailed'))
  } finally {
    showSendTo.value = false
    sendToFile.value = null
  }
}

function formatSize(bytes: number | null): string {
  if (bytes === null || bytes === undefined) return ''
  if (bytes < 1024) return bytes + 'B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + 'K'
  return (bytes / (1024 * 1024)).toFixed(1) + 'M'
}

function getIcon(entry: FileEntry): string {
  if (entry.file_type === 'directory') return '📁'
  if (/\.(js|ts|py|rs|go|java|c|cpp|h|rb|sh|yaml|yml|json|toml|xml|sql)$/i.test(entry.name)) return '📄'
  if (/\.(png|jpg|jpeg|gif|svg|webp|ico)$/i.test(entry.name)) return '🖼'
  if (/\.(zip|tar|gz|rar|7z|bz2)$/i.test(entry.name)) return '📦'
  return '📄'
}

function getIconClass(entry: FileEntry): string {
  if (entry.file_type === 'directory') return 'folder'
  if (/\.(js|ts|py|rs|go|java|c|cpp|h|rb|sh|yaml|yml|json|toml|xml|sql)$/i.test(entry.name)) return 'code'
  if (/\.(png|jpg|jpeg|gif|svg|webp|ico)$/i.test(entry.name)) return 'image'
  if (/\.(zip|tar|gz|rar|7z|bz2)$/i.test(entry.name)) return 'archive'
  return 'file'
}

function onDragStart(e: DragEvent, entry: FileEntry) {
  e.dataTransfer?.setData('text/plain', entry.path)
}

function showEntryCtxMenu(e: MouseEvent, entry: FileEntry) {
  selectedEntry.value = entry
  ctxMenu.value = { visible: true, x: e.clientX, y: e.clientY, entry }
}

function showBgCtxMenu(e: MouseEvent) {
  ctxMenu.value = { visible: true, x: e.clientX, y: e.clientY, entry: null }
}

function closeCtxMenu() {
  ctxMenu.value.visible = false
}

function copyPath(path: string) {
  navigator.clipboard.writeText(path).catch(() => {})
}

function copyFileName(name: string) {
  navigator.clipboard.writeText(name).catch(() => {})
}

function closeHandler() {
  ctxMenu.value.visible = false
}

watch(showMkdirInput, (v) => {
  if (v) {
    newDirName.value = ''
    nextTick(() => mkdirInputRef.value?.focus())
  }
})

onMounted(() => {
  document.addEventListener('click', closeHandler)
  loadDir()
})

onBeforeUnmount(() => {
  document.removeEventListener('click', closeHandler)
})
</script>

<style scoped>
.sftp-panel-inline {
  width: 400px;
  border-left: 1px solid var(--border);
  background: var(--bg-surface);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  flex-shrink: 0;
}

.sftp-inline-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-sm) var(--sp-md);
  border-bottom: 1px solid var(--border);
  font-size: var(--fs-sm);
  font-weight: 600;
  flex-shrink: 0;
}

.sftp-icon { color: #8B5CF6; }
.panel-title { flex: 1; }

.sftp-inline-breadcrumb {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: var(--sp-xs) var(--sp-md);
  border-bottom: 1px solid var(--border);
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  color: var(--text-muted);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.sftp-bc-seg { cursor: pointer; }
.sftp-bc-seg:hover { color: var(--text-primary); }
.sftp-bc-current { color: var(--text-primary); font-weight: 500; cursor: default; }

.sftp-inline-toolbar {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  padding: var(--sp-xs) var(--sp-md);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.sftp-inline-toolbar .btn { height: 24px; font-size: var(--fs-xs); }

.sftp-inline-mkdir {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  padding: var(--sp-xs) var(--sp-md);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.sftp-mkdir-input {
  flex: 1;
  padding: 2px var(--sp-sm);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  outline: none;
}

.sftp-mkdir-input:focus { border-color: var(--accent); }

.sftp-rename-input {
  flex: 1;
  padding: 2px var(--sp-sm);
  background: var(--bg-deep);
  border: 1px solid var(--accent);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  outline: none;
}

.sftp-inline-files {
  flex: 1;
  overflow-y: auto;
}

.sftp-loading, .sftp-empty {
  padding: var(--sp-xl);
  text-align: center;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.sfile-row {
  display: flex;
  align-items: center;
  padding: 4px var(--sp-md);
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background var(--transition-fast);
  font-size: var(--fs-xs);
}

.sfile-row:hover { background: var(--bg-hover); }
.sfile-row.selected { background: rgba(88,166,255,0.1); }

.sfile-icon { width: 16px; text-align: center; margin-right: var(--sp-sm); flex-shrink: 0; }
.sfile-icon.folder { color: var(--info); }
.sfile-icon.file { color: var(--text-muted); }
.sfile-icon.code { color: var(--success); }
.sfile-icon.image { color: #E879F9; }
.sfile-icon.archive { color: var(--warning); }

.sfile-name { flex: 1; font-family: var(--font-mono); color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.sfile-name.parent { color: var(--text-secondary); }
.sfile-size { width: 60px; text-align: right; color: var(--text-muted); font-family: var(--font-mono); }

.sftp-inline-status {
  padding: var(--sp-xs) var(--sp-md);
  border-top: 1px solid var(--border);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
  flex-shrink: 0;
}

.sftp-ctx-menu {
  position: fixed;
  z-index: 1000;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  padding: 4px 0;
  min-width: 140px;
}

.ctx-menu-item {
  display: block;
  width: 100%;
  padding: 6px 12px;
  font-size: var(--fs-sm);
  color: var(--text-primary);
  background: none;
  border: none;
  text-align: left;
  cursor: pointer;
  white-space: nowrap;
}

.ctx-menu-item:hover { background: var(--bg-hover); }
.ctx-menu-item.danger { color: var(--danger); }

.ctx-menu-divider {
  height: 1px;
  background: var(--border);
  margin: 4px 0;
}

.sftp-sendto-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.sftp-sendto-modal {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  min-width: 320px;
  max-width: 400px;
  width: 90%;
}

.sftp-sendto-title {
  font-size: var(--fs-md);
  font-weight: 600;
  margin-bottom: var(--sp-xs);
}

.sftp-sendto-desc {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  margin-bottom: var(--sp-md);
}

.sftp-sendto-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
  margin-bottom: var(--sp-lg);
  max-height: 200px;
  overflow-y: auto;
}

.sftp-sendto-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-sm) var(--sp-md);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.sftp-sendto-item:hover {
  border-color: var(--accent);
}

.sftp-sendto-item.active {
  background: var(--accent-muted);
  border-color: var(--accent);
}

.sftp-sendto-name {
  font-size: var(--fs-sm);
  color: var(--text-primary);
}

.sftp-sendto-proto {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.sftp-sendto-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
}
</style>
