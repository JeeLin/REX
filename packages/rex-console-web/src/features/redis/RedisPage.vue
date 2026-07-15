<script setup lang="ts">
import { ref, computed, onBeforeUnmount } from 'vue'
import { onClickOutside } from '@vueuse/core'
import * as redisApi from '@/api/redis'
import type { DbInfo, KeyInfo, RedisValue } from '@/api/redis'

const sessionId = ref<string | null>(null)
const connecting = ref(false)
const connectError = ref('')

// Connection form
const connHost = ref('127.0.0.1')
const connPort = ref(6379)
const connPassword = ref('')
const showConnect = ref(true)

async function doConnect() {
  connecting.value = true
  connectError.value = ''
  try {
    sessionId.value = await redisApi.connect(connHost.value, connPort.value, connPassword.value || undefined)
    showConnect.value = false
    await loadDatabases()
  } catch (e: unknown) {
    connectError.value = e instanceof Error ? e.message : String(e)
  } finally {
    connecting.value = false
  }
}

// Databases
const databases = ref<DbInfo[]>([])
const currentDb = ref(0)

async function loadDatabases() {
  if (!sessionId.value) return
  try {
    databases.value = await redisApi.getDatabases(sessionId.value)
  } catch { /* ignore */ }
}

async function switchDb(db: number) {
  if (!sessionId.value) return
  await redisApi.selectDb(sessionId.value, db)
  currentDb.value = db
  await loadKeys()
}

// Key tree
const keys = ref<KeyInfo[]>([])
const keyLoading = ref(false)
const searchPattern = ref('*')
const selectedKeys = ref<Set<string>>(new Set())

// Namespace tree
interface NamespaceNode {
  name: string
  fullName: string
  children: Map<string, NamespaceNode>
  keys: KeyInfo[]
  expanded: boolean
}

const namespaceTree = computed(() => {
  const root = new Map<string, NamespaceNode>()
  for (const k of keys.value) {
    const parts = k.key.split(':')
    let current = root
    let fullName = ''
    for (let i = 0; i < parts.length - 1; i++) {
      const part = parts[i]!
      fullName += (i > 0 ? ':' : '') + part
      if (!current.has(part)) {
        current.set(part, { name: part, fullName, children: new Map(), keys: [], expanded: false })
      }
      current = current.get(part)!.children
    }
    // Last part is the key itself (or leaf namespace)
    const leafName = parts[parts.length - 1] || k.key
    fullName += (fullName ? ':' : '') + leafName
    if (!current.has(leafName)) {
      current.set(leafName, { name: leafName, fullName, children: new Map(), keys: [], expanded: false })
    }
    current.get(leafName)!.keys.push(k)
  }
  return root
})

async function loadKeys() {
  if (!sessionId.value) return
  keyLoading.value = true
  try {
    keys.value = await redisApi.scan(sessionId.value, searchPattern.value, 500)
  } catch { /* ignore */ }
  finally { keyLoading.value = false }
}

function toggleKey(key: string) {
  if (selectedKeys.value.has(key)) {
    selectedKeys.value.delete(key)
  } else {
    selectedKeys.value.add(key)
  }
  selectedKeys.value = new Set(selectedKeys.value)
}

// Value viewer
const selectedKey = ref<string | null>(null)
const keyValue = ref<RedisValue | null>(null)
const valueLoading = ref(false)

async function viewKey(key: string) {
  selectedKey.value = key
  if (!sessionId.value) return
  valueLoading.value = true
  try {
    keyValue.value = await redisApi.getValue(sessionId.value, key)
  } catch { keyValue.value = null }
  finally { valueLoading.value = false }
}

async function deleteSelected() {
  if (!sessionId.value || selectedKeys.value.size === 0) return
  await redisApi.delKeys(sessionId.value, Array.from(selectedKeys.value))
  selectedKeys.value.clear()
  selectedKey.value = null
  keyValue.value = null
  await loadKeys()
}

// Panel resize
const panelWidth = ref(280)
const dragging = ref(false)
let startX = 0
let startW = 0

