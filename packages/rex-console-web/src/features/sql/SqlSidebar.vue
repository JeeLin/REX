<template>
  <div class="sql-sidebar" :style="{ width: sidebarWidth + 'px' }">
    <!-- 数据库选择器（SQLite 隐藏） -->
    <div v-if="!isSqlite" class="sql-sidebar-db">
      <select class="db-select" :value="database" @change="$emit('update:database', ($event.target as HTMLSelectElement).value)">
        <option v-for="db in databases" :key="db.name" :value="db.name">{{ db.name }}</option>
      </select>
      <button class="btn btn-ghost btn-xs" @click="$emit('refresh')">↻</button>
    </div>

    <!-- 库表结构树 -->
    <div class="sql-sidebar-schema">
      <div class="sql-sidebar-search">
        <input v-model="search" type="text" :placeholder="t('sql.searchPlaceholder')" />
      </div>
      <div class="sql-tree" @contextmenu.prevent="handleTreeContextMenu">
        <div v-for="table in filteredTables" :key="table.name" class="tree-group">
          <div class="tree-group-header" @click="toggleTable(table.name)" @contextmenu.prevent="handleTableContextMenu($event, table)">
            <span class="tree-icon">{{ expanded.has(table.name) ? '▾' : '▸' }}</span>
            <span>📊</span>
            <span>{{ table.name }}</span>
            <span v-if="table.row_count != null" class="tree-count">{{ table.row_count.toLocaleString() }}</span>
          </div>
          <div v-if="expanded.has(table.name)" class="tree-children">
            <div v-for="col in columns.get(table.name)" :key="col.name" class="tree-col-item" @contextmenu.prevent="handleColumnContextMenu($event, col)">
              <span v-if="col.is_primary_key" class="col-key">PK</span>
              <span v-else class="col-key" style="visibility:hidden">_</span>
              <span class="col-name">{{ col.name }}</span>
              <span class="col-type">{{ col.data_type }}</span>
            </div>
          </div>
        </div>
        <template v-if="!isSqlite && filteredViews.length > 0">
          <div class="tree-section-label">{{ t('sql.viewLabel') }}</div>
          <div v-for="view in filteredViews" :key="view.name" class="tree-group">
            <div class="tree-group-header" @click="toggleView(view.name)" @contextmenu.prevent="handleViewContextMenu($event, view)">
              <span class="tree-icon">{{ viewExpanded.has(view.name) ? '▾' : '▸' }}</span>
              <span>📐</span>
              <span>{{ view.name }}</span>
            </div>
            <div v-if="viewExpanded.has(view.name)" class="tree-children">
              <div v-for="col in viewColumns.get(view.name)" :key="col.name" class="tree-col-item">
                <span v-if="col.is_primary_key" class="col-key">PK</span>
                <span v-else class="col-key" style="visibility:hidden">_</span>
                <span class="col-name">{{ col.name }}</span>
                <span class="col-type">{{ col.data_type }}</span>
              </div>
            </div>
          </div>
        </template>
        <template v-if="!isSqlite && filteredProcedures.length > 0">
          <div class="tree-section-label">{{ t('sql.procedureLabel') }}</div>
          <div v-for="proc in filteredProcedures" :key="proc.name" class="tree-group">
            <div class="tree-group-header" @contextmenu.prevent="handleProcedureContextMenu($event, proc)">
              <span class="tree-icon" style="visibility:hidden">▸</span>
              <span>🔧</span>
              <span>{{ proc.name }}</span>
              <span class="col-type" style="font-size:var(--fs-xs);margin-left:auto">{{ proc.type }}</span>
            </div>
          </div>
        </template>
      </div>
    </div>

    <!-- 查询文件（底部可折叠区域） -->
    <div class="sql-sidebar-queries">
      <div class="queries-header" @click="queriesExpanded = !queriesExpanded">
        <span class="tree-icon">{{ queriesExpanded ? '▾' : '▸' }}</span>
        <span>{{ t('sql.sidebar.savedQueries') }}</span>
        <span class="queries-count">{{ queries.length }}</span>
      </div>
      <template v-if="queriesExpanded">
        <div class="sql-sidebar-search">
          <input v-model="querySearch" type="text" :placeholder="t('sql.sidebar.searchQueries')" />
        </div>
        <div class="sql-tree queries-list">
          <div
            v-for="q in filteredQueries"
            :key="q.id"
            class="tree-query-item"
            @click="$emit('open-query', q)"
            @contextmenu.prevent="handleQueryContextMenu($event, q)"
          >
            <span class="query-icon">📄</span>
            <div class="query-info">
              <span class="query-name">{{ q.name }}</span>
              <span class="query-meta">{{ q.database }} · {{ formatDate(q.updated_at) }}</span>
            </div>
          </div>
          <div v-if="filteredQueries.length === 0" class="tree-empty">
            {{ t('sql.sidebar.noQueries') }}
          </div>
        </div>
      </template>
    </div>

    <!-- 拖拽调整宽度 -->
    <div class="sidebar-resize" @mousedown.prevent="startResize" />

    <ConfirmDialog
      :visible="showDeleteConfirm"
      :title="t('confirm.deleteTitle')"
      :message="deleteConfirmMsg"
      :confirm-label="t('common.delete')"
      :cancel-label="t('common.cancel')"
      danger
      @confirm="doDeleteQuery"
      @cancel="showDeleteConfirm = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'
