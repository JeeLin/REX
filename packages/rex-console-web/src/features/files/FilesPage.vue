<script setup lang="ts">
import { ref, reactive, computed, onMounted, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import { onClickOutside } from '@vueuse/core'
import * as filesApi from '@/api/files'
import type { FileEntry } from '@/api/files'
import FolderSyncDialog from './FolderSyncDialog.vue'
import MobileFilesBar from './MobileFilesBar.vue'
import FileEditorDialog from './FileEditorDialog.vue'

const { t } = useI18n()

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
  const key = `${sessionId.value}:dl:${entry.path}`
  const item: TransferItem = {
    sessionId: sessionId.value, remotePath: entry.path, fileName: entry.name,
    type: 'download', status: 'transferring', transferredBytes: 0, totalBytes: entry.size,
    side,
  }
  transferQueue.value.set(key, item)
  transferQueue.value = new Map(transferQueue.value)
  try {
    const blob = await filesApi.downloadFile(sessionId.value, entry.path)
    const url = URL.createObjectURL(blob); const a = document.createElement('a'); a.href = url; a.download = entry.name; a.click(); URL.revokeObjectURL(url)
    item.status = 'completed'; item.transferredBytes = item.totalBytes || blob.size
  } catch (e) {
    item.status = 'failed'
    item.errorMessage = e instanceof Error ? e.message : String(e)
    console.error('Download failed:', e)
  }
  transferQueue.value = new Map(transferQueue.value)
}

/* ---- Transfer queue ---- */
interface TransferItem {
  sessionId: string
  remotePath: string
  fileName: string
  type: 'upload' | 'download'
  status: 'pending' | 'transferring' | 'resuming' | 'failed' | 'completed'
  transferredBytes: number
  totalBytes: number
  side?: Side
  // Upload-specific
  file?: File
  uploadId?: string
  errorMessage?: string
}
const transferQueue = ref<Map<string, TransferItem>>(new Map())
const showTransferQueue = ref(false)
const PART_SIZE = 5 * 1024 * 1024 // 5MB

function fmtPercent(transferred: number, total: number) {
  if (!total) return '0%'
  return `${Math.round((transferred / total) * 100)}%`
}

const completedCount = computed(() => {
  let n = 0
  transferQueue.value.forEach(item => { if (item.status === 'completed') n++ })
  return n
})
const activeCount = computed(() => {
  let n = 0
  transferQueue.value.forEach(item => { if (item.status === 'transferring' || item.status === 'resuming' || item.status === 'pending') n++ })
  return n
})

async function uploadTo(side: Side) {
  if (!sessionId.value) return
  const input = document.createElement('input'); input.type = 'file'; input.multiple = true
  input.onchange = async () => {
    if (!sessionId.value) return
    for (const file of Array.from(input.files || [])) {
      const remotePath = panels[side].path + file.name
      const key = `${sessionId.value}:ul:${remotePath}`
      const item: TransferItem = {
        sessionId: sessionId.value, remotePath, fileName: file.name,
        type: 'upload', status: 'transferring', transferredBytes: 0, totalBytes: file.size,
        file, side,
      }
      transferQueue.value.set(key, item)
      transferQueue.value = new Map(transferQueue.value)
      try {
        if (file.size > PART_SIZE) {
          const result = await filesApi.uploadFileWithProgress(sessionId.value, remotePath, file, (_pct, loaded) => {
            item.transferredBytes = loaded
            transferQueue.value = new Map(transferQueue.value)
          })
          item.uploadId = result.upload_id
          item.status = 'completed'
        } else {
          await filesApi.uploadFile(sessionId.value, remotePath, file)
          item.status = 'completed'
        }
      } catch (e) {
        item.status = 'failed'
        item.errorMessage = e instanceof Error ? e.message : String(e)
        console.error('Upload failed:', e)
      }
    }
    transferQueue.value = new Map(transferQueue.value)
    loadPanel(side)
  }; input.click()
}

