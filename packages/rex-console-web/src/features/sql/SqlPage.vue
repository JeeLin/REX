<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useSqlNav } from './useSqlNav'
import SqlNavTree from './SqlNavTree.vue'
import SqlEditor from './SqlEditor.vue'
import SqlResultGrid from './SqlResultGrid.vue'
import { useSqlQuery, type ExecuteMode } from './useSqlQuery'
import { connect as sqlConnect, disconnect as sqlDisconnect, type ConnectRequest } from '@/api/sql'
import type { QueryResult } from '@/api/sql'

const props = defineProps<{
  resourceId?: string
  host?: string
  port?: number
  username?: string
  password?: string
  database?: string
  dbType?: string
  protocol?: string
}>()

const sessionId = ref<string | null>(null)
const connectError = ref<string | null>(null)
const { databases, loading, searchQuery, loadDatabases } = useSqlNav(sessionId)

// Auto-connect on mount if props provided
onMounted(async () => {
  if (props.host) {
    try {
      const req: ConnectRequest = {
        type: props.dbType || props.protocol || 'mysql',
        host: props.host,
        port: props.port || 3306,
        username: props.username || 'root',
        password: props.password,
        database: props.database,
      }
      sessionId.value = await sqlConnect(req)
    } catch (e: unknown) {
      connectError.value = e instanceof Error ? e.message : String(e)
    }
  }
})

// Disconnect on unmount
onBeforeUnmount(async () => {
  if (sessionId.value) {
    try {
      await sqlDisconnect(sessionId.value)
    } catch {
      // ignore
    }
  }
})

/* ---- resizable panel ---- */
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
  const delta = e.clientX - startX
  panelWidth.value = Math.min(400, Math.max(200, startW + delta))
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

/* ---- vertical split (editor / result) ---- */
const editorHeight = ref(50) // percent
const vDragging = ref(false)
let vStartY = 0
let vStartH = 0

function onVDragStart(e: MouseEvent) {
  vDragging.value = true
  vStartY = e.clientY
  vStartH = editorHeight.value
  document.addEventListener('mousemove', onVDragMove)
  document.addEventListener('mouseup', onVDragEnd)
  document.body.style.cursor = 'row-resize'
  document.body.style.userSelect = 'none'
}

function onVDragMove(e: MouseEvent) {
  const container = document.querySelector('.sql-right-split') as HTMLElement
  if (!container) return
  const rect = container.getBoundingClientRect()
  const pct = ((e.clientY - rect.top) / rect.height) * 100
  editorHeight.value = Math.min(80, Math.max(20, pct))
}

function onVDragEnd() {
  vDragging.value = false
  document.removeEventListener('mousemove', onVDragMove)
  document.removeEventListener('mouseup', onVDragEnd)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}

/* ---- query tabs ---- */
interface QueryTab {
  id: number
  title: string
  sql: string
  dirty: boolean
  result: QueryResult | null
  loading: boolean
  error: string | null
}

let nextTabId = 1
const tabs = ref<QueryTab[]>([])
const activeTabId = ref<number | null>(null)

function createTab(initialSql = ''): QueryTab {
  const tab: QueryTab = {
    id: nextTabId++,
    title: `Query ${tabs.value.length + 1}`,
    sql: initialSql,
    dirty: false,
    result: null,
    loading: false,
    error: null,
  }
  tabs.value.push(tab)
  activeTabId.value = tab.id
  return tab
}

function closeTab(id: number) {
  const idx = tabs.value.findIndex((t) => t.id === id)
  if (idx === -1) return
  tabs.value.splice(idx, 1)
  if (activeTabId.value === id) {
    activeTabId.value = tabs.value[Math.min(idx, tabs.value.length - 1)]?.id ?? null
  }
}

function onSelectTable(db: string, table: string) {
  const sql = `SELECT * FROM "${db}"."${table}" LIMIT 100`
  const existing = tabs.value.find((t) => t.title === table)
  if (existing) {
    activeTabId.value = existing.id
    return
  }
  createTab(sql)
}

const { mode: execMode, run: runQuery } = useSqlQuery(() => sessionId.value)

async function onExecute(sql: string) {
  const tab = activeTab.value
  if (!tab) return
  await runQuery(sql, tab)
}

function onSave(sql: string) {
  console.log('save', sql)
}

const activeTab = computed(() => tabs.value.find((t) => t.id === activeTabId.value))
</script>