import { useToast } from '@/composables/useToast'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import { listTables, listColumns, listViews, listProcedures, listQueries, deleteQuery, renameQuery, getDdl } from '@/api/sql'
import type { TableInfo, ColumnInfo, ViewInfo, ProcedureInfo, QueryFileMeta, DatabaseInfo } from '@/api/sql'

const { t } = useI18n()
const ctxMenu = useContextMenu()
const toast = useToast()

const props = defineProps<{
  resourceId: string
  database: string
  databases: DatabaseInfo[]
  protocol: string
}>()

const emit = defineEmits<{
  'update:database': [db: string]
  'select-table': [table: string]
  'export-table': [table: string]
  'open-query': [query: QueryFileMeta]
  'refresh': []
  'query-deleted': []
  'query-renamed': []
  'open-sql-tab': [title: string, sql: string]
}>()

// SQLite 协议检测
const isSqlite = computed(() => props.protocol.toLowerCase() === 'sqlite')

const search = ref('')
const querySearch = ref('')
const tables = ref<TableInfo[]>([])
const views = ref<ViewInfo[]>([])
const viewColumns = ref<Map<string, ColumnInfo[]>>(new Map())
const viewExpanded = ref<Set<string>>(new Set())
const procedures = ref<ProcedureInfo[]>([])
const columns = ref<Map<string, ColumnInfo[]>>(new Map())
const expanded = ref<Set<string>>(new Set())
const queries = ref<QueryFileMeta[]>([])
const queriesExpanded = ref(true)
const showDeleteConfirm = ref(false)
const deleteConfirmMsg = ref('')
let pendingDeleteResourceId = ''
let pendingDeleteQueryId = ''

// 侧边栏宽度拖拽
const SIDEBAR_WIDTH_KEY = 'rex-sql-sidebar-width'
const sidebarWidth = ref(parseInt(localStorage.getItem(SIDEBAR_WIDTH_KEY) || '260'))
let resizing = false
let startX = 0
let startWidth = 0

function startResize(e: MouseEvent) {
  resizing = true
  startX = e.clientX
  startWidth = sidebarWidth.value
  document.addEventListener('mousemove', onResize)
  document.addEventListener('mouseup', stopResize)
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
}

function onResize(e: MouseEvent) {
  if (!resizing) return
  const delta = e.clientX - startX
  sidebarWidth.value = Math.min(400, Math.max(200, startWidth + delta))
}

function stopResize() {
  resizing = false
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  localStorage.setItem(SIDEBAR_WIDTH_KEY, String(sidebarWidth.value))
}

const filteredTables = computed(() => {
  if (!search.value) return tables.value
  const q = search.value.toLowerCase()
  return tables.value.filter(t => t.name.toLowerCase().includes(q))
})

const filteredViews = computed(() => {
  if (!search.value) return views.value
  const q = search.value.toLowerCase()
  return views.value.filter(v => v.name.toLowerCase().includes(q))
})

