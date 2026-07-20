<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useSqlNav } from './useSqlNav'
import SqlNavTree from './SqlNavTree.vue'
import SqlEditor from './SqlEditor.vue'
import SqlResultGrid from './SqlResultGrid.vue'
import TableDesigner from './TableDesigner.vue'
import ExportWizard from './ExportWizard.vue'
import GlobalQueryModal from './GlobalQueryModal.vue'
import AiAssistantDrawer from './AiAssistantDrawer.vue'
import ImportWizard from './ImportWizard.vue'
import SqlFormView from './SqlFormView.vue'
import { useSqlQuery, type ExecuteMode } from './useSqlQuery'
import { connect as sqlConnect, disconnect as sqlDisconnect, getDdl, type ConnectRequest, type QueryResult } from '@/api/sql'

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

interface DesignerTab {
  id: number
  type: 'designer'
  title: string
  db: string
  table: string
}

type AnyTab = QueryTab | DesignerTab

let nextTabId = 1
const tabs = ref<AnyTab[]>([])
const activeTabId = ref<number | null>(null)

function isQueryTab(tab: AnyTab): tab is QueryTab {
  return !('type' in tab)
}

function isDesignerTab(tab: AnyTab): tab is DesignerTab {
  return 'type' in tab && tab.type === 'designer'
}

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