function onDragStart(e: MouseEvent) {
  dragging.value = true
  startX = e.clientX
  startW = panelWidth.value
  document.addEventListener('mousemove', onDragMove)
  document.addEventListener('mouseup', onDragEnd)
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
}

function onDragMove(e: MouseEvent) {
  panelWidth.value = Math.min(500, Math.max(200, startW + (e.clientX - startX)))
}

function onDragEnd() {
  dragging.value = false
  document.removeEventListener('mousemove', onDragMove)
  document.removeEventListener('mouseup', onDragEnd)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}

onBeforeUnmount(() => {
  document.removeEventListener('mousemove', onDragMove)
  document.removeEventListener('mouseup', onDragEnd)
})

// Context menu
const ctxMenu = ref({ show: false, x: 0, y: 0, key: '' })
const ctxMenuRef = ref<HTMLElement | null>(null)
onClickOutside(ctxMenuRef, () => { ctxMenu.value.show = false })

function onContextMenu(e: MouseEvent, key: string) {
  e.preventDefault()
  ctxMenu.value = { show: true, x: e.clientX, y: e.clientY, key }
}

async function ctxDelete() {
  if (!sessionId.value) return
  await redisApi.delKeys(sessionId.value, [ctxMenu.value.key])
  ctxMenu.value.show = false
  await loadKeys()
}

function ctxCopy() {
  navigator.clipboard?.writeText(ctxMenu.value.key)
  ctxMenu.value.show = false
}
</script>