const filteredProcedures = computed(() => {
  if (!search.value) return procedures.value
  const q = search.value.toLowerCase()
  return procedures.value.filter(p => p.name.toLowerCase().includes(q))
})

const filteredQueries = computed(() => {
  if (!querySearch.value) return queries.value
  const q = querySearch.value.toLowerCase()
  return queries.value.filter(
    item => item.name.toLowerCase().includes(q) || item.database.toLowerCase().includes(q)
  )
})

watch(() => props.database, () => { loadTables(); loadViews(); loadProcedures(); loadQueries() }, { immediate: true })

async function loadTables() {
  if (!props.database) return
  tables.value = await listTables(props.resourceId, props.database)
  columns.value = new Map()
  expanded.value = new Set()
}

async function loadViews() {
  if (!props.database) return
  try {
    views.value = await listViews(props.resourceId, props.database)
  } catch {
    toast.error(t('sql.toast.viewListFailed'))
  }
  viewColumns.value = new Map()
  viewExpanded.value = new Set()
}

async function loadProcedures() {
  if (!props.database) return
  try {
    procedures.value = await listProcedures(props.resourceId, props.database)
  } catch {
    toast.error(t('sql.toast.procedureListFailed'))
  }
}

async function loadQueries() {
  queries.value = await listQueries(props.resourceId)
}

async function loadColumnsForTable(tableName: string) {
  if (!columns.value.has(tableName)) {
    const cols = await listColumns(props.resourceId, props.database, tableName)
    columns.value.set(tableName, cols)
  }
}

async function toggleTable(name: string) {
  if (expanded.value.has(name)) {
    expanded.value.delete(name)
  } else {
    expanded.value.add(name)
    await loadColumnsForTable(name)
    emit('select-table', name)
  }
}

async function loadColumnsForView(viewName: string) {
  if (!viewColumns.value.has(viewName)) {
    const cols = await listColumns(props.resourceId, props.database, viewName)
    viewColumns.value.set(viewName, cols)
  }
}

async function toggleView(name: string) {
  if (viewExpanded.value.has(name)) {
    viewExpanded.value.delete(name)
  } else {
    viewExpanded.value.add(name)
    await loadColumnsForView(name)
  }
}

function handleViewContextMenu(event: MouseEvent, view: ViewInfo) {
  ctxMenu.show(event, [
    { label: t('sql.tree.ctx.viewStructure'), action: () => toggleView(view.name) },
    { label: t('sql.tree.ctx.viewDefinition'), action: () => handleViewDefinition(view.name, 'view') },
    { separator: true },
    { label: t('sql.tree.ctx.viewName'), action: () => navigator.clipboard.writeText(view.name) },
    { separator: true },
    { label: t('sql.tree.ctx.refresh'), action: () => loadViews() },
  ])
}

function handleProcedureContextMenu(event: MouseEvent, proc: ProcedureInfo) {
  ctxMenu.show(event, [
    { label: t('sql.tree.ctx.viewDefinition'), action: () => handleViewDefinition(proc.name, proc.type === 'PROCEDURE' ? 'procedure' : 'function') },
    { separator: true },
    { label: t('sql.tree.ctx.procedureName'), action: () => navigator.clipboard.writeText(proc.name) },
    { separator: true },
    { label: t('sql.tree.ctx.refresh'), action: () => loadProcedures() },
  ])
}

function formatDate(isoTs: string): string {
  const ts = parseInt(isoTs, 10)
  const d = new Date(ts * 1000)
  const now = new Date()
  const diffMs = now.getTime() - d.getTime()
  const diffDays = Math.floor(diffMs / 86400000)
  if (diffDays === 0) return t('sql.sidebar.today')
  if (diffDays === 1) return t('sql.sidebar.yesterday')
  if (diffDays < 7) return t('sql.sidebar.daysAgo', { n: diffDays })
  return d.toLocaleDateString()
}