function createDesignerTab(db: string, table: string): DesignerTab {
  // Check if already open
  const existing = tabs.value.find(t => isDesignerTab(t) && t.db === db && t.table === table)
  if (existing) {
    activeTabId.value = existing.id
    return existing as DesignerTab
  }
  const tab: DesignerTab = {
    id: nextTabId++,
    type: 'designer',
    title: `${table}`,
    db,
    table,
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
  const existing = tabs.value.find((t) => isQueryTab(t) && t.title === table)
  if (existing) {
    activeTabId.value = existing.id
    return
  }
  createTab(sql)
}

function onDesignTable(db: string, table: string) {
  createDesignerTab(db, table)
}

// DDL Drawer state
const ddlDrawer = ref<{ open: boolean; db: string; table: string; ddl: string }>({
  open: false, db: '', table: '', ddl: '',
})

function copyDdl() {
  navigator.clipboard?.writeText(ddlDrawer.value.ddl)
}

async function onViewDdl(db: string, table: string) {
  if (!sessionId.value) return
  try {
    const result = await getDdl(sessionId.value, db, table)
    ddlDrawer.value = { open: true, db, table, ddl: result.ddl }
  } catch (e: unknown) {
    ddlDrawer.value = { open: true, db, table, ddl: `Error: ${e instanceof Error ? e.message : String(e)}` }
  }
}

const { mode: execMode, run: runQuery } = useSqlQuery(() => sessionId.value)

/* ---- editor toolbar ---- */
const editorRef = ref<InstanceType<typeof SqlEditor>>()
const showClipboard = ref(false)

function onFormat() { editorRef.value?.format() }
function onToggleComment() { editorRef.value?.toggleComment() }
function onToggleCase() { editorRef.value?.toggleCase() }
function onZoomIn() { editorRef.value?.zoomIn() }
function onZoomOut() { editorRef.value?.zoomOut() }
function onZoomReset() { editorRef.value?.zoomReset() }
function toggleClipboard() { showClipboard.value = !showClipboard.value }
function onPasteFromHistory(item: string) {
  editorRef.value?.pasteFromHistory(item)
  showClipboard.value = false
}

async function onExecute(sql: string) {
  const tab = activeTab.value
  if (!tab || !isQueryTab(tab)) return
  await runQuery(sql, tab)
}

function onSave(sql: string) {
  console.log('save', sql)
}

const activeTab = computed(() => tabs.value.find((t) => t.id === activeTabId.value))
const activeQueryTab = computed(() => {
  const tab = activeTab.value
  return tab && isQueryTab(tab) ? tab : null
})

// Export wizard state
const showExport = ref(false)

// Global query modal state
const showGlobalQuery = ref(false)

function openGlobalQuery() {
  if (databases.value.length > 0) {
    showGlobalQuery.value = true
  }
}

// AI Assistant drawer state
const showAiAssistant = ref(false)

function openAiAssistant() {
  showAiAssistant.value = true
}

// Import wizard state
const showImport = ref(false)
const importTarget = ref<{ db: string; table: string }>({ db: '', table: '' })

function openImport(db: string, table: string) {
  importTarget.value = { db, table }
  showImport.value = true
}

// Form view state
const viewMode = ref<'grid' | 'form'>('grid')
const formRowIndex = ref(0)

function onGlobalQueryExecute(results: { db: string; result: QueryResult | null; error: string | null }[]) {
  // Create a new query tab with combined results
  const combinedResult: QueryResult = {
    columns: results[0]?.result?.columns || [],
    rows: results.flatMap((r) => r.result?.rows || []),
    affected_rows: 0,
    elapsed_ms: 0,
  }
  createTab(`-- Global Query: ${results.map((r) => r.db).join(', ')}\n${activeQueryTab.value?.sql || ''}`)
  const tab = tabs.value[tabs.value.length - 1]
  if (tab && isQueryTab(tab)) {
    tab.result = combinedResult
  }
}

function onInsertAiSql(sql: string) {
  const tab = activeTab.value
  if (tab && isQueryTab(tab)) {
    tab.sql = sql
    tab.dirty = true
  }
}

function onImported() {
  // Refresh the nav tree to show updated data
  loadDatabases()
}

import type { EditCell } from './SqlResultGrid.vue'

async function onApplyChanges(changes: EditCell[]) {
  if (!sessionId.value || !activeQueryTab.value) return
  // Generate UPDATE statements based on changes
  // This is a simplified version - in production, you'd need to know the table name and primary key
  console.log('Apply changes:', changes)
}

// Keyboard shortcuts
function handleKeydown(e: KeyboardEvent) {
  // Ctrl+Shift+Q: Global Query
  if (e.ctrlKey && e.shiftKey && e.key === 'Q') {
    e.preventDefault()
    openGlobalQuery()
  }
  // Ctrl+Shift+A: AI Assistant
  if (e.ctrlKey && e.shiftKey && e.key === 'A') {
    e.preventDefault()
    openAiAssistant()
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleKeydown)
})
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
        @design-table="onDesignTable"
        @view-ddl="onViewDdl"
        @import-data="openImport"
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
            <span v-if="isDesignerTab(tab)" class="sql-tab-icon">⊞</span>
            <span class="sql-tab-title">{{ tab.title }}</span>
            <span class="sql-tab-close" @click.stop="closeTab(tab.id)">×</span>
          </div>
          <button class="sql-tab-add" @click="createTab()" title="New Query">+</button>
        </div>
        <div v-if="activeQueryTab" class="sql-toolbar">
          <select v-model="execMode" class="sql-toolbar-select mono" title="Execute mode">
            <option value="all">Run All</option>
            <option value="current">Run Current</option>
            <option value="selected">Run Selected</option>
          </select>
          <button
            class="sql-toolbar-btn sql-run-btn"
            title="Execute (Ctrl+Enter)"
            :disabled="!activeQueryTab || activeQueryTab.loading"
            @click="activeQueryTab && onExecute(activeQueryTab.sql)"
          >
            ▶ Run
          </button>
          <div class="sql-toolbar-sep" />
          <button class="sql-toolbar-btn" title="Format SQL (Ctrl+Shift+F)" @click="onFormat">✦ Format</button>
          <button class="sql-toolbar-btn" title="Toggle Comment (Ctrl+/)" @click="onToggleComment">💬</button>
          <button class="sql-toolbar-btn" title="Toggle Case (Ctrl+Shift+U)" @click="onToggleCase">Aa</button>
          <div class="sql-toolbar-sep" />
          <div class="sql-clipboard-wrap">
            <button class="sql-toolbar-btn" title="Clipboard History (Ctrl+Shift+V)" @click="toggleClipboard">📋</button>
            <div v-if="showClipboard" class="sql-clipboard-popup">
              <div v-if="editorRef?.clipboardHistory?.length" class="sql-clipboard-list">
                <div
                  v-for="(item, i) in editorRef.clipboardHistory"
                  :key="i"
                  class="sql-clipboard-item"
                  :title="item"
                  @click="onPasteFromHistory(item)"
                >
                  {{ item.length > 60 ? item.slice(0, 60) + '…' : item }}
                </div>
              </div>
              <div v-else class="sql-clipboard-empty">No clipboard history</div>
            </div>
          </div>
          <div class="sql-toolbar-sep" />
          <button class="sql-toolbar-btn sql-zoom-btn" title="Zoom In (Ctrl+=)" @click="onZoomIn">+</button>
          <button class="sql-toolbar-btn sql-zoom-btn" title="Zoom Out (Ctrl+-)" @click="onZoomOut">−</button>
          <button class="sql-toolbar-btn sql-zoom-btn" title="Reset Zoom (Ctrl+0)" @click="onZoomReset">1:1</button>
        </div>
      </div>

      <!-- Split: editor top / result bottom (query tabs) -->
      <div v-if="activeQueryTab" class="sql-right-split">
        <!-- Editor -->
        <div class="sql-split-editor" :style="{ height: editorHeight + '%' }">
          <SqlEditor
            ref="editorRef"
            :key="activeQueryTab.id"
            :model-value="activeQueryTab.sql"
            @update:model-value="activeQueryTab.sql = $event; activeQueryTab.dirty = true"
            @execute="onExecute"
            @save="onSave"
          />
        </div>

        <!-- Vertical resize handle -->
        <div
          class="sql-vhandle"
          :class="{ 'sql-vhandle--active': vDragging }"
          @mousedown.prevent="onVDragStart"
        />

        <!-- Result grid -->
        <div class="sql-split-result">
          <!-- View toggle -->
          <div v-if="activeQueryTab.result && activeQueryTab.result.rows.length > 0" class="view-toggle">
            <button
              class="view-btn"
              :class="{ 'view-btn--active': viewMode === 'grid' }"
              title="Grid View"
              @click="viewMode = 'grid'"
            >
              ⊞
            </button>
            <button
              class="view-btn"
              :class="{ 'view-btn--active': viewMode === 'form' }"
              title="Form View"
              @click="viewMode = 'form'"
            >
              ☰
            </button>
          </div>

          <!-- Grid view -->
          <SqlResultGrid
            v-if="viewMode === 'grid'"
            :result="activeQueryTab.result"
            :loading="activeQueryTab.loading"
            :error="activeQueryTab.error"
            @export="showExport = true"
            @apply="onApplyChanges"
          />

          <!-- Form view -->
          <SqlFormView
            v-else-if="viewMode === 'form' && activeQueryTab.result"
            :columns="activeQueryTab.result.columns"
            :rows="activeQueryTab.result.rows"
            :current-index="formRowIndex"
            @update:current-index="formRowIndex = $event"
          />
        </div>
      </div>

      <!-- Table Designer (designer tabs) -->
      <div v-else-if="activeTab && isDesignerTab(activeTab)" class="sql-designer-wrap">
        <TableDesigner
          :key="activeTab.id"
          :session-id="sessionId || ''"
          :db="activeTab.db"
          :table="activeTab.table"
          @close="closeTab(activeTab.id)"
        />
      </div>

      <!-- Placeholder (no tab) -->
      <div v-else class="sql-page-placeholder">
        <div class="placeholder-title">SQL Console</div>
        <div class="placeholder-desc">
          Select a table or click + to create a new query
        </div>
      </div>

      <!-- DDL Drawer -->
      <div v-if="ddlDrawer.open" class="sql-ddl-drawer">
        <div class="sql-ddl-drawer-header">
          <span class="sql-ddl-drawer-title mono">DDL: {{ ddlDrawer.table }}</span>
          <div class="sql-ddl-drawer-actions">
            <button class="sql-ddl-btn" @click="copyDdl" title="Copy DDL">Copy</button>
            <button class="sql-ddl-btn" @click="ddlDrawer.open = false" title="Close">×</button>
          </div>
        </div>
        <pre class="sql-ddl-drawer-content mono">{{ ddlDrawer.ddl }}</pre>
      </div>

      <!-- Export Wizard -->
      <ExportWizard
        v-if="showExport && activeQueryTab?.result"
        :result="activeQueryTab.result"
        :table-name="activeQueryTab.title"
        @close="showExport = false"
      />

      <!-- Global Query Modal -->
      <GlobalQueryModal
        :visible="showGlobalQuery"
        :session-id="sessionId || ''"
        :databases="databases.map(db => db.name)"
        @close="showGlobalQuery = false"
        @execute="onGlobalQueryExecute"
      />

      <!-- AI Assistant Drawer -->
      <AiAssistantDrawer
        :visible="showAiAssistant"
        :session-id="sessionId || ''"
        :db="databases[0]?.name || ''"
        :table="undefined"
        :query="activeQueryTab?.sql"
        @close="showAiAssistant = false"
        @insert-sql="onInsertAiSql"
      />

      <!-- Import Wizard -->
      <ImportWizard
        :visible="showImport"
        :session-id="sessionId || ''"
        :db="importTarget.db"
        :table="importTarget.table"
        @close="showImport = false"
        @imported="onImported"
      />
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