<template>
  <div class="redis-page" @mousemove.prevent>
    <!-- Connect dialog -->
    <div v-if="showConnect" class="redis-connect-overlay">
      <div class="redis-connect-dialog">
        <h3 class="dialog-title">Connect to Redis</h3>
        <div class="dialog-field">
          <label>Host</label>
          <input v-model="connHost" class="mono" placeholder="127.0.0.1" />
        </div>
        <div class="dialog-field">
          <label>Port</label>
          <input v-model.number="connPort" class="mono" type="number" placeholder="6379" />
        </div>
        <div class="dialog-field">
          <label>Password</label>
          <input v-model="connPassword" class="mono" type="password" placeholder="(optional)" />
        </div>
        <div v-if="connectError" class="dialog-error">{{ connectError }}</div>
        <button class="btn-primary" :disabled="connecting" @click="doConnect">
          {{ connecting ? 'Connecting...' : 'Connect' }}
        </button>
      </div>
    </div>

    <!-- Left panel: DB selector + key tree -->
    <div class="redis-panel" :style="{ width: panelWidth + 'px' }">
      <!-- DB selector -->
      <div class="redis-dbs">
        <div
          v-for="db in databases"
          :key="db.index"
          class="redis-db-item"
          :class="{ 'redis-db-item--active': db.index === currentDb }"
          @click="switchDb(db.index)"
        >
          <span class="redis-db-name">db{{ db.index }}</span>
          <span class="redis-db-count">{{ db.keys }}</span>
        </div>
      </div>

      <!-- Search + actions -->
      <div class="redis-tree-toolbar">
        <input
          v-model="searchPattern"
          class="redis-search mono"
          placeholder="Pattern: *"
          @keyup.enter="loadKeys"
        />
        <button class="redis-toolbar-btn" title="Refresh" @click="loadKeys">↻</button>
        <button
          v-if="selectedKeys.size > 0"
          class="redis-toolbar-btn redis-toolbar-btn--danger"
          title="Delete selected"
          @click="deleteSelected"
        >🗑</button>
      </div>

      <!-- Key tree -->
      <div class="redis-tree" :class="{ 'redis-tree--loading': keyLoading }">
        <div
          v-for="k in keys"
          :key="k.key"
          class="redis-key-item"
          :class="{ 'redis-key-item--selected': selectedKey === k.key }"
          @click="viewKey(k.key)"
          @contextmenu="onContextMenu($event, k.key)"
        >
          <span class="redis-key-type" :class="'type-' + k.type_name">{{ k.type_name[0] }}</span>
          <span class="redis-key-name mono">{{ k.key }}</span>
        </div>
        <div v-if="!keyLoading && keys.length === 0" class="redis-tree-empty">
          No keys found
        </div>
      </div>
    </div>

    <!-- Resize handle -->
    <div
      class="redis-handle"
      :class="{ 'redis-handle--active': dragging }"
      @mousedown.prevent="onDragStart"
    />

    <!-- Right panel: value viewer -->
    <div class="redis-content">
      <div v-if="!selectedKey" class="redis-content-placeholder">
        Select a key to view its value
      </div>
      <div v-else class="redis-value-panel">
        <div class="redis-value-header">
          <span class="redis-value-key mono">{{ selectedKey }}</span>
          <button class="redis-toolbar-btn redis-toolbar-btn--danger" @click="deleteSelected">Delete</button>
        </div>
        <div v-if="valueLoading" class="redis-value-loading">Loading...</div>
        <div v-else-if="keyValue" class="redis-value-body">
          <div class="redis-value-type">Type: {{ keyValue.type }}</div>
          <!-- String -->
          <pre v-if="keyValue.type === 'String'" class="redis-value-text mono">{{ keyValue.value }}</pre>
          <!-- Hash -->
          <table v-else-if="keyValue.type === 'Hash'" class="redis-value-table">
            <thead><tr><th>#</th><th>Field</th><th>Value</th></tr></thead>
            <tbody>
              <tr v-for="(entry, i) in (keyValue.value as [string, string][])" :key="i">
                <td class="muted">{{ i + 1 }}</td>
                <td class="mono">{{ entry[0] }}</td>
                <td class="mono">{{ entry[1] }}</td>
              </tr>
            </tbody>
          </table>
          <!-- List / Set -->
          <table v-else-if="keyValue.type === 'List' || keyValue.type === 'Set'" class="redis-value-table">
            <thead><tr><th>#</th><th>Value</th></tr></thead>
            <tbody>
              <tr v-for="(val, i) in (keyValue.value as string[])" :key="i">
                <td class="muted">{{ i + 1 }}</td>
                <td class="mono">{{ val }}</td>
              </tr>
            </tbody>
          </table>
          <!-- ZSet -->
          <table v-else-if="keyValue.type === 'ZSet'" class="redis-value-table">
            <thead><tr><th>#</th><th>Score</th><th>Member</th></tr></thead>
            <tbody>
              <tr v-for="(entry, i) in (keyValue.value as [string, number][])" :key="i">
                <td class="muted">{{ i + 1 }}</td>
                <td class="mono">{{ entry[1] }}</td>
                <td class="mono">{{ entry[0] }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- Context menu -->
    <div
      v-if="ctxMenu.show"
      ref="ctxMenuRef"
      class="redis-ctx-menu"
      :style="{ top: ctxMenu.y + 'px', left: ctxMenu.x + 'px' }"
    >
      <div class="ctx-item" @click="ctxCopy">Copy Key Name</div>
      <div class="ctx-item ctx-item--danger" @click="ctxDelete">Delete</div>
    </div>
  </div>
</template>

<style scoped>
.redis-page {
  display: flex;
  height: 100%;
  background: var(--bg-page);
  position: relative;
}

/* ---- connect dialog ---- */
.redis-connect-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0,0,0,0.6);
}

.redis-connect-dialog {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--space-5);
  min-width: 320px;
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.dialog-title {
  margin: 0;
  font-size: var(--text-lg);
  color: var(--text-primary);
}

.dialog-field {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.dialog-field label {
  font-size: var(--text-xs);
  color: var(--text-muted);
  text-transform: uppercase;
}

.dialog-field input {
  padding: var(--space-2);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}

.dialog-field input:focus {
  border-color: var(--accent);
}

.dialog-error {
  color: var(--danger);
  font-size: var(--text-sm);
}

.btn-primary {
  padding: var(--space-2) var(--space-4);
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--text-sm);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* ---- left panel ---- */
.redis-panel {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border);
  overflow: hidden;
}