async function retryTransfer(key: string) {
  const item = transferQueue.value.get(key)
  if (!item) return
  item.status = item.uploadId ? 'resuming' : 'transferring'
  item.errorMessage = undefined
  transferQueue.value = new Map(transferQueue.value)
  try {
    if (item.type === 'upload') {
      if (item.uploadId && item.file) {
        await filesApi.resumeMultipartUpload(item.sessionId, item.remotePath, item.uploadId, item.file)
      } else if (item.file) {
        await filesApi.uploadFile(item.sessionId, item.remotePath, item.file, item.transferredBytes)
      }
      item.status = 'completed'
    } else {
      // Download retry: use Range header for resume
      const blob = await filesApi.downloadFile(item.sessionId, item.remotePath, item.transferredBytes > 0 ? item.transferredBytes : undefined)
      const url = URL.createObjectURL(blob); const a = document.createElement('a'); a.href = url; a.download = item.fileName; a.click(); URL.revokeObjectURL(url)
      item.status = 'completed'; item.transferredBytes = item.totalBytes || blob.size
    }
  } catch (e) {
    item.status = 'failed'
    item.errorMessage = e instanceof Error ? e.message : String(e)
    console.error('Transfer retry failed:', e)
  }
  transferQueue.value = new Map(transferQueue.value)
}

function dismissCompleted() {
  const q = new Map(transferQueue.value)
  q.forEach((item, key) => { if (item.status === 'completed') q.delete(key) })
  transferQueue.value = q
}

// Context menu
const ctx = ref({ show: false, x: 0, y: 0, path: '', name: '' })
const ctxRef = ref<HTMLElement | null>(null)
onClickOutside(ctxRef, () => { ctx.value.show = false })
function onCtx(e: MouseEvent, entry: FileEntry) { e.preventDefault(); ctx.value = { show: true, x: e.clientX, y: e.clientY, path: entry.path, name: entry.name } }
async function ctxDelete() { confirmCtxDelete(); ctx.value.show = false }
function ctxCopy() { navigator.clipboard?.writeText(ctx.value.path); ctx.value.show = false }
async function ctxPresignedUrl() {
  if (!sessionId.value) return
  try {
    const url = await filesApi.presignedUrl(sessionId.value, ctx.value.path)
    navigator.clipboard?.writeText(url)
    // TODO: show toast "Presigned URL copied to clipboard"
  } catch (e) {
    console.error('Failed to generate presigned URL:', e)
  }
  ctx.value.show = false
}