.sql-tab-icon {
  font-size: var(--text-xs);
  color: var(--accent);
  margin-right: var(--space-1);
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

.sql-toolbar-sep {
  width: 1px;
  height: 16px;
  background: var(--border);
  margin: 0 var(--space-1);
}

.sql-zoom-btn {
  padding: var(--space-1) var(--space-2) !important;
  font-weight: 600;
  min-width: 24px;
  text-align: center;
}

/* ---- clipboard history popup ---- */
.sql-clipboard-wrap {
  position: relative;
}

.sql-clipboard-popup {
  position: absolute;
  top: 100%;
  right: 0;
  z-index: 100;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  min-width: 300px;
  max-width: 400px;
  max-height: 240px;
  overflow-y: auto;
}

.sql-clipboard-list {
  padding: var(--space-1) 0;
}

.sql-clipboard-item {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-xs);
  font-family: var(--font-mono);
  color: var(--text-primary);
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: background var(--transition);
}

.sql-clipboard-item:hover {
  background: var(--bg-hover);
}

.sql-clipboard-empty {
  padding: var(--space-3);
  text-align: center;
  color: var(--text-muted);
  font-size: var(--text-xs);
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

/* ---- designer tab ---- */
.sql-designer-wrap {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

/* ---- DDL drawer ---- */
.sql-ddl-drawer {
  border-top: 1px solid var(--border);
  background: var(--bg-deep);
  flex-shrink: 0;
  max-height: 200px;
  display: flex;
  flex-direction: column;
}

.sql-ddl-drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-1) var(--space-3);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.sql-ddl-drawer-title {
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.sql-ddl-drawer-actions {
  display: flex;
  gap: var(--space-2);
}

.sql-ddl-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-xs);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  transition: color var(--transition);
}

.sql-ddl-btn:hover {
  color: var(--text-primary);
}

.sql-ddl-drawer-content {
  flex: 1;
  overflow: auto;
  padding: var(--space-2) var(--space-3);
  margin: 0;
  font-size: var(--text-sm);
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}

/* ---- view toggle ---- */
.view-toggle {
  display: flex;
  gap: 2px;
  padding: var(--space-1);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
}

.view-btn {
  padding: var(--space-1) var(--space-2);
  background: none;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-sm);
  transition: all var(--transition);
}

.view-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.view-btn--active {
  color: var(--accent);
  background: rgba(232, 145, 45, 0.1);
}
</style>