async function handleQueryContextMenu(event: MouseEvent, query: QueryFileMeta) {
  ctxMenu.show(event, [
    { label: t('sql.sidebar.ctx.open'), action: () => emit('open-query', query) },
    { separator: true },
    { label: t('sql.sidebar.ctx.rename'), action: () => handleRenameQuery(query) },
    { label: t('sql.sidebar.ctx.delete'), action: () => handleDeleteQuery(query) },
  ])
}

async function handleRenameQuery(query: QueryFileMeta) {
  const newName = prompt(t('sql.sidebar.renamePrompt'), query.name)
  if (newName && newName.trim()) {
    await renameQuery(props.resourceId, query.id, newName.trim())
    emit('query-renamed')
    await loadQueries()
  }
}

function handleDeleteQuery(query: QueryFileMeta) {
  pendingDeleteResourceId = props.resourceId
  pendingDeleteQueryId = query.id
  deleteConfirmMsg.value = t('sql.sidebar.deleteConfirm', { name: query.name })
  showDeleteConfirm.value = true
}

async function doDeleteQuery() {
  showDeleteConfirm.value = false
  await deleteQuery(pendingDeleteResourceId, pendingDeleteQueryId)
  emit('query-deleted')
  await loadQueries()
}

async function handleTableContextMenu(event: MouseEvent, table: TableInfo) {
  ctxMenu.show(event, [
    { label: t('sql.tree.ctx.viewStructure'), action: () => toggleTable(table.name) },
    { label: t('sql.tree.ctx.viewRowCount'), action: () => toast.info(`${table.name}: ${table.row_count?.toLocaleString() ?? 'N/A'}`), disabled: table.row_count == null },
    { label: t('sql.tree.ctx.viewDefinition'), action: () => handleViewDefinition(table.name, 'table') },
    { separator: true },
    { label: t('sql.tree.ctx.copyTableName'), action: () => navigator.clipboard.writeText(table.name) },
    { label: t('sql.tree.ctx.selectStar'), action: () => emit('select-table', table.name) },
    { label: t('sql.tree.ctx.exportData'), action: () => emit('export-table', table.name) },
    { separator: true },
    { label: t('sql.tree.ctx.refresh'), action: () => loadTables() },
  ])
}

function handleColumnContextMenu(event: MouseEvent, col: ColumnInfo) {
  ctxMenu.show(event, [
    { label: t('sql.tree.ctx.copyColumnName'), action: () => navigator.clipboard.writeText(col.name) },
    { label: t('sql.tree.ctx.copyColumnType'), action: () => navigator.clipboard.writeText(col.data_type) },
    { label: t('sql.tree.ctx.viewConstraints'), action: () => toast.info(`${col.name}: ${col.is_primary_key ? 'PK' : ''}${col.is_nullable === false ? ' NOT NULL' : ''}`) },
  ])
}

function handleTreeContextMenu(event: MouseEvent) {
  ctxMenu.show(event, [
    { label: t('sql.tree.ctx.expandAll'), action: expandAll },
    { label: t('sql.tree.ctx.collapseAll'), action: collapseAll },
    { separator: true },
    { label: t('sql.tree.ctx.refreshStructure'), action: () => emit('refresh') },
    { separator: true },
    { label: t('sql.tree.ctx.createNewTable'), action: handleCreateNewTable },
  ])
}

async function handleViewDefinition(name: string, type: 'table' | 'view' | 'procedure' | 'function' = 'view') {
  if (!props.database) return
  try {
    const { ddl } = await getDdl(props.resourceId, props.database, name, type)
    emit('open-sql-tab', `${type} ${name} — DDL`, ddl)
  } catch {
    toast.error(t('sql.toast.definitionFailed'))
  }
}

function getDialect(): string {
  const protocol = props.protocol?.toLowerCase() ?? ''
  if (protocol.includes('postgres')) return 'postgresql'
  if (protocol.includes('sqlite')) return 'sqlite'
  return 'mysql'
}

