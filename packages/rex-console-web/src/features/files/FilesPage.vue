<script setup lang="ts">
import { ref, reactive, computed, onMounted, onBeforeUnmount } from 'vue'
import { onClickOutside } from '@vueuse/core'
import * as filesApi from '@/api/files'
import type { FileEntry } from '@/api/files'
import FolderSyncDialog from './FolderSyncDialog.vue'
import MobileFilesBar from './MobileFilesBar.vue'

const props = defineProps<{
  resourceId?: string
  protocol?: 'sftp' | 's3'
  host?: string
  port?: number
  username?: string
  password?: string
}>()

// Connection
const sessionId = ref<string | null>(null)
const showConnect = ref(!props.host)
const connProtocol = ref(props.protocol || 'sftp')
const connHost = ref(props.host || '')
const connPort = ref(props.port || 22)
const connUsername = ref(props.username || '')
const connPassword = ref(props.password || '')
const connError = ref('')
const connLoading = ref(false)
// S3 fields
const connBucket = ref('')
const connRegion = ref('')
const connEndpoint = ref('')
const connAccessKey = ref('')
const connSecretKey = ref('')

// Auto-connect on mount if props provided
onMounted(async () => {
  if (props.host) {
    await doConnect()
  }
})

async function doConnect() {
  connLoading.value = true; connError.value = ''
  try {
    const req: Parameters<typeof filesApi.connect>[0] = {
      protocol: connProtocol.value, host: connHost.value, port: connPort.value,
      username: connUsername.value || undefined, password: connPassword.value || undefined,
    }
    if (connProtocol.value === 's3') {
      req.bucket = connBucket.value || undefined
      req.region = connRegion.value || undefined
      req.endpoint = connEndpoint.value || undefined
      req.access_key = connAccessKey.value || undefined
      req.secret_key = connSecretKey.value || undefined
    }
    sessionId.value = await filesApi.connect(req)
    showConnect.value = false
    await loadPanel('left'); await loadPanel('right')
  } catch (e: unknown) { connError.value = e instanceof Error ? e.message : String(e) }
  finally { connLoading.value = false }
}

// Panels
const panels = reactive({
  left: { path: '/', entries: [] as FileEntry[], loading: false, selected: new Set<string>(), active: true },
  right: { path: '/', entries: [] as FileEntry[], loading: false, selected: new Set<string>(), active: false },
})
type Side = 'left' | 'right'
const mobileActiveSide = ref<Side>('left')

async function loadPanel(side: Side) {
  const p = panels[side]; if (!sessionId.value) return
  p.loading = true
  try { p.entries = await filesApi.listFiles(sessionId.value, p.path) }
  catch { p.entries = [] } finally { p.loading = false }
}

// Sync browsing
const syncBrowsing = ref(false)

function navigate(side: Side, entry: FileEntry) {
  if (entry.is_dir) {
    panels[side].path = entry.path.endsWith('/') ? entry.path : entry.path + '/'
    panels[side].selected.clear()
    loadPanel(side)

    // Sync browsing: navigate other panel to relative path
    if (syncBrowsing.value) {
      const otherSide = other(side)
      const currentPath = panels[side].path
      const basePath = panels[side].path.split('/').slice(0, -2).join('/') + '/'
      const relativePath = currentPath.replace(basePath, '')
      if (relativePath && relativePath !== currentPath) {
        const targetPath = panels[otherSide].path + relativePath
        panels[otherSide].path = targetPath
        panels[otherSide].selected.clear()
        loadPanel(otherSide)
      }
    }
  }
}

