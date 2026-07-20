<script setup lang="ts">
import { ref, reactive, onMounted, onBeforeUnmount } from 'vue'
import { onClickOutside } from '@vueuse/core'
import * as filesApi from '@/api/files'
import type { FileEntry } from '@/api/files'

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

// Auto-connect on mount if props provided
onMounted(async () => {
  if (props.host) {
    await doConnect()
  }
})

async function doConnect() {
  connLoading.value = true; connError.value = ''
  try {
    sessionId.value = await filesApi.connect({
      protocol: connProtocol.value, host: connHost.value, port: connPort.value,
      username: connUsername.value || undefined, password: connPassword.value || undefined,
    })
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

async function deleteSelected(side: Side) {
  if (!sessionId.value) return
  for (const name of panels[side].selected) {
    const entry = panels[side].entries.find(e => e.name === name)
    if (entry) await filesApi.deleteFile(sessionId.value, entry.path)
  }
  panels[side].selected.clear(); loadPanel(side)
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
async function ctxDelete() { if (sessionId.value) await filesApi.deleteFile(sessionId.value, ctx.value.path); ctx.value.show = false; await loadPanel('left'); await loadPanel('right') }
function ctxCopy() { navigator.clipboard?.writeText(ctx.value.path); ctx.value.show = false }

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
</script>

<template>
  <div class="fp" @mousemove.prevent>
    <div v-if="showConnect" class="fp-overlay">
      <div class="fp-dialog">
        <h3>Connect to Server</h3>
        <div class="f"><label>Protocol</label><select v-model="connProtocol" class="mono"><option value="sftp">SFTP</option><option value="s3">S3</option></select></div>
        <div class="f"><label>Host</label><input v-model="connHost" class="mono" placeholder="192.168.1.1" /></div>
        <div class="f"><label>Port</label><input v-model.number="connPort" class="mono" type="number" /></div>
        <div v-if="connProtocol==='sftp'" class="f"><label>User</label><input v-model="connUsername" class="mono" /></div>
        <div v-if="connProtocol==='sftp'" class="f"><label>Password</label><input v-model="connPassword" class="mono" type="password" /></div>
        <div v-if="connError" class="err">{{ connError }}</div>
        <button class="btn" :disabled="connLoading" @click="doConnect">{{ connLoading ? 'Connecting...' : 'Connect' }}</button>
      </div>
    </div>

    <template v-for="side in (['left','right'] as const)" :key="side">
      <div class="fp-panel" :class="{ 'fp-panel--active': panels[side].active }" :style="side==='left' ? {width:leftW+'px'} : {flex:'1',minWidth:'0'}" @click="activate(side)">
        <div class="ptb">
          <button class="pb" @click="goUp(side)">↑</button>
          <span class="pp mono">{{ panels[side].path }}</span>
          <button class="pb" :class="{ 'pb--active': syncBrowsing }" title="Sync Browsing" @click="syncBrowsing = !syncBrowsing">🔗</button>
          <button class="pb" @click="uploadTo(side)">⬆</button>
          <button class="pb" @click="loadPanel(side)">↻</button>
        </div>
        <div class="pf">
          <div class="fr fh"><span class="cn">Name</span><span class="cs">Size</span><span class="cm">Modified</span></div>
          <div v-for="e in panels[side].entries" :key="e.name" class="fr" :class="{ 'fr--sel': panels[side].selected.has(e.name) }" @click="toggleSelect(side, e.name, $event)" @dblclick="navigate(side, e)" @contextmenu="onCtx($event, e)">
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
      <div class="ci" @click="ctxCopy">Copy Path</div>
      <div class="ci ci--d" @click="ctxDelete">Delete</div>
    </div>
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
</style>