// Mobile bar actions
function mfbNewFolder() {
  if (!sessionId.value) return
  const name = prompt(t('files.folderNamePrompt'))
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
  const newName = prompt(t('files.newNamePrompt'), entry.name)
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
const isS3 = computed(() => connProtocol.value === 's3')

// ACL dialog
const showAclDialog = ref(false)
const aclPath = ref('')
const aclValue = ref('private')

function openAclDialog(path: string) {
  aclPath.value = path
  aclValue.value = 'private'
  showAclDialog.value = true
  // Load current ACL
  if (sessionId.value) {
    filesApi.getAcl(sessionId.value, path).then(acl => { aclValue.value = acl }).catch(() => {})
  }
}

async function applyAcl() {
  if (!sessionId.value) return
  await filesApi.putAcl(sessionId.value, aclPath.value, aclValue.value)
  showAclDialog.value = false
  await loadPanel('left')
  await loadPanel('right')
}

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

// Edit file
const editorVisible = ref(false)
const editorFilePath = ref('')
function editFile(path: string) {
  if (!sessionId.value) return
  editorFilePath.value = path
  editorVisible.value = true
  ctx.value.show = false
}
function onEditorSaved() {
  editorVisible.value = false
  loadPanel('left'); loadPanel('right')
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
      <button class="fp-switcher-btn" :class="{ 'fp-switcher-btn--active': mobileActiveSide === 'left' }" @click="mobileActiveSide = 'left'">{{ t('files.left') }}</button>
      <button class="fp-switcher-btn" :class="{ 'fp-switcher-btn--active': mobileActiveSide === 'right' }" @click="mobileActiveSide = 'right'">{{ t('files.right') }}</button>
    </div>

    <div v-if="showConnect" class="fp-overlay">
      <div class="fp-dialog">
        <h3>{{ t('files.connectToServer') }}</h3>
        <div class="f"><label>{{ t('files.protocol') }}</label><select v-model="connProtocol" class="mono"><option value="sftp">SFTP</option><option value="s3">S3</option></select></div>
        <template v-if="connProtocol==='sftp'">
          <div class="f"><label>{{ t('files.host') }}</label><input v-model="connHost" class="mono" placeholder="192.168.1.1" /></div>
          <div class="f"><label>{{ t('files.port') }}</label><input v-model.number="connPort" class="mono" type="number" /></div>
          <div class="f"><label>{{ t('files.user') }}</label><input v-model="connUsername" class="mono" /></div>
          <div class="f"><label>{{ t('files.password') }}</label><input v-model="connPassword" class="mono" type="password" /></div>
        </template>
        <template v-if="connProtocol==='s3'">
          <div class="f"><label>{{ t('files.bucket') }}</label><input v-model="connBucket" class="mono" placeholder="my-bucket" /></div>
          <div class="f"><label>{{ t('files.region') }}</label><input v-model="connRegion" class="mono" placeholder="us-east-1 (optional)" /></div>
          <div class="f"><label>{{ t('files.endpointUrl') }}</label><input v-model="connEndpoint" class="mono" placeholder="https://s3.amazonaws.com (optional)" /></div>
          <div class="f"><label>{{ t('files.accessKey') }}</label><input v-model="connAccessKey" class="mono" placeholder="optional for IAM" /></div>
          <div class="f"><label>{{ t('files.secretKey') }}</label><input v-model="connSecretKey" class="mono" type="password" placeholder="optional for IAM" /></div>
        </template>
        <div v-if="connError" class="err">{{ connError }}</div>
        <button class="btn" :disabled="connLoading" @click="doConnect">{{ connLoading ? t('files.connecting') : t('files.connect') }}</button>
      </div>
    </div>

    <template v-for="side in (['left','right'] as const)" :key="side">
      <div class="fp-panel" :class="{ 'fp-panel--active': panels[side].active, 'fp-panel--drop': dropTarget === side, 'fp-panel--mobile-hidden': mobileActiveSide !== side }" :style="side==='left' ? {width:leftW+'px'} : {flex:'1',minWidth:'0'}" @click="activate(side)" @dragover="onDragOver($event, side)" @dragleave="onDragLeave" @drop="onDrop($event, side)">
        <div class="ptb">
          <button class="pb" @click="goUp(side)">↑</button>
          <span class="pp mono">{{ panels[side].path }}</span>
          <button class="pb" :class="{ 'pb--active': syncBrowsing }" :title="t('files.syncBrowsing')" @click="syncBrowsing = !syncBrowsing">🔗</button>
          <button class="pb" :title="t('files.folderSync')" @click="openSyncDialog">🔄</button>
          <button class="pb" @click="uploadTo(side)">⬆</button>
          <button class="pb" @click="loadPanel(side)">↻</button>
        </div>
        <div class="pf">
          <div class="fr fh"><span class="cn">{{ t('files.name') }}</span><span class="cs">{{ t('files.size') }}</span><span class="cm">{{ t('files.modified') }}</span><span v-if="isS3" class="csc">{{ t('files.storageClass') }}</span><span v-if="isS3" class="csc">{{ t('files.acl') }}</span></div>
          <div v-for="e in panels[side].entries" :key="e.name" class="fr" :class="{ 'fr--sel': panels[side].selected.has(e.name) }" draggable="true" @dragstart="onDragStart($event, side, e.name)" @dragend="onDragEnd" @click="toggleSelect(side, e.name, $event)" @dblclick="navigate(side, e)" @contextmenu="onCtx($event, e)">
            <span class="cn"><span class="fi">{{ e.is_dir ? '📁' : '📄' }}</span> {{ e.name }}</span>
            <span class="cs mu">{{ e.is_dir ? '-' : fmtSize(e.size) }}</span>
            <span class="cm mu">{{ e.modified || '-' }}</span>
            <span v-if="isS3" class="csc mu">{{ e.storage_class || '-' }}</span>
            <span v-if="isS3" class="csc mu">{{ e.acl || '-' }}</span>
          </div>
          <div v-if="!panels[side].loading && !panels[side].entries.length" class="pe">{{ t('files.empty') }}</div>
        </div>
        <div class="ps">{{ panels[side].entries.length }} {{ t('files.items') }}<template v-if="panels[side].selected.size"> · {{ panels[side].selected.size }} {{ t('files.selected') }}</template></div>
      </div>
      <div v-if="side==='left'" class="fh2" :class="{ 'fh2--a': dragging }" @mousedown.prevent="onDS" />
    </template>

    <div v-if="ctx.show" ref="ctxRef" class="fctx" :style="{top:ctx.y+'px',left:ctx.x+'px'}">
      <div class="ci" @click="editFile(ctx.path)">{{ t('files.edit') }}</div>
      <div class="ci" @click="ctxCopy">{{ t('files.copyPath') }}</div>
      <div v-if="isS3" class="ci" @click="ctxPresignedUrl">{{ t('files.copyPresignedUrl') }}</div>
      <div class="ci" @click="isS3 ? openAclDialog(ctx.path) : openChmod(ctx.path)">{{ t('files.permissions') }}</div>
      <div class="ci ci--d" @click="ctxDelete">{{ t('files.delete') }}</div>
    </div>

    <!-- Chmod Modal -->
    <Teleport to="body">
      <div v-if="showChmod" class="fp-overlay" @click.self="showChmod = false">
        <div class="fp-dialog">
          <h3>{{ t('files.permissions') }}: {{ chmodPath }}</h3>
          <div class="chmod-grid">
            <div class="chmod-header">
              <span></span>
              <span>{{ t('files.owner') }}</span>
              <span>{{ t('files.group') }}</span>
              <span>{{ t('files.others') }}</span>
            </div>
            <div class="chmod-row">
              <span>{{ t('files.read') }}</span>
              <input v-model="chmodPerms.owner.read" type="checkbox" />
              <input v-model="chmodPerms.group.read" type="checkbox" />
              <input v-model="chmodPerms.other.read" type="checkbox" />
            </div>
            <div class="chmod-row">
              <span>{{ t('files.write') }}</span>
              <input v-model="chmodPerms.owner.write" type="checkbox" />
              <input v-model="chmodPerms.group.write" type="checkbox" />
              <input v-model="chmodPerms.other.write" type="checkbox" />
            </div>
            <div class="chmod-row">
              <span>{{ t('files.execute') }}</span>
              <input v-model="chmodPerms.owner.exec" type="checkbox" />
              <input v-model="chmodPerms.group.exec" type="checkbox" />
              <input v-model="chmodPerms.other.exec" type="checkbox" />
            </div>
          </div>
          <div class="chmod-octal">{{ t('files.octal') }}: {{ calcOctal().toString(8) }}</div>
          <div style="display:flex;gap:var(--space-2);justify-content:flex-end">
            <button class="btn" @click="showChmod = false">{{ t('files.cancel') }}</button>
            <button class="btn" style="background:var(--accent);color:#fff" @click="applyChmod">{{ t('files.apply') }}</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- S3 ACL Dialog -->
    <Teleport to="body">
      <div v-if="showAclDialog" class="fp-overlay" @click.self="showAclDialog = false">
        <div class="fp-dialog">
          <h3>{{ t('files.acl') }}: {{ aclPath }}</h3>
          <div style="margin:var(--space-3) 0">
            <label style="display:block;font-size:var(--text-sm);color:var(--text-muted);margin-bottom:var(--space-1)">{{ t('files.cannedAcl') }}</label>
            <select v-model="aclValue" style="width:100%;padding:var(--space-2);background:var(--bg-secondary);border:1px solid var(--border);border-radius:var(--radius-sm);color:var(--text-primary);font-size:var(--text-sm)">
              <option value="private">private</option>
              <option value="public-read">public-read</option>
              <option value="public-read-write">public-read-write</option>
              <option value="authenticated-read">authenticated-read</option>
            </select>
          </div>
          <div style="display:flex;gap:var(--space-2);justify-content:flex-end">
            <button class="btn" @click="showAclDialog = false">{{ t('files.cancel') }}</button>
            <button class="btn" style="background:var(--accent);color:#fff" @click="applyAcl">{{ t('files.apply') }}</button>
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

    <FileEditorDialog
      :visible="editorVisible"
      :session-id="sessionId || ''"
      :file-path="editorFilePath"
      :protocol="connProtocol"
      @close="editorVisible = false"
      @saved="onEditorSaved"
    />

    <!-- Delete Confirmation -->
    <Teleport to="body">
      <div v-if="showDeleteConfirm" class="fp-overlay" @click.self="cancelDelete">
        <div class="fp-dialog">
          <h3>{{ t('files.confirmDelete') }}</h3>
          <p style="color:var(--text-secondary);margin:0">
            {{ t('files.deleteConfirm') }}
          </p>
          <div style="display:flex;gap:var(--space-2);justify-content:flex-end">
            <button class="btn" style="background:var(--bg-hover);color:var(--text-primary)" @click="cancelDelete">{{ t('files.cancel') }}</button>
            <button class="btn" style="background:var(--danger);color:#fff" @click="executeDelete">{{ t('files.delete') }}</button>
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

    <!-- Transfer Queue Toggle -->
    <button v-if="transferQueue.size > 0" class="tq-toggle" @click="showTransferQueue = !showTransferQueue">
      📥 {{ transferQueue.size }} <span v-if="activeCount">· {{ activeCount }} {{ t('files.active') }}</span>
      <span v-if="completedCount" class="tq-badge tq-badge--done">{{ completedCount }} ✓</span>
    </button>

    <!-- Transfer Queue Panel -->
    <Teleport to="body">
      <Transition name="tq-slide">
        <div v-if="showTransferQueue" class="tq-panel">
          <div class="tq-header">
            <span>{{ t('files.transferQueue') }} ({{ transferQueue.size }})</span>
            <div class="tq-header-actions">
              <button v-if="completedCount" class="tq-btn tq-btn--sm" @click="dismissCompleted">{{ t('files.clearDone') }}</button>
              <button class="tq-btn tq-btn--sm" @click="showTransferQueue = false">✕</button>
            </div>
          </div>
          <div class="tq-list">
            <div v-for="[key, item] in transferQueue" :key="key" class="tq-item" :class="`tq-item--${item.status}`">
              <div class="tq-item-info">
                <span class="tq-item-type">{{ item.type === 'upload' ? '⬆' : '⬇' }}</span>
                <div class="tq-item-details">
                  <span class="tq-item-name">{{ item.fileName }}</span>
                  <span class="tq-item-path">{{ item.remotePath }}</span>
                </div>
              </div>
              <div class="tq-item-status">
                <!-- Progress bar for active transfers -->
                <template v-if="item.status === 'transferring' || item.status === 'resuming'">
                  <div class="tq-progress">
                    <div class="tq-progress-bar" :style="{ width: fmtPercent(item.transferredBytes, item.totalBytes) }"></div>
                  </div>
                  <span class="tq-item-pct">{{ fmtPercent(item.transferredBytes, item.totalBytes) }}</span>
                </template>
                <!-- Failed: show error + retry -->
                <template v-else-if="item.status === 'failed'">
                  <span class="tq-item-error" :title="item.errorMessage">{{ item.errorMessage || t('files.failed') }}</span>
                  <button class="tq-btn tq-btn--retry" @click="retryTransfer(key)">↻ {{ t('files.retry') }}</button>
                </template>
                <!-- Completed -->
                <template v-else-if="item.status === 'completed'">
                  <span class="tq-item-done">✓</span>
                </template>
                <!-- Pending -->
                <template v-else-if="item.status === 'pending'">
                  <span class="tq-item-pending">{{ t('files.waiting') }}</span>
                </template>
              </div>
            </div>
            <div v-if="!transferQueue.size" class="tq-empty">{{ t('files.noTransfers') }}</div>
          </div>
        </div>
      </Transition>
    </Teleport>
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
.csc{width:100px;text-align:right}
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
  .csc{display:none !important}
  .fp-panel{border-right:none !important}
  .fp-dialog{min-width:auto;width:90vw;max-width:340px}
  .fp{padding-bottom:56px}
  .ptb{gap:2px;padding:var(--space-1)}
  .pb{padding:4px;font-size:var(--text-xs)}
  .pp{font-size:11px}
}

/* Transfer queue toggle button */
.tq-toggle{position:fixed;bottom:var(--space-4);right:var(--space-4);z-index:90;display:flex;align-items:center;gap:var(--space-2);padding:var(--space-2) var(--space-3);background:var(--bg-elevated);border:1px solid var(--border);border-radius:var(--radius);cursor:pointer;font-size:var(--text-sm);color:var(--text-primary);box-shadow:var(--shadow);transition:border-color 0.15s}
.tq-toggle:hover{border-color:var(--accent)}
.tq-badge{font-size:var(--text-xs);padding:1px 6px;border-radius:var(--radius-sm)}
.tq-badge--done{background:rgba(34,197,94,0.15);color:#22c55e}

/* Transfer queue panel */
.tq-panel{position:fixed;bottom:0;right:0;z-index:200;width:380px;max-height:50vh;background:var(--bg-elevated);border-top:1px solid var(--border);border-left:1px solid var(--border);border-radius:var(--radius) var(--radius) 0 0;display:flex;flex-direction:column;box-shadow:0 -4px 16px rgba(0,0,0,0.25)}
.tq-header{display:flex;align-items:center;justify-content:space-between;padding:var(--space-2) var(--space-3);border-bottom:1px solid var(--border);font-size:var(--text-sm);font-weight:600;color:var(--text-primary)}
.tq-header-actions{display:flex;gap:var(--space-2)}
.tq-list{flex:1;overflow-y:auto;padding:var(--space-1) 0}
.tq-item{display:flex;align-items:flex-start;justify-content:space-between;padding:var(--space-2) var(--space-3);font-size:var(--text-sm);border-bottom:1px solid var(--border);gap:var(--space-3)}
.tq-item:last-child{border-bottom:none}
.tq-item--failed{background:rgba(239,68,68,0.04)}
.tq-item--completed{opacity:0.6}
.tq-item-info{display:flex;align-items:center;gap:var(--space-2);min-width:0;flex:1}
.tq-item-type{font-size:14px;flex-shrink:0}
.tq-item-details{display:flex;flex-direction:column;min-width:0}
.tq-item-name{color:var(--text-primary);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.tq-item-path{font-size:var(--text-xs);color:var(--text-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.tq-item-status{display:flex;align-items:center;gap:var(--space-2);flex-shrink:0}
.tq-item-pct{font-size:var(--text-xs);color:var(--text-muted);min-width:36px;text-align:right}
.tq-item-error{font-size:var(--text-xs);color:var(--danger);max-width:120px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.tq-item-done{color:#22c55e;font-weight:600}
.tq-item-pending{font-size:var(--text-xs);color:var(--text-muted)}

/* Progress bar */
.tq-progress{width:60px;height:4px;background:var(--bg-deep);border-radius:2px;overflow:hidden}
.tq-progress-bar{height:100%;background:var(--accent);border-radius:2px;transition:width 0.2s}

/* Buttons */
.tq-btn{padding:var(--space-1) var(--space-2);background:var(--bg-hover);border:1px solid var(--border);border-radius:var(--radius-sm);cursor:pointer;font-size:var(--text-xs);color:var(--text-primary);white-space:nowrap}
.tq-btn:hover{background:var(--bg-deep)}
.tq-btn--retry{color:var(--accent);border-color:var(--accent)}
.tq-btn--retry:hover{background:rgba(232,145,45,0.1)}
.tq-btn--sm{padding:2px var(--space-2);font-size:var(--text-xs)}
.tq-empty{padding:var(--space-4);text-align:center;color:var(--text-muted);font-size:var(--text-sm)}

/* Slide transition */
.tq-slide-enter-active,.tq-slide-leave-active{transition:transform 0.2s ease,opacity 0.2s ease}
.tq-slide-enter-from,.tq-slide-leave-to{transform:translateY(100%);opacity:0}

@media(max-width:768px){
  .tq-panel{width:100%;max-height:60vh}
  .tq-toggle{bottom:60px}
}
</style>