function activate(side: Side) { panels.left.active = side === 'left'; panels.right.active = side === 'right' }
function other(side: Side): Side { return side === 'left' ? 'right' : 'left' }
function goUp(side: Side) {
  const parts = panels[side].path.replace(/\/$/, '').split('/'); parts.pop()
  panels[side].path = parts.length ? parts.join('/') + '/' : '/'; panels[side].selected.clear(); loadPanel(side)
}
function toggleSelect(side: Side, name: string, e: MouseEvent) {
  const sel = panels[side].selected
  if (e.shiftKey && sel.size > 0) {
    const entries = panels[side].entries; const last = Array.from(sel).pop()!
    const si = entries.findIndex(x => x.name === last), ei = entries.findIndex(x => x.name === name)
    const [a, b] = si < ei ? [si, ei] : [ei, si]; for (let i = a; i <= b; i++) sel.add(entries[i]!.name)
  } else if (e.ctrlKey || e.metaKey) { if (sel.has(name)) sel.delete(name); else sel.add(name) }
  else { sel.clear(); sel.add(name) }
  panels[side].selected = new Set(sel)
}

// Delete confirmation
const showDeleteConfirm = ref(false)
const pendingDelete = ref<{ side: Side; names: string[] } | null>(null)
const pendingCtxDelete = ref(false)

function confirmDelete(side: Side) {
  if (!sessionId.value || panels[side].selected.size === 0) return
  pendingDelete.value = { side, names: Array.from(panels[side].selected) }
  showDeleteConfirm.value = true
}
async function executeDelete() {
  if (!sessionId.value || !pendingDelete.value) return
  if (pendingCtxDelete.value) {
    // Context menu delete
    await filesApi.deleteFile(sessionId.value, ctx.value.path)
    pendingCtxDelete.value = false
    await loadPanel('left'); await loadPanel('right')
  } else {
    // Toolbar/bulk delete
    const { side, names } = pendingDelete.value
    for (const name of names) {
      const entry = panels[side].entries.find(e => e.name === name)
      if (entry) await filesApi.deleteFile(sessionId.value, entry.path)
    }
    panels[side].selected.clear(); loadPanel(side)
  }
  showDeleteConfirm.value = false; pendingDelete.value = null
}
function cancelDelete() { showDeleteConfirm.value = false; pendingDelete.value = null }
function confirmCtxDelete() { pendingCtxDelete.value = true; showDeleteConfirm.value = true; pendingDelete.value = { side: 'left', names: [ctx.value.name] } }

async function deleteSelected(side: Side) {
  confirmDelete(side)
}
async function downloadSelected(side: Side) {
  if (!sessionId.value || panels[side].selected.size !== 1) return
  const name = Array.from(panels[side].selected)[0]; const entry = panels[side].entries.find(e => e.name === name)
  if (!entry || entry.is_dir) return
  const blob = await filesApi.downloadFile(sessionId.value, entry.path)
  const url = URL.createObjectURL(blob); const a = document.createElement('a'); a.href = url; a.download = entry.name; a.click(); URL.revokeObjectURL(url)
}
async function uploadTo(side: Side) {
  if (!sessionId.value) return
  const input = document.createElement('input'); input.type = 'file'; input.multiple = true
  input.onchange = async () => {
    for (const file of Array.from(input.files || [])) await filesApi.uploadFile(sessionId.value!, panels[side].path + file.name, file)
    loadPanel(side)
  }; input.click()
}

// Context menu
const ctx = ref({ show: false, x: 0, y: 0, path: '', name: '' })
const ctxRef = ref<HTMLElement | null>(null)
onClickOutside(ctxRef, () => { ctx.value.show = false })
function onCtx(e: MouseEvent, entry: FileEntry) { e.preventDefault(); ctx.value = { show: true, x: e.clientX, y: e.clientY, path: entry.path, name: entry.name } }
async function ctxDelete() { confirmCtxDelete(); ctx.value.show = false }
function ctxCopy() { navigator.clipboard?.writeText(ctx.value.path); ctx.value.show = false }