<template>
  <div class="sql-page" @mousemove.prevent>
    <!-- Left panel: nav tree -->
    <div class="sql-page-panel" :style="{ width: panelWidth + 'px' }">
      <SqlNavTree
        :databases="databases"
        :loading="loading"
        :search-query="searchQuery"
        @select-table="onSelectTable"
        @refresh="loadDatabases"
        @update:search-query="(v: string) => (searchQuery = v)"
      />
    </div>

    <!-- Resize handle -->
    <div
      class="sql-page-handle"
      :class="{ 'sql-page-handle--active': dragging }"
      @mousedown.prevent="onDragStart"
    />

    <!-- Right panel: tabs + editor + result -->
    <div class="sql-page-content">
      <!-- Tab bar + toolbar -->
      <div class="sql-tab-bar">
        <div class="sql-tabs">
          <div
            v-for="tab in tabs"
            :key="tab.id"
            class="sql-tab"
            :class="{ 'sql-tab--active': tab.id === activeTabId }"
            @click="activeTabId = tab.id"
          >
            <span class="sql-tab-title">{{ tab.title }}</span>
            <span class="sql-tab-close" @click.stop="closeTab(tab.id)">×</span>
          </div>
          <button class="sql-tab-add" @click="createTab()" title="New Query">+</button>
        </div>
        <div class="sql-toolbar">
          <select v-model="execMode" class="sql-toolbar-select mono" title="Execute mode">
            <option value="all">Run All</option>
            <option value="current">Run Current</option>
            <option value="selected">Run Selected</option>
          </select>
          <button
            class="sql-toolbar-btn sql-run-btn"
            title="Execute (Ctrl+Enter)"
            :disabled="!activeTab || activeTab.loading"
            @click="activeTab && onExecute(activeTab.sql)"
          >
            ▶ Run
          </button>
        </div>
      </div>

      <!-- Split: editor top / result bottom -->
      <div class="sql-right-split">
        <!-- Editor -->
        <div class="sql-split-editor" :style="{ height: editorHeight + '%' }">
          <SqlEditor
            v-if="activeTab"
            :key="activeTab.id"
            :model-value="activeTab.sql"
            @update:model-value="activeTab.sql = $event; activeTab.dirty = true"
            @execute="onExecute"
            @save="onSave"
          />
          <div v-else class="sql-page-placeholder">
            <div class="placeholder-title">SQL Console</div>
            <div class="placeholder-desc">
              Select a table or click + to create a new query
            </div>
          </div>
        </div>

        <!-- Vertical resize handle -->
        <div
          class="sql-vhandle"
          :class="{ 'sql-vhandle--active': vDragging }"
          @mousedown.prevent="onVDragStart"
        />

        <!-- Result grid -->
        <div class="sql-split-result">
          <SqlResultGrid
            v-if="activeTab"
            :result="activeTab.result"
            :loading="activeTab.loading"
            :error="activeTab.error"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sql-page {
  display: flex;
  height: 100%;
  overflow: hidden;
  background: var(--bg-page);
}

.sql-page-panel {
  flex-shrink: 0;
  height: 100%;
  overflow: hidden;
}

.sql-page-handle {
  width: 4px;
  flex-shrink: 0;
  cursor: col-resize;
  background: var(--border);
  transition: background var(--transition);
}

.sql-page-handle:hover,
.sql-page-handle--active {
  background: var(--accent);
}

.sql-page-content {
  flex: 1;
  min-width: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* ---- tab bar + toolbar ---- */
.sql-tab-bar {
  display: flex;
  flex-direction: column;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.sql-tabs {
  display: flex;
  align-items: center;
  overflow-x: auto;
}

.sql-tab {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-muted);
  cursor: pointer;
  border-right: 1px solid var(--border);
  white-space: nowrap;
  transition: color var(--transition), background var(--transition);
}

.sql-tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.sql-tab--active {
  color: var(--text-primary);
  background: var(--bg-deep);
  border-bottom: 2px solid var(--accent);
}

.sql-tab-title {
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sql-tab-close {
  font-size: var(--text-xs);
  color: var(--text-muted);
  border-radius: var(--radius-sm);
  padding: 1px 4px;
  transition: color var(--transition), background var(--transition);
}

.sql-tab-close:hover {
  color: var(--danger);
  background: rgba(248, 81, 73, 0.15);
}

.sql-tab-add {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-md);
  padding: var(--space-2) var(--space-3);
  transition: color var(--transition);
}

.sql-tab-add:hover {
  color: var(--accent);
}

/* ---- toolbar ---- */
.sql-toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  border-top: 1px solid var(--border);
}

.sql-toolbar-select {
  padding: var(--space-1) var(--space-2);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-xs);
  outline: none;
  cursor: pointer;
}

.sql-toolbar-select:focus {
  border-color: var(--accent);
}

.sql-toolbar-btn {
  padding: var(--space-1) var(--space-3);
  border: none;
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: background var(--transition), opacity var(--transition);
}

.sql-toolbar-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.sql-run-btn {
  background: var(--accent);
  color: #fff;
}

.sql-run-btn:hover:not(:disabled) {
  background: #d6820f;
}

/* ---- split ---- */
.sql-right-split {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.sql-split-editor {
  min-height: 80px;
  overflow: hidden;
}

.sql-vhandle {
  height: 4px;
  flex-shrink: 0;
  cursor: row-resize;
  background: var(--border);
  transition: background var(--transition);
}

.sql-vhandle:hover,
.sql-vhandle--active {
  background: var(--accent);
}

.sql-split-result {
  flex: 1;
  min-height: 80px;
  overflow: hidden;
}

.sql-page-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-muted);
  gap: var(--space-2);
}

.placeholder-title {
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.placeholder-desc {
  font-size: var(--text-sm);
  text-align: center;
  max-width: 300px;
}
</style>
