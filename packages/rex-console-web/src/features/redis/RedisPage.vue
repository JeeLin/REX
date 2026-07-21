<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { onClickOutside } from '@vueuse/core'
import * as redisApi from '@/api/redis'
import type { DbInfo, KeyInfo, RedisValue } from '@/api/redis'
import FormatViewer from './FormatViewer.vue'

const props = defineProps<{
  resourceId?: string
  host?: string
  port?: number
  password?: string
  db?: number
}>()

const sessionId = ref<string | null>(null)
const connecting = ref(false)
const connectError = ref('')

// Connection form
const connHost = ref(props.host || '127.0.0.1')
const connPort = ref(props.port || 6379)
const connPassword = ref(props.password || '')
const showConnect = ref(!props.host)

// Connection management
interface Connection {
  id: string
  name: string
  host: string
  port: number
  password: string
}

const connections = ref<Connection[]>([])
const showEditConnection = ref(false)
const editingConnection = ref<Connection | null>(null)
const showDeleteConnection = ref(false)
const deletingConnection = ref<Connection | null>(null)

// Auto-connect on mount if props provided
onMounted(async () => {
  if (props.host) {
    await doConnect()
  }
})

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

// Connection management functions
function editConnection(conn: Connection) {
  editingConnection.value = { ...conn }
  showEditConnection.value = true
}

function saveConnection() {
  if (!editingConnection.value) return
  const idx = connections.value.findIndex((c) => c.id === editingConnection.value!.id)
  if (idx >= 0) {
    connections.value[idx] = editingConnection.value
  }
  showEditConnection.value = false
  editingConnection.value = null
}

function deleteConnection(conn: Connection) {
  deletingConnection.value = conn
  showDeleteConnection.value = true
}

function confirmDeleteConnection() {
  if (!deletingConnection.value) return
  connections.value = connections.value.filter((c) => c.id !== deletingConnection.value!.id)
  showDeleteConnection.value = false
  deletingConnection.value = null
}

function copyConnection(conn: Connection) {
  const newConn: Connection = {
    id: Date.now().toString(),
    name: conn.name + ' (copy)',
    host: conn.host,
    port: conn.port,
    password: conn.password,
  }
  connections.value.push(newConn)
  editConnection(newConn)
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
  try {
    await redisApi.selectDb(sessionId.value, db)
    currentDb.value = db
    await loadKeys()
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e)
    alert(`Failed to switch database: ${msg}`)
  }
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

// Tab management
const openTabs = ref<string[]>([])
const activeTab = ref<string | null>(null)
const tabValues = ref<Map<string, RedisValue>>(new Map())

async function viewKey(key: string) {
  selectedKey.value = key
  activeTab.value = null
  if (!sessionId.value) return
  valueLoading.value = true
  try {
    keyValue.value = await redisApi.getValue(sessionId.value, key)
    // Load stream data if stream type
    if (keyValue.value?.type === 'Stream') {
      streamTab.value = 'messages'
      loadStreamData()
    }
  } catch { keyValue.value = null }
  finally { valueLoading.value = false }
}

function openInNewTab() {
  const key = selectedKey.value || activeTab.value
  if (!key) return
  if (!openTabs.value.includes(key)) {
    openTabs.value.push(key)
    tabValues.value.set(key, keyValue.value!)
  }
  activeTab.value = key
}

async function switchTab(key: string) {
  activeTab.value = key
  selectedKey.value = null
  const cached = tabValues.value.get(key)
  if (cached) {
    keyValue.value = cached
  } else if (sessionId.value) {
    valueLoading.value = true
    try {
      keyValue.value = await redisApi.getValue(sessionId.value, key)
      tabValues.value.set(key, keyValue.value!)
    } catch { keyValue.value = null }
    finally { valueLoading.value = false }
  }
}

function closeTab(key: string) {
  const idx = openTabs.value.indexOf(key)
  if (idx >= 0) {
    openTabs.value.splice(idx, 1)
    tabValues.value.delete(key)
  }
  if (activeTab.value === key) {
    activeTab.value = openTabs.value[Math.min(idx, openTabs.value.length - 1)] || null
    if (activeTab.value) {
      switchTab(activeTab.value)
    }
  }
}