// Mobile bar actions
function mfbNewFolder() {
  if (!sessionId.value) return
  const name = prompt('Folder name:')
  if (!name) return
  const side = mobileActiveSide.value
  filesApi.mkdir(sessionId.value, panels[side].path + name).then(() => loadPanel(side))
}
function mfbRename() {
  if (!sessionId.value) return
  const side = mobileActiveSide.value
  const sel = Array.from(panels[side].selected)
  if (sel.length !== 1) return
  const entry = panels[side].entries.find(e => e.name === sel[0])
  if (!entry) return
  const newName = prompt('New name:', entry.name)
  if (!newName || newName === entry.name) return
  filesApi.renameFile(sessionId.value, entry.path, panels[side].path + newName).then(() => loadPanel(side))
}
function mfbDelete() { confirmDelete(mobileActiveSide.value) }
function mfbPermissions() {
  const side = mobileActiveSide.value
  const sel = Array.from(panels[side].selected)
  if (sel.length !== 1) return
  const entry = panels[side].entries.find(e => e.name === sel[0])
  if (entry) openChmod(entry.path)
}
function mfbCopyPath() {
  const side = mobileActiveSide.value
  const sel = Array.from(panels[side].selected)
  if (sel.length !== 1) return
  const entry = panels[side].entries.find(e => e.name === sel[0])
  if (entry) navigator.clipboard?.writeText(entry.path)
}
const mfbSelectedCount = computed(() => panels[mobileActiveSide.value].selected.size)

// Chmod permissions
const showChmod = ref(false)
const chmodPath = ref('')
const chmodPerms = reactive({
  owner: { read: true, write: true, exec: false },
  group: { read: true, write: false, exec: false },
  other: { read: false, write: false, exec: false },
})

function openChmod(path: string) {
  chmodPath.value = path
  showChmod.value = true
}

function calcOctal(): number {
  let octal = 0
  if (chmodPerms.owner.read) octal += 400
  if (chmodPerms.owner.write) octal += 200
  if (chmodPerms.owner.exec) octal += 100
  if (chmodPerms.group.read) octal += 40
  if (chmodPerms.group.write) octal += 20
  if (chmodPerms.group.exec) octal += 10
  if (chmodPerms.other.read) octal += 4
  if (chmodPerms.other.write) octal += 2
  if (chmodPerms.other.exec) octal += 1
  return octal
}

async function applyChmod() {
  if (!sessionId.value) return
  const octal = calcOctal()
  await filesApi.chmod(sessionId.value, chmodPath.value, octal.toString(8))
  showChmod.value = false
  await loadPanel('left')
  await loadPanel('right')
}

// Edit file (temp download → edit → upload back)
async function editFile(path: string) {
  if (!sessionId.value) return
  try {
    // Download file to temp location
    const blob = await filesApi.downloadFile(sessionId.value, path)
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = path.split('/').pop() || 'temp'
    a.click()
    URL.revokeObjectURL(url)
  } catch (e) {
    console.error('Edit failed:', e)
  }
}

// Resize
const leftW = ref(400); const dragging = ref(false); let sx = 0, sw = 0
function onDS(e: MouseEvent) { dragging.value = true; sx = e.clientX; sw = leftW.value; document.addEventListener('mousemove', onDM); document.addEventListener('mouseup', onDE); document.body.style.cursor = 'col-resize'; document.body.style.userSelect = 'none' }
function onDM(e: MouseEvent) { leftW.value = Math.min(800, Math.max(250, sw + (e.clientX - sx))) }
function onDE() { dragging.value = false; document.removeEventListener('mousemove', onDM); document.removeEventListener('mouseup', onDE); document.body.style.cursor = ''; document.body.style.userSelect = '' }
onBeforeUnmount(async () => {
  document.removeEventListener('mousemove', onDM)
  document.removeEventListener('mouseup', onDE)
  if (sessionId.value) {
    try { await filesApi.disconnect(sessionId.value) } catch { /* ignore */ }
  }
})

function fmtSize(b: number) { if (!b) return '-'; const u = ['B','KB','MB','GB']; let i = 0, s = b; while (s >= 1024 && i < 3) { s /= 1024; i++ } return `${s.toFixed(i ? 1 : 0)} ${u[i]}` }

/* ---- drag & drop transfer ---- */
const dragData = ref<{ side: Side; names: string[] } | null>(null)
const dropTarget = ref<Side | null>(null)

