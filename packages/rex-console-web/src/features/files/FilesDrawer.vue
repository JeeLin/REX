<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import { onClickOutside } from '@vueuse/core'
import * as filesApi from '@/api/files'
import type { FileEntry } from '@/api/files'

const props = defineProps<{
  resourceId?: string
  host?: string
  port?: number
  username?: string
  password?: string
}>()

// Connection
const sessionId = ref<string | null>(null)
const connected = ref(false)
const loading = ref(false)
const error = ref('')

// Current directory
const currentPath = ref('/')
const entries = ref<FileEntry[]>([])
const selected = ref(new Set<string>())

// Transfer queue
interface TransferItem {
  id: string
  fileName: string
  direction: 'upload' | 'download'
  progress: number
  speed: number
  totalSize: number
  transferred: number
  status: 'pending' | 'transferring' | 'completed' | 'error' | 'cancelled'
  xhr?: XMLHttpRequest
}
const transfers = ref<TransferItem[]>([])
const showTransfers = ref(true)

// Inline rename
const renaming = ref<{ name: string; value: string } | null>(null)

// New folder
const newFolderName = ref('')
const showNewFolder = ref(false)

onMounted(async () => {
  if (props.host) await doConnect()
})

watch(() => props.host, async () => {
  if (props.host && !connected.value) await doConnect()
})

