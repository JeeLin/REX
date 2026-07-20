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

onMounted(async () => {
  if (props.host) {
    await doConnect()
  }
})

watch(() => props.host, async () => {
  if (props.host && !connected.value) {
    await doConnect()
  }
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
  try {
    entries.value = await filesApi.listFiles(sessionId.value, currentPath.value)
  } catch { entries.value = [] } finally { loading.value = false }
}

function navigate(entry: FileEntry) {
  if (entry.is_dir) {
    currentPath.value = entry.path.endsWith('/') ? entry.path : entry.path + '/'
    selected.value.clear()
    loadDir()
  }
}

function goUp() {
  const parts = currentPath.value.replace(/\/$/, '').split('/')
  parts.pop()
  currentPath.value = parts.length ? parts.join('/') + '/' : '/'
  selected.value.clear()
  loadDir()
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
    selected.value.clear()
    selected.value.add(name)
  }
  selected.value = new Set(selected.value)
}

// Upload
function upload() {
  if (!sessionId.value) return
  const input = document.createElement('input')
  input.type = 'file'; input.multiple = true
  input.onchange = async () => {
    for (const file of Array.from(input.files || [])) {
      await filesApi.uploadFile(sessionId.value!, currentPath.value + file.name, file)
    }
    loadDir()
  }
  input.click()
}

// Download
function downloadSelected() {
  if (!sessionId.value || selected.value.size !== 1) return
  const name = Array.from(selected.value)[0]
  const entry = entries.value.find(e => e.name === name)
  if (!entry || entry.is_dir) return
  filesApi.downloadFile(sessionId.value, entry.path).then(blob => {
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a'); a.href = url; a.download = entry.name; a.click()
    URL.revokeObjectURL(url)
  })
}

// Delete
async function deleteSelected() {
  if (!sessionId.value) return
  for (const name of selected.value) {
    const entry = entries.value.find(e => e.name === name)
    if (entry) await filesApi.deleteFile(sessionId.value, entry.path)
  }
  selected.value.clear()
  loadDir()
}

// Context menu
const ctx = ref({ show: false, x: 0, y: 0, name: '', path: '', isDir: false })
const ctxRef = ref<HTMLElement | null>(null)
onClickOutside(ctxRef, () => { ctx.value.show = false })

function onCtx(e: MouseEvent, entry: FileEntry) {
  e.preventDefault()
  ctx.value = { show: true, x: e.clientX, y: e.clientY, name: entry.name, path: entry.path, isDir: entry.is_dir }
}

async function ctxDelete() {
  if (sessionId.value) await filesApi.deleteFile(sessionId.value, ctx.value.path)
  ctx.value.show = false
  loadDir()
}

function ctxCopy() {
  navigator.clipboard?.writeText(ctx.value.path)
  ctx.value.show = false
}

// Breadcrumb
const crumbs = ref<{ name: string; path: string }[]>([])
watch(currentPath, (p) => {
  const parts = p.replace(/\/$/, '').split('/').filter(Boolean)
  const result: { name: string; path: string }[] = [{ name: '/', path: '/' }]
  let acc = ''
  for (const part of parts) {
    acc += '/' + part
    result.push({ name: part, path: acc + '/' })
  }
  crumbs.value = result
}, { immediate: true })

function goCrumb(path: string) {
  currentPath.value = path
  selected.value.clear()
  loadDir()
}

function fmtSize(b: number) {
  if (!b) return '-'
  const u = ['B', 'KB', 'MB', 'GB']
  let i = 0, s = b
  while (s >= 1024 && i < 3) { s /= 1024; i++ }
  return `${s.toFixed(i ? 1 : 0)} ${u[i]}`
}

onBeforeUnmount(async () => {
  if (sessionId.value) {
    try { await filesApi.disconnect(sessionId.value) } catch { /* ignore */ }
  }
})
</script>

<template>
  <div class="fd">
    <!-- Toolbar -->
    <div class="fd-toolbar">
      <div class="fd-breadcrumb mono">
        <span
          v-for="(crumb, i) in crumbs"
          :key="crumb.path"
          class="fd-crumb"
          @click="goCrumb(crumb.path)"
        >
          <span v-if="i > 0" class="fd-crumb-sep">/</span>
          <span class="fd-crumb-name">{{ crumb.name }}</span>
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
          v-for="entry in entries"
          :key="entry.name"
          class="fd-row"
          :class="{ 'fd-row--sel': selected.has(entry.name) }"
          @click="select(entry.name, $event)"
          @dblclick="navigate(entry)"
          @contextmenu="onCtx($event, entry)"
        >
          <span class="fd-icon">{{ entry.is_dir ? '📁' : '📄' }}</span>
          <span class="fd-name">{{ entry.name }}</span>
          <span class="fd-size mono muted">{{ entry.is_dir ? '-' : fmtSize(entry.size) }}</span>
        </div>
        <div v-if="!entries.length" class="fd-status muted">Empty</div>
      </template>
    </div>

    <!-- Context menu -->
    <div v-if="ctx.show" ref="ctxRef" class="fd-ctx" :style="{ top: ctx.y + 'px', left: ctx.x + 'px' }">
      <div class="fd-ctx-item" @click="ctxCopy">Copy Path</div>
      <div v-if="!ctx.isDir" class="fd-ctx-item" @click="downloadSelected">Download</div>
      <div class="fd-ctx-item fd-ctx-item--d" @click="ctxDelete">Delete</div>
    </div>
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

.fd-list { flex: 1; overflow-y: auto; }

.fd-row {
  display: flex; align-items: center; gap: var(--space-2);
  padding: 2px var(--space-2); font-size: var(--text-sm); cursor: pointer;
  color: var(--text-secondary); border-left: 2px solid transparent;
}
.fd-row:hover { background: var(--bg-hover); }
.fd-row--sel { background: var(--bg-hover); border-left-color: var(--accent); }

.fd-icon { font-size: 12px; width: 16px; text-align: center; flex-shrink: 0; }
.fd-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-primary); }
.fd-size { width: 70px; text-align: right; flex-shrink: 0; }

.fd-status, .fd-error { padding: var(--space-3); text-align: center; font-size: var(--text-sm); }
.fd-error { color: var(--danger); }

.fd-ctx {
  position: fixed; z-index: 200; min-width: 140px;
  background: var(--bg-elevated); border: 1px solid var(--border);
  border-radius: var(--radius); box-shadow: var(--shadow); padding: var(--space-1) 0;
}
.fd-ctx-item {
  padding: var(--space-1) var(--space-3); font-size: var(--text-sm);
  color: var(--text-primary); cursor: pointer;
}
.fd-ctx-item:hover { background: var(--bg-hover); }
.fd-ctx-item--d { color: var(--danger); }

.muted { color: var(--text-muted); }
.mono { font-family: var(--font-mono); }
</style>