function onDragStart(e: DragEvent, side: Side, name: string) {
  // Include all selected items if the dragged one is selected, filter out directories
  const sel = panels[side].selected
  const allNames = sel.has(name) ? Array.from(sel) : [name]
  const names = allNames.filter(n => {
    const entry = panels[side].entries.find(en => en.name === n)
    return entry && !entry.is_dir
  })
  if (names.length === 0) return
  dragData.value = { side, names }
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'copy'
    e.dataTransfer.setData('text/plain', names.join(','))
  }
}

function onDragOver(e: DragEvent, side: Side) {
  e.preventDefault()
  if (dragData.value && dragData.value.side !== side) {
    e.dataTransfer!.dropEffect = 'copy'
    dropTarget.value = side
  }
}

function onDragLeave() {
  dropTarget.value = null
}

async function onDrop(e: DragEvent, targetSide: Side) {
  e.preventDefault()
  dropTarget.value = null
  if (!dragData.value || !sessionId.value) return
  const { side: sourceSide, names } = dragData.value

  for (const name of names) {
    const srcEntry = panels[sourceSide].entries.find(en => en.name === name)
    if (!srcEntry) continue

    if (srcEntry.is_dir) {
      // For directories, we'd need recursive transfer — skip for now
      continue
    }

    const srcPath = srcEntry.path
    const dstPath = panels[targetSide].path + name

    try {
      if (sourceSide === 'left' && targetSide === 'right') {
        // Local → Remote: upload
        const blob = await filesApi.downloadFile(sessionId.value, srcPath)
        await filesApi.uploadFile(sessionId.value, dstPath, new File([blob], name))
      } else if (sourceSide === 'right' && targetSide === 'left') {
        // Remote → Local: download
        const blob = await filesApi.downloadFile(sessionId.value, srcPath)
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url; a.download = name; a.click()
        URL.revokeObjectURL(url)
      } else {
        // Same-side or local→local: download from source, upload to target
        const blob = await filesApi.downloadFile(sessionId.value, srcPath)
        await filesApi.uploadFile(sessionId.value, dstPath, new File([blob], name))
      }
    } catch (err) {
      console.error('Transfer failed:', err)
    }
  }

  dragData.value = null
  loadPanel(targetSide)
}

function onDragEnd() {
  dragData.value = null
  dropTarget.value = null
}

/* ---- folder sync dialog ---- */
const showSyncDialog = ref(false)

function openSyncDialog() {
  showSyncDialog.value = true
}

function onSync(_options: { direction: string; compareSize: boolean; compareTime: boolean; includePattern: string; excludePattern: string; deleteOrphans: boolean }) {
  showSyncDialog.value = false
  // In real implementation: call backend API to perform sync
  loadPanel('left')
  loadPanel('right')
}
</script>