function handleCreateNewTable() {
  if (!props.database) return
  const dialect = getDialect()
  let ddl: string
  if (dialect === 'postgresql') {
    ddl = `CREATE TABLE ${props.database}.new_table (\n  id SERIAL PRIMARY KEY,\n  name VARCHAR(255) NOT NULL,\n  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);`
  } else if (dialect === 'sqlite') {
    ddl = `CREATE TABLE ${props.database}.new_table (\n  id INTEGER PRIMARY KEY AUTOINCREMENT,\n  name TEXT NOT NULL,\n  created_at DATETIME DEFAULT CURRENT_TIMESTAMP\n);`
  } else {
    ddl = `CREATE TABLE ${props.database}.new_table (\n  id INT PRIMARY KEY AUTO_INCREMENT,\n  name VARCHAR(255) NOT NULL,\n  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);`
  }
  emit('open-sql-tab', t('sql.tree.ctx.createNewTable'), ddl)
}

async function expandAll() {
  for (const table of filteredTables.value) {
    expanded.value.add(table.name)
    await loadColumnsForTable(table.name)
  }
}

function collapseAll() {
  expanded.value = new Set()
}

// 暴露 loadQueries 给父组件调用
defineExpose({ loadQueries })
</script>

<style scoped>
.sql-sidebar {
  border-right: 1px solid var(--border);
  background: var(--bg-surface);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
  position: relative;
}

.sql-sidebar-db {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  padding: var(--sp-xs) var(--sp-sm);
  border-bottom: 1px solid var(--border);
  background: var(--bg-deep);
}

.db-select {
  flex: 1;
  padding: var(--sp-xs) var(--sp-sm);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--fs-sm);
  outline: none;
  min-width: 0;
}

.sql-sidebar-schema {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 120px;
}

.sql-sidebar-search {
  padding: var(--sp-xs) var(--sp-sm);
  border-bottom: 1px solid var(--border);
}

.sql-sidebar-search input {
  width: 100%;
  padding: var(--sp-xs) var(--sp-sm);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-deep);
  color: var(--text-primary);
  font-size: var(--fs-sm);
  outline: none;
}

.sql-tree {
  flex: 1;
  overflow-y: auto;
  padding: var(--sp-xs) 0;
  font-size: var(--fs-sm);
}

.tree-group-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: 3px var(--sp-md);
  cursor: pointer;
  font-weight: 600;
  color: var(--text-primary);
  font-size: var(--fs-sm);
}

.tree-group-header:hover { background: var(--bg-hover); }
.tree-group-header .tree-icon { font-size: 8px; color: var(--text-muted); }

.tree-children { padding-left: var(--sp-lg); }

.tree-col-item {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: 2px var(--sp-md);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.tree-col-item .col-name { flex: 1; color: var(--text-secondary); }
.tree-col-item .col-type { color: var(--accent); }
.tree-col-item .col-key { color: var(--info); font-size: 9px; }

.tree-section-label {
  padding: var(--sp-xs) var(--sp-md);
  font-size: var(--fs-xs);
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-top: 1px solid var(--border);
  margin-top: var(--sp-xs);
}

/* 查询文件区域 */
.sql-sidebar-queries {
  border-top: 1px solid var(--border);
  flex-shrink: 0;
  max-height: 50%;
  display: flex;
  flex-direction: column;
}

.queries-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-xs) var(--sp-sm);
  cursor: pointer;
  font-size: var(--fs-sm);
  font-weight: 600;
  color: var(--text-secondary);
  background: var(--bg-deep);
}

.queries-header:hover { background: var(--bg-hover); }

.queries-count {
  margin-left: auto;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-weight: 400;
}

.queries-list {
  max-height: 200px;
}

.tree-query-item {
  display: flex;
  align-items: flex-start;
  gap: var(--sp-sm);
  padding: var(--sp-xs) var(--sp-md);
  cursor: pointer;
}

.tree-query-item:hover { background: var(--bg-hover); }

.query-icon { font-size: 12px; margin-top: 2px; }

.query-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.query-name {
  font-size: var(--fs-sm);
  color: var(--text-primary);
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.query-meta {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.tree-empty {
  padding: var(--sp-lg) var(--sp-md);
  text-align: center;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

/* 拖拽调整宽度 */
.sidebar-resize {
  position: absolute;
  top: 0;
  right: -3px;
  width: 6px;
  height: 100%;
  cursor: col-resize;
  z-index: 10;
}

.sidebar-resize:hover {
  background: var(--accent);
  opacity: 0.3;
}
</style>