async function deleteSelected() {
  if (!sessionId.value || selectedKeys.value.size === 0) return
  await redisApi.delKeys(sessionId.value, Array.from(selectedKeys.value))
  selectedKeys.value.clear()
  selectedKey.value = null
  keyValue.value = null
  await loadKeys()
}

// Batch operations
const showBatchTtl = ref(false)
const batchTtlValue = ref(3600)
const showBatchDelete = ref(false)
const showExport = ref(false)
const showImport = ref(false)

function selectAll() {
  for (const k of keys.value) {
    selectedKeys.value.add(k.key)
  }
  selectedKeys.value = new Set(selectedKeys.value)
}

function clearSelection() {
  selectedKeys.value.clear()
  selectedKeys.value = new Set(selectedKeys.value)
}

async function batchSetTtl() {
  if (!sessionId.value || selectedKeys.value.size === 0) return
  for (const key of selectedKeys.value) {
    await redisApi.setTtl(sessionId.value, key, batchTtlValue.value)
  }
  showBatchTtl.value = false
  await loadKeys()
}

async function batchDelete() {
  if (!sessionId.value || selectedKeys.value.size === 0) return
  await redisApi.delKeys(sessionId.value, Array.from(selectedKeys.value))
  showBatchDelete.value = false
  selectedKeys.value.clear()
  selectedKey.value = null
  keyValue.value = null
  await loadKeys()
}

function exportKeys() {
  const data = Array.from(selectedKeys.value).map((key) => ({ key }))
  const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `redis-keys-${new Date().toISOString().slice(0, 10)}.json`
  a.click()
  URL.revokeObjectURL(url)
  showExport.value = false
}