async function doConnect() {
  if (!props.host) return
  loading.value = true; error.value = ''
  try {
    sessionId.value = await filesApi.connect({
      protocol: 'sftp', host: props.host, port: props.port || 22,
      username: props.username, password: props.password,
    })
    connected.value = true
    await loadDir()
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally { loading.value = false }
}

async function loadDir() {
  if (!sessionId.value) return
  loading.value = true
  try { entries.value = await filesApi.listFiles(sessionId.value, currentPath.value) }
  catch { entries.value = [] } finally { loading.value = false }
}

function navigate(entry: FileEntry) {
  if (entry.is_dir) {
    currentPath.value = entry.path.endsWith('/') ? entry.path : entry.path + '/'
    selected.value.clear(); loadDir()
  }
}

function goUp() {
  const parts = currentPath.value.replace(/\/$/, '').split('/'); parts.pop()
  currentPath.value = parts.length ? parts.join('/') + '/' : '/'
  selected.value.clear(); loadDir()
}

function select(name: string, e: MouseEvent) {
  if (e.shiftKey && selected.value.size > 0) {
    const last = Array.from(selected.value).pop()!
    const si = entries.value.findIndex(x => x.name === last)
    const ei = entries.value.findIndex(x => x.name === name)
    const [a, b] = si < ei ? [si, ei] : [ei, si]
    for (let i = a; i <= b; i++) selected.value.add(entries.value[i]!.name)
  } else if (e.ctrlKey || e.metaKey) {
    selected.value.has(name) ? selected.value.delete(name) : selected.value.add(name)
  } else {
    selected.value.clear(); selected.value.add(name)
  }
  selected.value = new Set(selected.value)
}

// Upload with progress
function upload() {
  if (!sessionId.value) return
  const input = document.createElement('input')
  input.type = 'file'; input.multiple = true
  input.onchange = () => {
    for (const file of Array.from(input.files || [])) {
      const id = `tr-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
      const item: TransferItem = {
        id, fileName: file.name, direction: 'upload', progress: 0, speed: 0,
        totalSize: file.size, transferred: 0, status: 'transferring',
      }
      transfers.value.push(item)
      let lastLoaded = 0, lastTime = Date.now()
      filesApi.uploadFileWithProgress(sessionId.value!, currentPath.value + file.name, file,
        (pct, loaded) => {
          item.progress = pct; item.transferred = loaded
          const now = Date.now(), dt = (now - lastTime) / 1000
          if (dt >= 0.5) {
            item.speed = Math.round((loaded - lastLoaded) / dt)
            lastLoaded = loaded; lastTime = now
          }
        }).then(() => {
          item.status = 'completed'; item.progress = 100
          setTimeout(() => { transfers.value = transfers.value.filter(t => t.id !== id) }, 30000)
          loadDir()
        }).catch(() => { item.status = 'error' })
    }
  }
  input.click()
}

// Download
function download(entry: FileEntry) {
  if (!sessionId.value || entry.is_dir) return
  const id = `tr-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
  const item: TransferItem = {
    id, fileName: entry.name, direction: 'download', progress: -1, speed: 0,
    totalSize: entry.size, transferred: 0, status: 'transferring',
  }
  transfers.value.push(item)
  filesApi.downloadFile(sessionId.value, entry.path).then(blob => {
    item.status = 'completed'; item.progress = 100
    setTimeout(() => { transfers.value = transfers.value.filter(t => t.id !== id) }, 30000)
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a'); a.href = url; a.download = entry.name; a.click()
    URL.revokeObjectURL(url)
  }).catch(() => { item.status = 'error' })
}

function cancelTransfer(item: TransferItem) {
  item.xhr?.abort()
  item.status = 'cancelled'
  transfers.value = transfers.value.filter(t => t.id !== item.id)
}

// Delete
async function deleteSelected() {
  if (!sessionId.value) return
  for (const name of selected.value) {
    const entry = entries.value.find(e => e.name === name)
    if (entry) await filesApi.deleteFile(sessionId.value, entry.path)
  }
  selected.value.clear(); loadDir()
}

// Rename
function startRename(entry: FileEntry) {
  renaming.value = { name: entry.name, value: entry.name }
  ctx.value.show = false
}
async function finishRename() {
  if (!sessionId.value || !renaming.value) return
  const oldPath = currentPath.value + renaming.value.name
  const newPath = currentPath.value + renaming.value.value
  if (renaming.value.value && oldPath !== newPath) {
    await filesApi.renameFile(sessionId.value, oldPath, newPath)
  }
  renaming.value = null; loadDir()
}

// New folder
function startNewFolder() {
  showNewFolder.value = true; newFolderName.value = ''
  ctx.value.show = false
}
async function createFolder() {
  if (!sessionId.value || !newFolderName.value) return
  await filesApi.mkdir(sessionId.value, currentPath.value + newFolderName.value)
  showNewFolder.value = false; loadDir()
}

// Context menu
const ctx = ref({ show: false, x: 0, y: 0, name: '', path: '', isDir: false })
const ctxRef = ref<HTMLElement | null>(null)
onClickOutside(ctxRef, () => { ctx.value.show = false })

function onCtx(e: MouseEvent, entry: FileEntry) {
  e.preventDefault()
  ctx.value = { show: true, x: e.clientX, y: e.clientY, name: entry.name, path: entry.path, isDir: entry.is_dir }
}
function onBlankCtx(e: MouseEvent) {
  e.preventDefault()
  ctx.value = { show: true, x: e.clientX, y: e.clientY, name: '', path: '', isDir: false }
}
async function ctxDelete() {
  if (sessionId.value && ctx.value.path) await filesApi.deleteFile(sessionId.value, ctx.value.path)
  ctx.value.show = false; loadDir()
}
function ctxCopy() { navigator.clipboard?.writeText(ctx.value.path); ctx.value.show = false }
function ctxRename() {
  const entry = entries.value.find(e => e.name === ctx.value.name)
  if (entry) startRename(entry)
}

// Breadcrumb
const crumbs = ref<{ name: string; path: string }[]>([])
watch(currentPath, (p) => {
  const parts = p.replace(/\/$/, '').split('/').filter(Boolean)
  const result: { name: string; path: string }[] = [{ name: '/', path: '/' }]
  let acc = ''
  for (const part of parts) { acc += '/' + part; result.push({ name: part, path: acc + '/' }) }
  crumbs.value = result
}, { immediate: true })

function goCrumb(path: string) { currentPath.value = path; selected.value.clear(); loadDir() }

function fmtSize(b: number) {
  if (!b) return '-'
  const u = ['B', 'KB', 'MB', 'GB']; let i = 0, s = b
  while (s >= 1024 && i < 3) { s /= 1024; i++ }
  return `${s.toFixed(i ? 1 : 0)} ${u[i]}`
}

function fmtSpeed(b: number) { return fmtSize(b) + '/s' }

onBeforeUnmount(async () => {
  if (sessionId.value) { try { await filesApi.disconnect(sessionId.value) } catch { /* */ } }
})
</script>

<template>
  <div class="fd" @contextmenu.prevent="onBlankCtx">
    <!-- Toolbar -->
    <div class="fd-toolbar">
      <div class="fd-breadcrumb mono">
        <span v-for="(c, i) in crumbs" :key="c.path" class="fd-crumb" @click="goCrumb(c.path)">
          <span v-if="i > 0" class="fd-crumb-sep">/</span>
          <span class="fd-crumb-name">{{ c.name }}</span>
        </span>
      </div>
      <div class="fd-actions">
        <button class="fd-btn" @click="goUp" title="Go up">↑</button>
        <button class="fd-btn" @click="loadDir" title="Refresh">↻</button>
        <button class="fd-btn" @click="upload" title="Upload">⬆</button>
        <span class="fd-count muted">{{ entries.length }} items<template v-if="selected.size"> · {{ selected.size }} sel</template></span>
      </div>
    </div>

    <!-- File list -->
    <div class="fd-list">
      <div v-if="error" class="fd-error">{{ error }}</div>
      <div v-else-if="!connected && loading" class="fd-status muted">Connecting...</div>
      <div v-else-if="loading" class="fd-status muted">Loading...</div>
      <template v-else>
        <div
          v-for="entry in entries" :key="entry.name"
          class="fd-row" :class="{ 'fd-row--sel': selected.has(entry.name) }"
          @click="select(entry.name, $event)" @dblclick="navigate(entry)" @contextmenu="onCtx($event, entry)"
        >
          <span class="fd-icon">{{ entry.is_dir ? '📁' : '📄' }}</span>
          <template v-if="renaming?.name === entry.name">
            <input
              v-model="renaming.value" class="fd-rename mono" autofocus
              @blur="finishRename" @keydown.enter="finishRename" @keydown.escape="renaming = null"
              @click.stop
            />
          </template>
          <template v-else>
            <span class="fd-name">{{ entry.name }}</span>
          </template>
          <span class="fd-size mono muted">{{ entry.is_dir ? '-' : fmtSize(entry.size) }}</span>
        </div>
        <!-- New folder input -->
        <div v-if="showNewFolder" class="fd-row fd-row--new">
          <span class="fd-icon">📁</span>
          <input
            v-model="newFolderName" class="fd-rename mono" placeholder="Folder name" autofocus
            @blur="showNewFolder = false" @keydown.enter="createFolder" @keydown.escape="showNewFolder = false"
          />
        </div>
        <div v-if="!entries.length && !showNewFolder" class="fd-status muted">Empty</div>
      </template>
    </div>

    <!-- Transfer queue -->
    <div v-if="transfers.length" class="fd-transfers">
      <div class="fd-transfer-header" @click="showTransfers = !showTransfers">
        <span>Transfers ({{ transfers.length }})</span>
        <span class="fd-transfer-toggle">{{ showTransfers ? '▾' : '▸' }}</span>
      </div>
      <div v-if="showTransfers" class="fd-transfer-list">
        <div v-for="t in transfers" :key="t.id" class="fd-transfer-item">
          <span class="fd-transfer-dir">{{ t.direction === 'upload' ? '⬆' : '⬇' }}</span>
          <span class="fd-transfer-name">{{ t.fileName }}</span>
          <div v-if="t.status === 'transferring'" class="fd-transfer-bar">
            <div v-if="t.progress >= 0" class="fd-transfer-fill" :style="{ width: t.progress + '%' }" />
            <div v-else class="fd-transfer-fill fd-transfer-fill--indeterminate" />
          </div>
          <span v-if="t.status === 'completed'" class="fd-transfer-status fd-transfer-status--ok">✓</span>
          <span v-else-if="t.status === 'error'" class="fd-transfer-status fd-transfer-status--err">✗</span>
          <span v-else-if="t.speed > 0" class="fd-transfer-speed mono muted">{{ fmtSpeed(t.speed) }}</span>
          <button v-if="t.status === 'transferring'" class="fd-transfer-cancel" @click="cancelTransfer(t)">×</button>
        </div>
      </div>
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <div v-if="ctx.show" class="fd-ctx-overlay" @click="ctx.show = false" @contextmenu.prevent="ctx.show = false" />
      <div v-if="ctx.show" ref="ctxRef" class="fd-ctx" :style="{ top: ctx.y + 'px', left: ctx.x + 'px' }">
        <template v-if="ctx.name">
          <div class="fd-ctx-item" @click="() => { const e = entries.find(x => x.name === ctx.name); if (e) navigate(e) }">Open</div>
          <div class="fd-ctx-item" @click="ctxRename">Rename</div>
          <div v-if="!ctx.isDir" class="fd-ctx-item" @click="() => { const e = entries.find(x => x.name === ctx.name); if (e) download(e); ctx.show = false }">Download</div>
          <div class="fd-ctx-item" @click="ctxDelete">Delete</div>
          <div class="fd-ctx-sep" />
        </template>
        <div class="fd-ctx-item" @click="startNewFolder">New Folder</div>
        <div class="fd-ctx-item" @click="upload">Upload Here</div>
        <div class="fd-ctx-sep" />
        <div class="fd-ctx-item" @click="ctxCopy">Copy Path</div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.fd { display: flex; flex-direction: column; height: 100%; overflow: hidden; }

.fd-toolbar {
  display: flex; align-items: center; justify-content: space-between;
  padding: 0 var(--space-2); height: 28px; flex-shrink: 0;
  border-bottom: 1px solid var(--border); background: var(--bg-surface);
}
.fd-breadcrumb { display: flex; align-items: center; font-size: var(--text-xs); overflow: hidden; }
.fd-crumb { cursor: pointer; white-space: nowrap; }
.fd-crumb:hover .fd-crumb-name { color: var(--accent); }
.fd-crumb-sep { color: var(--text-muted); margin: 0 1px; }
.fd-crumb-name { color: var(--text-secondary); }
.fd-crumb:last-child .fd-crumb-name { color: var(--text-primary); font-weight: 600; }
.fd-actions { display: flex; align-items: center; gap: var(--space-1); flex-shrink: 0; }
.fd-btn {
  background: none; border: none; color: var(--text-muted); cursor: pointer;
  padding: 2px 4px; border-radius: var(--radius-sm); font-size: var(--text-sm);
}
.fd-btn:hover { color: var(--text-primary); }
.fd-count { font-size: var(--text-xs); color: var(--text-muted); margin-left: var(--space-2); }

.fd-list { flex: 1; overflow-y: auto; min-height: 0; }
.fd-row {
  display: flex; align-items: center; gap: var(--space-2);
  padding: 2px var(--space-2); font-size: var(--text-sm); cursor: pointer;
  color: var(--text-secondary); border-left: 2px solid transparent;
}
.fd-row:hover { background: var(--bg-hover); }
.fd-row--sel { background: var(--bg-hover); border-left-color: var(--accent); }
.fd-row--new { border-left-color: var(--accent); cursor: default; }
.fd-icon { font-size: 12px; width: 16px; text-align: center; flex-shrink: 0; }
.fd-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-primary); }
.fd-size { width: 70px; text-align: right; flex-shrink: 0; }
.fd-rename {
  flex: 1; background: var(--bg-deep); border: 1px solid var(--accent); border-radius: 2px;
  color: var(--text-primary); font-size: var(--text-sm); padding: 0 4px; outline: none;
}
.fd-status, .fd-error { padding: var(--space-3); text-align: center; font-size: var(--text-sm); }
.fd-error { color: var(--danger); }

/* Transfer queue */
.fd-transfers { border-top: 1px solid var(--border); flex-shrink: 0; }
.fd-transfer-header {
  display: flex; justify-content: space-between; align-items: center;
  padding: 2px var(--space-2); font-size: var(--text-xs); color: var(--text-muted);
  background: var(--bg-surface); cursor: pointer; user-select: none;
}
.fd-transfer-toggle { font-size: 10px; }
.fd-transfer-list { max-height: 100px; overflow-y: auto; }
.fd-transfer-item {
  display: flex; align-items: center; gap: var(--space-2);
  padding: 2px var(--space-2); font-size: var(--text-xs);
}
.fd-transfer-dir { width: 16px; text-align: center; flex-shrink: 0; }
.fd-transfer-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-secondary); }
.fd-transfer-bar { width: 80px; height: 4px; background: var(--bg-deep); border-radius: 2px; overflow: hidden; flex-shrink: 0; }
.fd-transfer-fill { height: 100%; background: var(--accent); border-radius: 2px; transition: width 0.3s; }
.fd-transfer-fill--indeterminate { width: 40%; animation: fd-slide 1.5s infinite; }
@keyframes fd-slide { 0% { transform: translateX(-100%); } 100% { transform: translateX(350%); } }
.fd-transfer-status { width: 16px; text-align: center; flex-shrink: 0; }
.fd-transfer-status--ok { color: var(--success, #3FB950); }
.fd-transfer-status--err { color: var(--danger); }
.fd-transfer-speed { width: 70px; text-align: right; flex-shrink: 0; }
.fd-transfer-cancel {
  background: none; border: none; color: var(--text-muted); cursor: pointer;
  font-size: var(--text-sm); padding: 0 2px;
}
.fd-transfer-cancel:hover { color: var(--danger); }

/* Context menu */
.fd-ctx-overlay { position: fixed; inset: 0; z-index: 200; }
.fd-ctx {
  position: fixed; z-index: 210; min-width: 160px;
  background: var(--bg-elevated); border: 1px solid var(--border);
  border-radius: var(--radius); box-shadow: var(--shadow); padding: var(--space-1) 0;
}
.fd-ctx-item { padding: var(--space-1) var(--space-3); font-size: var(--text-sm); color: var(--text-primary); cursor: pointer; }
.fd-ctx-item:hover { background: var(--bg-hover); }
.fd-ctx-sep { height: 1px; background: var(--border); margin: var(--space-1) 0; }

.muted { color: var(--text-muted); }
.mono { font-family: var(--font-mono); }
</style>