.redis-dbs {
  display: flex;
  flex-wrap: wrap;
  gap: 2px;
  padding: var(--space-2);
  border-bottom: 1px solid var(--border);
}

.redis-db-item {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--text-xs);
  color: var(--text-muted);
  transition: background var(--transition);
}

.redis-db-item:hover {
  background: var(--bg-hover);
}

.redis-db-item--active {
  background: var(--accent);
  color: #fff;
}

.redis-db-count {
  font-size: 10px;
  opacity: 0.7;
}

.redis-tree-toolbar {
  display: flex;
  gap: var(--space-1);
  padding: var(--space-2);
  border-bottom: 1px solid var(--border);
}

.redis-search {
  flex: 1;
  padding: var(--space-1) var(--space-2);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-xs);
  outline: none;
}

.redis-search:focus {
  border-color: var(--accent);
}

.redis-toolbar-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: var(--space-1);
  border-radius: var(--radius-sm);
}

.redis-toolbar-btn:hover {
  color: var(--text-primary);
}

.redis-toolbar-btn--danger:hover {
  color: var(--danger);
}

.redis-tree {
  flex: 1;
  overflow-y: auto;
}

.redis-tree--loading {
  opacity: 0.5;
}

.redis-key-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  cursor: pointer;
  font-size: var(--text-sm);
  color: var(--text-primary);
  transition: background var(--transition);
}

.redis-key-item:hover {
  background: var(--bg-hover);
}

.redis-key-item--selected {
  background: var(--bg-hover);
  border-left: 2px solid var(--accent);
}

.redis-key-type {
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 3px;
  font-size: 10px;
  font-weight: 700;
  flex-shrink: 0;
}

.type-string { background: rgba(63,185,80,0.2); color: #3FB950; }
.type-list { background: rgba(139,92,246,0.2); color: #8B5CF6; }
.type-set { background: rgba(88,166,255,0.2); color: #58A6FF; }
.type-zset { background: rgba(232,145,45,0.2); color: #E8912D; }
.type-hash { background: rgba(248,81,73,0.2); color: #F85149; }

.redis-key-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--text-xs);
}

.redis-tree-empty {
  padding: var(--space-4);
  text-align: center;
  color: var(--text-muted);
  font-size: var(--text-sm);
}

/* ---- handle ---- */
.redis-handle {
  width: 4px;
  cursor: col-resize;
  background: var(--border);
  flex-shrink: 0;
}

.redis-handle:hover, .redis-handle--active {
  background: var(--accent);
}

/* ---- right panel ---- */
.redis-content {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.redis-content-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-muted);
  font-size: var(--text-sm);
}

.redis-value-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.redis-value-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
  background: var(--bg-surface);
}

.redis-value-key {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
}

.redis-value-loading {
  padding: var(--space-4);
  text-align: center;
  color: var(--text-muted);
}

.redis-value-body {
  flex: 1;
  overflow: auto;
  padding: var(--space-3);
}

.redis-value-type {
  font-size: var(--text-xs);
  color: var(--text-muted);
  margin-bottom: var(--space-2);
}

.redis-value-text {
  padding: var(--space-3);
  background: var(--bg-deep);
  border-radius: var(--radius);
  font-size: var(--text-sm);
  white-space: pre-wrap;
  word-break: break-all;
  overflow: auto;
  max-height: 400px;
}

.redis-value-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-sm);
}

.redis-value-table th,
.redis-value-table td {
  padding: var(--space-1) var(--space-2);
  text-align: left;
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.redis-value-table thead {
  background: var(--bg-surface);
  position: sticky;
  top: 0;
}

.muted { color: var(--text-muted); }

/* ---- context menu ---- */
.redis-ctx-menu {
  position: fixed;
  z-index: 200;
  min-width: 160px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  padding: var(--space-1) 0;
}

.ctx-item {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  cursor: pointer;
  color: var(--text-primary);
}

.ctx-item:hover {
  background: var(--bg-hover);
}

.ctx-item--danger {
  color: var(--danger);
}
</style>