<template>
  <div class="fp" @mousemove.prevent>
    <!-- Mobile panel switcher -->
    <div class="fp-switcher">
      <button class="fp-switcher-btn" :class="{ 'fp-switcher-btn--active': mobileActiveSide === 'left' }" @click="mobileActiveSide = 'left'">Left</button>
      <button class="fp-switcher-btn" :class="{ 'fp-switcher-btn--active': mobileActiveSide === 'right' }" @click="mobileActiveSide = 'right'">Right</button>
    </div>

    <div v-if="showConnect" class="fp-overlay">
      <div class="fp-dialog">
        <h3>Connect to Server</h3>
        <div class="f"><label>Protocol</label><select v-model="connProtocol" class="mono"><option value="sftp">SFTP</option><option value="s3">S3</option></select></div>
        <template v-if="connProtocol==='sftp'">
          <div class="f"><label>Host</label><input v-model="connHost" class="mono" placeholder="192.168.1.1" /></div>
          <div class="f"><label>Port</label><input v-model.number="connPort" class="mono" type="number" /></div>
          <div class="f"><label>User</label><input v-model="connUsername" class="mono" /></div>
          <div class="f"><label>Password</label><input v-model="connPassword" class="mono" type="password" /></div>
        </template>
        <template v-if="connProtocol==='s3'">
          <div class="f"><label>Bucket</label><input v-model="connBucket" class="mono" placeholder="my-bucket" /></div>
          <div class="f"><label>Region</label><input v-model="connRegion" class="mono" placeholder="us-east-1 (optional)" /></div>
          <div class="f"><label>Endpoint URL</label><input v-model="connEndpoint" class="mono" placeholder="https://s3.amazonaws.com (optional)" /></div>
          <div class="f"><label>Access Key</label><input v-model="connAccessKey" class="mono" placeholder="optional for IAM" /></div>
          <div class="f"><label>Secret Key</label><input v-model="connSecretKey" class="mono" type="password" placeholder="optional for IAM" /></div>
        </template>
        <div v-if="connError" class="err">{{ connError }}</div>
        <button class="btn" :disabled="connLoading" @click="doConnect">{{ connLoading ? 'Connecting...' : 'Connect' }}</button>
      </div>
    </div>

    <template v-for="side in (['left','right'] as const)" :key="side">
      <div class="fp-panel" :class="{ 'fp-panel--active': panels[side].active, 'fp-panel--drop': dropTarget === side, 'fp-panel--mobile-hidden': mobileActiveSide !== side }" :style="side==='left' ? {width:leftW+'px'} : {flex:'1',minWidth:'0'}" @click="activate(side)" @dragover="onDragOver($event, side)" @dragleave="onDragLeave" @drop="onDrop($event, side)">
        <div class="ptb">
          <button class="pb" @click="goUp(side)">↑</button>
          <span class="pp mono">{{ panels[side].path }}</span>
          <button class="pb" :class="{ 'pb--active': syncBrowsing }" title="Sync Browsing" @click="syncBrowsing = !syncBrowsing">🔗</button>
          <button class="pb" title="Folder Sync" @click="openSyncDialog">🔄</button>
          <button class="pb" @click="uploadTo(side)">⬆</button>
          <button class="pb" @click="loadPanel(side)">↻</button>
        </div>
        <div class="pf">
          <div class="fr fh"><span class="cn">Name</span><span class="cs">Size</span><span class="cm">Modified</span></div>
          <div v-for="e in panels[side].entries" :key="e.name" class="fr" :class="{ 'fr--sel': panels[side].selected.has(e.name) }" draggable="true" @dragstart="onDragStart($event, side, e.name)" @dragend="onDragEnd" @click="toggleSelect(side, e.name, $event)" @dblclick="navigate(side, e)" @contextmenu="onCtx($event, e)">
            <span class="cn"><span class="fi">{{ e.is_dir ? '📁' : '📄' }}</span> {{ e.name }}</span>
            <span class="cs mu">{{ e.is_dir ? '-' : fmtSize(e.size) }}</span>
            <span class="cm mu">{{ e.modified || '-' }}</span>
          </div>
          <div v-if="!panels[side].loading && !panels[side].entries.length" class="pe">Empty</div>
        </div>
        <div class="ps">{{ panels[side].entries.length }} items<template v-if="panels[side].selected.size"> · {{ panels[side].selected.size }} sel</template></div>
      </div>
      <div v-if="side==='left'" class="fh2" :class="{ 'fh2--a': dragging }" @mousedown.prevent="onDS" />
    </template>

    <div v-if="ctx.show" ref="ctxRef" class="fctx" :style="{top:ctx.y+'px',left:ctx.x+'px'}">
      <div class="ci" @click="editFile(ctx.path)">Edit</div>
      <div class="ci" @click="ctxCopy">Copy Path</div>
      <div class="ci" @click="openChmod(ctx.path)">Permissions</div>
      <div class="ci ci--d" @click="ctxDelete">Delete</div>
    </div>

    <!-- Chmod Modal -->
    <Teleport to="body">
      <div v-if="showChmod" class="fp-overlay" @click.self="showChmod = false">
        <div class="fp-dialog">
          <h3>Permissions: {{ chmodPath }}</h3>
          <div class="chmod-grid">
            <div class="chmod-header">
              <span></span>
              <span>Owner</span>
              <span>Group</span>
              <span>Other</span>
            </div>
            <div class="chmod-row">
              <span>Read</span>
              <input type="checkbox" v-model="chmodPerms.owner.read" />
              <input type="checkbox" v-model="chmodPerms.group.read" />
              <input type="checkbox" v-model="chmodPerms.other.read" />
            </div>
            <div class="chmod-row">
              <span>Write</span>
              <input type="checkbox" v-model="chmodPerms.owner.write" />
              <input type="checkbox" v-model="chmodPerms.group.write" />
              <input type="checkbox" v-model="chmodPerms.other.write" />
            </div>
            <div class="chmod-row">
              <span>Exec</span>
              <input type="checkbox" v-model="chmodPerms.owner.exec" />
              <input type="checkbox" v-model="chmodPerms.group.exec" />
              <input type="checkbox" v-model="chmodPerms.other.exec" />
            </div>
          </div>
          <div class="chmod-octal">Octal: {{ calcOctal().toString(8) }}</div>
          <div style="display:flex;gap:var(--space-2);justify-content:flex-end">
            <button class="btn" @click="showChmod = false">Cancel</button>
            <button class="btn" style="background:var(--accent);color:#fff" @click="applyChmod">Apply</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Folder Sync Dialog -->
    <FolderSyncDialog
      :visible="showSyncDialog"
      :source-path="panels.left.path"
      :target-path="panels.right.path"
      @close="showSyncDialog = false"
      @sync="onSync"
    />

    <!-- Delete Confirmation -->
    <Teleport to="body">
      <div v-if="showDeleteConfirm" class="fp-overlay" @click.self="cancelDelete">
        <div class="fp-dialog">
          <h3>Confirm Delete</h3>
          <p style="color:var(--text-secondary);margin:0">
            Delete {{ pendingDelete?.names.length ?? 0 }} item(s)? This action cannot be undone.
          </p>
          <div style="display:flex;gap:var(--space-2);justify-content:flex-end">
            <button class="btn" style="background:var(--bg-hover);color:var(--text-primary)" @click="cancelDelete">Cancel</button>
            <button class="btn" style="background:var(--danger);color:#fff" @click="executeDelete">Delete</button>
          </div>
        </div>
      </div>
    </Teleport>

    <MobileFilesBar
      :selected-count="mfbSelectedCount"
      @upload="uploadTo(mobileActiveSide)"
      @download="downloadSelected(mobileActiveSide)"
      @new-folder="mfbNewFolder"
      @refresh="loadPanel(mobileActiveSide)"
      @rename="mfbRename"
      @delete="mfbDelete"
      @permissions="mfbPermissions"
      @copy-path="mfbCopyPath"
    />
  </div>