function onImportFile(e: Event) {
  const input = e.target as HTMLInputElement
  if (!input.files?.length) return
  const file = input.files[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = async () => {
    try {
      const data = JSON.parse(reader.result as string)
      if (Array.isArray(data)) {
        for (const item of data) {
          if (item.key) {
            await redisApi.setValue(sessionId.value!, item.key, '')
          }
        }
      }
    } catch { /* ignore */ }
    showImport.value = false
    await loadKeys()
  }
  reader.readAsText(file)
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

onBeforeUnmount(async () => {
  document.removeEventListener('mousemove', onDragMove)
  document.removeEventListener('mouseup', onDragEnd)
  if (sessionId.value) {
    try { await redisApi.disconnect(sessionId.value) } catch { /* ignore */ }
  }
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

/* ---- Stream support ---- */
const streamTab = ref<'messages' | 'groups'>('messages')
const streamMessages = ref<{ id: string; fields: [string, string][] }[]>([])
const streamGroups = ref<{ name: string; consumers: number; pending: number; lastDeliveredId: string }[]>([])
const streamMinId = ref('-')
const streamMaxId = ref('+')
const streamLoading = ref(false)

async function loadStreamMessages() {
  if (!sessionId.value || !selectedKey.value) return
  streamLoading.value = true
  try {
    const result = await redisApi.runCommand(selectedKey.value ? sessionId.value : '', [
      'XRANGE', selectedKey.value!, streamMinId.value, streamMaxId.value, 'COUNT', '100',
    ])
    // Parse XRANGE output: each entry is "id" followed by field-value pairs
    const lines = result.split('\n').filter(l => l.trim())
    const messages: { id: string; fields: [string, string][] }[] = []
    let i = 0
    while (i < lines.length) {
      const line = lines[i]?.trim()
      if (!line) { i++; continue }
      // Message ID line (e.g., "1234567890-0")
      if (/^\d+-\d+$/.test(line)) {
        const id = line
        const fields: [string, string][] = []
        i++
        while (i < lines.length && !/^\d+-\d+$/.test(lines[i]!.trim())) {
          // Field-value pair lines come in pairs
          const field = lines[i]?.trim().replace(/^"|"$/g, '')
          const value = lines[i + 1]?.trim().replace(/^"|"$/g, '')
          if (field !== undefined && value !== undefined) {
            fields.push([field, value])
          }
          i += 2
        }
        messages.push({ id, fields })
      } else {
        i++
      }
    }
    streamMessages.value = messages
  } catch {
    streamMessages.value = []
  } finally {
    streamLoading.value = false
  }
}

async function loadStreamGroups() {
  if (!sessionId.value || !selectedKey.value) return
  streamLoading.value = true
  try {
    const result = await redisApi.runCommand(sessionId.value, [
      'XINFO', 'GROUPS', selectedKey.value!,
    ])
    // Parse XINFO GROUPS output
    const lines = result.split('\n').filter(l => l.trim())
    const groups: { name: string; consumers: number; pending: number; lastDeliveredId: string }[] = []
    let i = 0
    while (i < lines.length) {
      const line = lines[i]?.trim()
      if (line === 'name') {
        const name = lines[i + 1]?.trim().replace(/^"|"$/g, '') || ''
        let consumers = 0, pending = 0, lastDeliveredId = '0-0'
        i += 2
        // Parse remaining fields for this group
        while (i < lines.length) {
          const key = lines[i]?.trim()
          if (key === 'name' || key === '') break
          if (key === 'consumers') { consumers = parseInt(lines[i + 1]?.trim() || '0', 10); i += 2 }
          else if (key === 'pending') { pending = parseInt(lines[i + 1]?.trim() || '0', 10); i += 2 }
          else if (key === 'last-delivered-id') { lastDeliveredId = lines[i + 1]?.trim().replace(/^"|"$/g, '') || '0-0'; i += 2 }
          else { i += 2 }
        }
        groups.push({ name, consumers, pending, lastDeliveredId })
      } else {
        i++
      }
    }
    streamGroups.value = groups
  } catch {
    streamGroups.value = []
  } finally {
    streamLoading.value = false
  }
}

function loadStreamData() {
  if (streamTab.value === 'messages') loadStreamMessages()
  else loadStreamGroups()
}

/* ---- Management features ---- */
const showMemoryAnalysis = ref(false)
const showSlowLog = ref(false)
const showFlushDb = ref(false)
const memoryData = ref<{ used: string; peak: string; fragmentation: string; keyTypes: { type: string; count: number }[]; totalKeys: number } | null>(null)
const slowLogEntries = ref<{ id: number; time: string; duration: string; client: string; command: string }[]>([])
const managementLoading = ref(false)

async function openMemoryAnalysis() {
  showMemoryAnalysis.value = true
  managementLoading.value = true
  try {
    // Get INFO memory
    const infoRaw = await redisApi.runCommand(sessionId.value!, ['INFO', 'memory'])
    const parseInfo = (raw: string, key: string) => {
      const m = raw.match(new RegExp(`^${key}:(.+)`, 'm'))
      return m?.[1]?.trim() || ''
    }
    const used = parseInfo(infoRaw, 'used_memory_human')
    const peak = parseInfo(infoRaw, 'used_memory_peak_human')
    const frag = parseInfo(infoRaw, 'mem_fragmentation_ratio')

    // Get key type distribution via SCAN
    const keysRaw = await redisApi.scan(sessionId.value!, '*', 1000)
    const typeMap = new Map<string, number>()
    for (const k of keysRaw) {
      typeMap.set(k.type_name, (typeMap.get(k.type_name) || 0) + 1)
    }
    const keyTypes = Array.from(typeMap.entries()).map(([type, count]) => ({ type, count })).sort((a, b) => b.count - a.count)

    memoryData.value = { used, peak, fragmentation: frag, keyTypes, totalKeys: keysRaw.length }
  } catch {
    memoryData.value = null
  } finally {
    managementLoading.value = false
  }
}

async function openSlowLog() {
  showSlowLog.value = true
  managementLoading.value = true
  try {
    const raw = await redisApi.runCommand(sessionId.value!, ['SLOWLOG', 'GET', '50'])
    // Parse SLOWLOG GET output
    const lines = raw.split('\n').filter(l => l.trim())
    const entries: { id: number; time: string; duration: string; client: string; command: string }[] = []
    let i = 0
    while (i < lines.length) {
      const line = lines[i]?.trim()
      // Entry starts with a number (the slow log entry ID)
      if (line && /^\d+$/.test(line)) {
        const id = parseInt(line, 10)
        const ts = lines[i + 1]?.trim() || ''
        const dur = lines[i + 2]?.trim() || ''
        const client = lines[i + 3]?.trim() || ''
        const cmd = lines.slice(i + 4, i + 8).join(' ').trim()
        entries.push({
          id,
          time: new Date(parseInt(ts, 10) * 1000).toLocaleTimeString(),
          duration: dur,
          client,
          command: cmd,
        })
        i += 8
      } else {
        i++
      }
    }
    slowLogEntries.value = entries
  } catch {
    slowLogEntries.value = []
  } finally {
    managementLoading.value = false
  }
}

async function flushDb() {
  if (!sessionId.value) return
  try {
    await redisApi.runCommand(sessionId.value, ['FLUSHDB'])
    showFlushDb.value = false
    await loadKeys()
  } catch { /* ignore */ }
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

    <!-- Edit Connection Modal -->
    <Teleport to="body">
      <div v-if="showEditConnection && editingConnection" class="modal-overlay" @click.self="showEditConnection = false">
        <div class="modal-content">
          <div class="modal-header">
            <span class="modal-title">Edit Connection</span>
            <button class="modal-close" @click="showEditConnection = false">×</button>
          </div>
          <div class="modal-body">
            <div class="dialog-field">
              <label>Name</label>
              <input v-model="editingConnection.name" class="mono" />
            </div>
            <div class="dialog-field">
              <label>Host</label>
              <input v-model="editingConnection.host" class="mono" />
            </div>
            <div class="dialog-field">
              <label>Port</label>
              <input v-model.number="editingConnection.port" class="mono" type="number" />
            </div>
            <div class="dialog-field">
              <label>Password</label>
              <input v-model="editingConnection.password" class="mono" type="password" />
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn btn-secondary" @click="showEditConnection = false">Cancel</button>
            <button class="btn btn-primary" @click="saveConnection">Save</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Delete Connection Modal -->
    <Teleport to="body">
      <div v-if="showDeleteConnection && deletingConnection" class="modal-overlay" @click.self="showDeleteConnection = false">
        <div class="modal-content">
          <div class="modal-header">
            <span class="modal-title">Delete Connection</span>
            <button class="modal-close" @click="showDeleteConnection = false">×</button>
          </div>
          <div class="modal-body">
            <p>Are you sure you want to delete "{{ deletingConnection.name }}"?</p>
          </div>
          <div class="modal-footer">
            <button class="btn btn-secondary" @click="showDeleteConnection = false">Cancel</button>
            <button class="btn btn-danger" @click="confirmDeleteConnection">Delete</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Left panel: DB selector + key tree -->
    <div class="redis-panel" :style="{ width: panelWidth + 'px' }">
      <!-- Connection list -->
      <div class="redis-connections">
        <div
          v-for="conn in connections"
          :key="conn.id"
          class="redis-conn-item"
          :class="{ 'redis-conn-item--active': connHost === conn.host && connPort === conn.port }"
        >
          <span class="redis-conn-name">{{ conn.name }}</span>
          <div class="redis-conn-actions">
            <button class="redis-toolbar-btn" title="Edit" @click="editConnection(conn)">✏</button>
            <button class="redis-toolbar-btn" title="Copy" @click="copyConnection(conn)">📋</button>
            <button class="redis-toolbar-btn" title="Delete" @click="deleteConnection(conn)">🗑</button>
          </div>
        </div>
      </div>

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
        <button class="redis-toolbar-btn" title="Select All" @click="selectAll">☑</button>
        <button class="redis-toolbar-btn" title="Clear Selection" @click="clearSelection">☐</button>
        <template v-if="selectedKeys.size > 0">
          <button class="redis-toolbar-btn" title="Batch Delete" @click="showBatchDelete = true">🗑</button>
          <button class="redis-toolbar-btn" title="Batch TTL" @click="showBatchTtl = true">⏱</button>
          <button class="redis-toolbar-btn" title="Export" @click="showExport = true">📤</button>
        </template>
        <button class="redis-toolbar-btn" title="Import" @click="showImport = true">📥</button>
        <template v-if="sessionId">
          <div class="redis-toolbar-sep" />
          <button class="redis-toolbar-btn" title="Memory Analysis" @click="openMemoryAnalysis">📊</button>
          <button class="redis-toolbar-btn" title="Slow Log" @click="openSlowLog">📋</button>
          <button class="redis-toolbar-btn redis-toolbar-btn--danger" title="Flush DB" @click="showFlushDb = true">⚠️</button>
        </template>
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
      <!-- Tab bar for open tabs -->
      <div v-if="openTabs.length > 0" class="redis-tabs">
        <div
          v-for="tab in openTabs"
          :key="tab"
          class="redis-tab"
          :class="{ 'redis-tab--active': activeTab === tab }"
          @click="switchTab(tab)"
        >
          <span class="redis-tab-name mono">{{ tab }}</span>
          <button class="redis-tab-close" @click.stop="closeTab(tab)">×</button>
        </div>
      </div>

      <div v-if="!selectedKey && openTabs.length === 0" class="redis-content-placeholder">
        Select a key to view its value
      </div>
      <div v-else class="redis-value-panel">
        <div class="redis-value-header">
          <span class="redis-value-key mono">{{ selectedKey || activeTab }}</span>
          <button class="redis-toolbar-btn" title="Open in new tab" @click="openInNewTab">↗</button>
          <button class="redis-toolbar-btn redis-toolbar-btn--danger" @click="deleteSelected">Delete</button>
        </div>
        <div v-if="valueLoading" class="redis-value-loading">Loading...</div>
        <div v-else-if="keyValue" class="redis-value-body">
          <div class="redis-value-type">Type: {{ keyValue.type }}</div>
          <!-- String -->
          <FormatViewer v-if="keyValue.type === 'String'" :value="String(keyValue.value || '')" />
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
          <!-- Stream -->
          <div v-else-if="keyValue.type === 'Stream'" class="stream-view">
            <div class="stream-tabs">
              <button class="stream-tab" :class="{ 'stream-tab--active': streamTab === 'messages' }" @click="streamTab = 'messages'; loadStreamData()">Messages</button>
              <button class="stream-tab" :class="{ 'stream-tab--active': streamTab === 'groups' }" @click="streamTab = 'groups'; loadStreamData()">Consumer Groups</button>
            </div>
            <div v-if="streamTab === 'messages'" class="stream-filter">
              <label class="muted">Min:</label>
              <input v-model="streamMinId" class="mono stream-input" placeholder="-" />
              <label class="muted">Max:</label>
              <input v-model="streamMaxId" class="mono stream-input" placeholder="+" />
              <button class="redis-toolbar-btn" @click="loadStreamMessages">↻</button>
            </div>
            <div v-if="streamLoading" class="redis-value-loading">Loading...</div>
            <template v-else-if="streamTab === 'messages'">
              <table v-if="streamMessages.length" class="redis-value-table">
                <thead><tr><th>#</th><th>ID</th><th>Fields</th></tr></thead>
                <tbody>
                  <tr v-for="(msg, i) in streamMessages" :key="msg.id">
                    <td class="muted">{{ i + 1 }}</td>
                    <td class="mono">{{ msg.id }}</td>
                    <td class="mono">
                      <span v-for="(f, fi) in msg.fields" :key="fi">
                        <span class="stream-field-key">{{ f[0] }}</span>: {{ f[1] }}<span v-if="fi < msg.fields.length - 1">, </span>
                      </span>
                    </td>
                  </tr>
                </tbody>
              </table>
              <div v-else class="redis-tree-empty">No messages in stream</div>
            </template>
            <template v-else>
              <table v-if="streamGroups.length" class="redis-value-table">
                <thead><tr><th>#</th><th>Group</th><th>Consumers</th><th>Pending</th><th>Last Delivered</th></tr></thead>
                <tbody>
                  <tr v-for="(g, i) in streamGroups" :key="g.name">
                    <td class="muted">{{ i + 1 }}</td>
                    <td class="mono">{{ g.name }}</td>
                    <td>{{ g.consumers }}</td>
                    <td>{{ g.pending }}</td>
                    <td class="mono">{{ g.lastDeliveredId }}</td>
                  </tr>
                </tbody>
              </table>
              <div v-else class="redis-tree-empty">No consumer groups</div>
            </template>
          </div>
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

    <!-- Batch Delete Modal -->
    <Teleport to="body">
      <div v-if="showBatchDelete" class="modal-overlay" @click.self="showBatchDelete = false">
        <div class="modal-content">
          <div class="modal-header">
            <span class="modal-title">Batch Delete</span>
            <button class="modal-close" @click="showBatchDelete = false">×</button>
          </div>
          <div class="modal-body">
            <p>Are you sure you want to delete {{ selectedKeys.size }} selected keys?</p>
          </div>
          <div class="modal-footer">
            <button class="btn btn-secondary" @click="showBatchDelete = false">Cancel</button>
            <button class="btn btn-danger" @click="batchDelete">Delete</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Batch TTL Modal -->
    <Teleport to="body">
      <div v-if="showBatchTtl" class="modal-overlay" @click.self="showBatchTtl = false">
        <div class="modal-content">
          <div class="modal-header">
            <span class="modal-title">Set TTL for {{ selectedKeys.size }} keys</span>
            <button class="modal-close" @click="showBatchTtl = false">×</button>
          </div>
          <div class="modal-body">
            <div class="dialog-field">
              <label>TTL (seconds)</label>
              <input v-model.number="batchTtlValue" type="number" class="mono" />
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn btn-secondary" @click="showBatchTtl = false">Cancel</button>
            <button class="btn btn-primary" @click="batchSetTtl">Apply</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Export Modal -->
    <Teleport to="body">
      <div v-if="showExport" class="modal-overlay" @click.self="showExport = false">
        <div class="modal-content">
          <div class="modal-header">
            <span class="modal-title">Export {{ selectedKeys.size }} keys</span>
            <button class="modal-close" @click="showExport = false">×</button>
          </div>
          <div class="modal-body">
            <p>Export selected keys to JSON file</p>
          </div>
          <div class="modal-footer">
            <button class="btn btn-secondary" @click="showExport = false">Cancel</button>
            <button class="btn btn-primary" @click="exportKeys">Export</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Import Modal -->
    <Teleport to="body">
      <div v-if="showImport" class="modal-overlay" @click.self="showImport = false">
        <div class="modal-content">
          <div class="modal-header">
            <span class="modal-title">Import Keys</span>
            <button class="modal-close" @click="showImport = false">×</button>
          </div>
          <div class="modal-body">
            <p>Select a JSON file with keys to import</p>
            <input type="file" accept=".json" @change="onImportFile" />
          </div>
          <div class="modal-footer">
            <button class="btn btn-secondary" @click="showImport = false">Cancel</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Memory Analysis Modal -->
    <Teleport to="body">
      <div v-if="showMemoryAnalysis" class="modal-overlay" @click.self="showMemoryAnalysis = false">
        <div class="modal-content modal-wide">
          <div class="modal-header">
            <span class="modal-title">Memory Analysis</span>
            <button class="modal-close" @click="showMemoryAnalysis = false">×</button>
          </div>
          <div class="modal-body">
            <div v-if="managementLoading" class="redis-value-loading">Loading...</div>
            <div v-else-if="memoryData">
              <div class="mgmt-stats">
                <div class="mgmt-stat"><span class="mgmt-label">Used</span><span class="mgmt-value mono">{{ memoryData.used }}</span></div>
                <div class="mgmt-stat"><span class="mgmt-label">Peak</span><span class="mgmt-value mono">{{ memoryData.peak }}</span></div>
                <div class="mgmt-stat"><span class="mgmt-label">Fragmentation</span><span class="mgmt-value mono">{{ memoryData.fragmentation }}</span></div>
                <div class="mgmt-stat"><span class="mgmt-label">Total Keys</span><span class="mgmt-value mono">{{ memoryData.totalKeys }}</span></div>
              </div>
              <div class="mgmt-section-title">Key Type Distribution</div>
              <table class="redis-value-table">
                <thead><tr><th>Type</th><th>Count</th><th>%</th></tr></thead>
                <tbody>
                  <tr v-for="kt in memoryData.keyTypes" :key="kt.type">
                    <td><span class="redis-key-type" :class="'type-' + kt.type.toLowerCase()">{{ kt.type[0] }}</span> {{ kt.type }}</td>
                    <td class="mono">{{ kt.count }}</td>
                    <td>{{ memoryData.totalKeys > 0 ? ((kt.count / memoryData.totalKeys) * 100).toFixed(1) : 0 }}%</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div v-else class="redis-tree-empty">Failed to load memory data</div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Slow Log Modal -->
    <Teleport to="body">
      <div v-if="showSlowLog" class="modal-overlay" @click.self="showSlowLog = false">
        <div class="modal-content modal-wide">
          <div class="modal-header">
            <span class="modal-title">Slow Log</span>
            <button class="modal-close" @click="showSlowLog = false">×</button>
          </div>
          <div class="modal-body">
            <div v-if="managementLoading" class="redis-value-loading">Loading...</div>
            <div v-else-if="slowLogEntries.length">
              <table class="redis-value-table">
                <thead><tr><th>#</th><th>Time</th><th>Duration</th><th>Client</th><th>Command</th></tr></thead>
                <tbody>
                  <tr v-for="entry in slowLogEntries" :key="entry.id">
                    <td class="muted">{{ entry.id }}</td>
                    <td>{{ entry.time }}</td>
                    <td class="mono">{{ entry.duration }}</td>
                    <td class="mono">{{ entry.client }}</td>
                    <td class="mono">{{ entry.command }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div v-else class="redis-tree-empty">No slow log entries</div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Flush DB Confirmation -->
    <Teleport to="body">
      <div v-if="showFlushDb" class="modal-overlay" @click.self="showFlushDb = false">
        <div class="modal-content">
          <div class="modal-header">
            <span class="modal-title">Flush Database</span>
            <button class="modal-close" @click="showFlushDb = false">×</button>
          </div>
          <div class="modal-body">
            <p>⚠️ This will permanently delete <strong>ALL</strong> keys in the current database (db{{ currentDb }}).</p>
            <p>This action cannot be undone.</p>
          </div>
          <div class="modal-footer">
            <button class="btn btn-secondary" @click="showFlushDb = false">Cancel</button>
            <button class="btn btn-danger" @click="flushDb">Flush DB</button>
          </div>
        </div>
      </div>
    </Teleport>
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

/* ---- connections ---- */
.redis-connections {
  border-bottom: 1px solid var(--border);
  max-height: 150px;
  overflow-y: auto;
}

.redis-conn-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
  cursor: pointer;
}

.redis-conn-item:hover {
  background: var(--bg-hover);
}

.redis-conn-item--active {
  background: rgba(232, 145, 45, 0.1);
  border-left: 2px solid var(--accent);
}

.redis-conn-name {
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.redis-conn-actions {
  display: flex;
  gap: var(--space-1);
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
.type-stream { background: rgba(210,153,34,0.2); color: #D29922; }

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
  display: flex;
  flex-direction: column;
}

/* ---- tabs ---- */
.redis-tabs {
  display: flex;
  gap: 2px;
  padding: var(--space-1);
  background: var(--bg-elevated);
  border-bottom: 1px solid var(--border);
}

.redis-tab {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-2);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--text-xs);
}

.redis-tab:hover {
  background: var(--bg-hover);
}

.redis-tab--active {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}

.redis-tab-name {
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.redis-tab-close {
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
  padding: 0;
  font-size: var(--text-sm);
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

/* ---- stream viewer ---- */
.stream-view {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.stream-tabs {
  display: flex;
  gap: 2px;
  border-bottom: 1px solid var(--border);
}

.stream-tab {
  padding: var(--space-1) var(--space-3);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-xs);
  transition: all var(--transition);
}

.stream-tab:hover {
  color: var(--text-primary);
}

.stream-tab--active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.stream-filter {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-xs);
}

.stream-input {
  width: 120px;
  padding: var(--space-1) var(--space-2);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-xs);
  outline: none;
}

.stream-input:focus {
  border-color: var(--accent);
}

.stream-field-key {
  color: var(--accent);
  font-weight: 500;
}

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

/* ---- modals ---- */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  min-width: 320px;
  max-width: 90vw;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}

.modal-title {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
}

.modal-close {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-xl);
}

.modal-close:hover {
  color: var(--text-primary);
}

.modal-body {
  padding: var(--space-4);
  color: var(--text-secondary);
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-top: 1px solid var(--border);
}

.btn {
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  cursor: pointer;
  border: 1px solid var(--border);
  background: var(--bg-surface);
  color: var(--text-primary);
}

.btn:hover {
  background: var(--bg-hover);
}

.btn-primary {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}

.btn-primary:hover {
  opacity: 0.9;
}

.btn-danger {
  background: var(--danger);
  border-color: var(--danger);
  color: white;
}

.btn-danger:hover {
  opacity: 0.9;
}

/* ---- management modals ---- */
.modal-wide {
  min-width: 500px;
  max-width: 700px;
}

.mgmt-stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--space-3);
  margin-bottom: var(--space-4);
}

.mgmt-stat {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: var(--space-3);
  background: var(--bg-deep);
  border-radius: var(--radius);
  border: 1px solid var(--border);
}

.mgmt-label {
  font-size: var(--text-xs);
  color: var(--text-muted);
  text-transform: uppercase;
}

.mgmt-value {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
}

.mgmt-section-title {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: var(--space-2);
}

.redis-toolbar-sep {
  width: 1px;
  height: 14px;
  background: var(--border);
}
</style>