</template>

<style scoped>
.fp{display:flex;height:100%;background:var(--bg-page);position:relative}
.fp-overlay{position:fixed;inset:0;z-index:100;display:flex;align-items:center;justify-content:center;background:rgba(0,0,0,.6)}
.fp-dialog{background:var(--bg-elevated);border:1px solid var(--border);border-radius:var(--radius);padding:var(--space-5);min-width:340px;display:flex;flex-direction:column;gap:var(--space-3)}
.fp-dialog h3{margin:0;color:var(--text-primary)}
.f{display:flex;flex-direction:column;gap:var(--space-1)}
.f label{font-size:var(--text-xs);color:var(--text-muted);text-transform:uppercase}
.f input,.f select{padding:var(--space-2);background:var(--bg-deep);border:1px solid var(--border);border-radius:var(--radius-sm);color:var(--text-primary);font-size:var(--text-sm);outline:none}
.f input:focus,.f select:focus{border-color:var(--accent)}
.err{color:var(--danger);font-size:var(--text-sm)}
.btn{padding:var(--space-2);background:var(--accent);color:#fff;border:none;border-radius:var(--radius-sm);cursor:pointer}
.btn:disabled{opacity:.5}
.fp-panel{display:flex;flex-direction:column;border-right:1px solid var(--border);overflow:hidden;flex-shrink:0}
.fp-panel--active{border-left:2px solid var(--accent)}
.fp-panel--drop{background:rgba(232,145,45,0.08);outline:2px dashed var(--accent);outline-offset:-2px}
.ptb{display:flex;align-items:center;gap:var(--space-1);padding:var(--space-1) var(--space-2);border-bottom:1px solid var(--border);background:var(--bg-surface)}
.pb{background:none;border:none;color:var(--text-muted);cursor:pointer;padding:var(--space-1);border-radius:var(--radius-sm);font-size:var(--text-sm)}
.pb:hover{color:var(--text-primary)}
.pb--active{color:var(--accent);background:rgba(232,145,45,0.1)}
.pp{flex:1;font-size:var(--text-xs);color:var(--text-secondary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.pf{flex:1;overflow-y:auto}
.fr{display:flex;padding:var(--space-1) var(--space-3);font-size:var(--text-sm);cursor:pointer}
.fr:hover{background:var(--bg-hover)}
.fr--sel{background:var(--bg-hover);border-left:2px solid var(--accent)}
.fh{font-weight:600;color:var(--text-muted);font-size:var(--text-xs);text-transform:uppercase;cursor:default}
.fh:hover{background:none}
.cn{flex:1;display:flex;align-items:center;gap:var(--space-2);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.cs{width:80px;text-align:right}
.cm{width:140px;text-align:right}
.fi{font-size:14px}
.mu{color:var(--text-muted)}
.pe{padding:var(--space-4);text-align:center;color:var(--text-muted);font-size:var(--text-sm)}
.ps{padding:var(--space-1) var(--space-3);font-size:var(--text-xs);color:var(--text-muted);border-top:1px solid var(--border);background:var(--bg-surface)}
.fh2{width:4px;cursor:col-resize;background:var(--border);flex-shrink:0}
.fh2:hover,.fh2--a{background:var(--accent)}
.fctx{position:fixed;z-index:200;min-width:160px;background:var(--bg-elevated);border:1px solid var(--border);border-radius:var(--radius);box-shadow:var(--shadow);padding:var(--space-1) 0}
.ci{padding:var(--space-2) var(--space-3);font-size:var(--text-sm);cursor:pointer;color:var(--text-primary)}
.ci:hover{background:var(--bg-hover)}
.ci--d{color:var(--danger)}
.chmod-grid{display:grid;grid-template-columns:auto 1fr 1fr 1fr;gap:var(--space-2);margin:var(--space-3) 0}
.chmod-header{display:contents;font-weight:600;font-size:var(--text-xs);color:var(--text-muted);text-transform:uppercase}
.chmod-header span{text-align:center}
.chmod-row{display:contents}
.chmod-row span{font-size:var(--text-sm);color:var(--text-primary)}
.chmod-row input[type="checkbox"]{margin:0 auto;accent-color:var(--accent)}
.chmod-octal{text-align:center;font-family:var(--font-mono);font-size:var(--text-lg);color:var(--accent);margin:var(--space-3) 0}
.fp-switcher{display:none}
@media(max-width:768px){
  .fp{flex-direction:column}
  .fp-switcher{display:flex;gap:0;border-bottom:1px solid var(--border);background:var(--bg-surface);flex-shrink:0}
  .fp-switcher-btn{flex:1;padding:var(--space-2);background:none;border:none;color:var(--text-muted);font-size:var(--text-sm);cursor:pointer;border-bottom:2px solid transparent}
  .fp-switcher-btn--active{color:var(--accent);border-bottom-color:var(--accent);background:rgba(232,145,45,0.05)}
  .fp-panel--mobile-hidden{display:none !important}
  .fh2{display:none !important}
  .cm{display:none !important}
  .fp-panel{border-right:none !important}
  .fp-dialog{min-width:auto;width:90vw;max-width:340px}
  .fp{padding-bottom:56px}
  .ptb{gap:2px;padding:var(--space-1)}
  .pb{padding:4px;font-size:var(--text-xs)}
  .pp{font-size:11px}
}
</style>
