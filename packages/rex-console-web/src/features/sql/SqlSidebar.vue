<template>
  <div class="sql-sidebar">
    <!-- 模式切换标签 -->
    <div class="sql-sidebar-tabs">
      <button
        class="sidebar-tab"
        :class="{ active: mode === 'schema' }"
        @click="mode = 'schema'"
      >
        {{ t('sql.sidebar.schema') }}
      </button>
      <button
        class="sidebar-tab"
        :class="{ active: mode === 'queries' }"
        @click="mode = 'queries'"
      >
        {{ t('sql.sidebar.queries') }}
      </button>
    </div>

    <!-- 库表结构模式 -->
    <template v-if="mode === 'schema'">
      <div class="sql-sidebar-header" @contextmenu.prevent="handleHeaderContextMenu">
        <span>{{ database }}</span>
        <button class="btn btn-ghost btn-sm" @click="$emit('refresh')">↻</button>
      </div>
      <div class="sql-sidebar-search">
        <input type="text" v-model="search" :placeholder="t('sql.searchPlaceholder')" />
      </div>
      <div class="sql-tree" @contextmenu.prevent="handleTreeContextMenu">
        <div v-for="table in filteredTables" :key="table.name" class="tree-group">
          <div class="tree-group-header" @click="toggleTable(table.name)" @contextmenu.prevent="handleTableContextMenu($event, table)">
            <span class="tree-icon">{{ expanded.has(table.name) ? '▾' : '▸' }}</span>
            <span>📊</span>
            <span>{{ table.name }}</span>
            <span class="tree-count" v-if="table.row_count != null">{{ table.row_count.toLocaleString() }}</span>
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
      </div>
    </template>

    <!-- 查询文件模式 -->
    <template v-if="mode === 'queries'">
      <div class="sql-sidebar-header">
        <span>{{ t('sql.sidebar.savedQueries') }}</span>
        <button class="btn btn-ghost btn-sm" @click="loadQueries">↻</button>
      </div>
      <div class="sql-sidebar-search">
        <input type="text" v-model="querySearch" :placeholder="t('sql.sidebar.searchQueries')" />
      </div>
      <div class="sql-tree">
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
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'
import { listTables, listColumns, listQueries, deleteQuery, renameQuery } from '@/api/sql'
import type { TableInfo, ColumnInfo, QueryFileMeta } from '@/api/sql'

const { t } = useI18n()
const ctxMenu = useContextMenu()

const props = defineProps<{
  resourceId: string
  database: string
}>()

const emit = defineEmits<{
  'select-table': [table: string]
  'open-query': [query: QueryFileMeta]
  'refresh': []
  'query-deleted': []
  'query-renamed': []
}>()

const mode = ref<'schema' | 'queries'>('schema')
const search = ref('')
const querySearch = ref('')
const tables = ref<TableInfo[]>([])
const columns = ref<Map<string, ColumnInfo[]>>(new Map())
const expanded = ref<Set<string>>(new Set())
const queries = ref<QueryFileMeta[]>([])

const filteredTables = computed(() => {
  if (!search.value) return tables.value
  const q = search.value.toLowerCase()
  return tables.value.filter(t => t.name.toLowerCase().includes(q))
})

const filteredQueries = computed(() => {
  if (!querySearch.value) return queries.value
  const q = querySearch.value.toLowerCase()
  return queries.value.filter(
    item => item.name.toLowerCase().includes(q) || item.database.toLowerCase().includes(q)
  )
})

watch(() => props.database, loadTables, { immediate: true })
watch(mode, (m) => {
  if (m === 'queries') loadQueries()
})

async function loadTables() {
  if (!props.database) return
  tables.value = await listTables(props.resourceId, props.database)
  columns.value = new Map()
  expanded.value = new Set()
}

async function loadQueries() {
  queries.value = await listQueries(props.resourceId)
}

async function toggleTable(name: string) {
  if (expanded.value.has(name)) {
    expanded.value.delete(name)
  } else {
    expanded.value.add(name)
    if (!columns.value.has(name)) {
      const cols = await listColumns(props.resourceId, props.database, name)
      columns.value.set(name, cols)
    }
  }
  emit('select-table', name)
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

async function handleDeleteQuery(query: QueryFileMeta) {
  if (!confirm(t('sql.sidebar.deleteConfirm', { name: query.name }))) return
  await deleteQuery(props.resourceId, query.id)
  emit('query-deleted')
  await loadQueries()
}

async function handleTableContextMenu(event: MouseEvent, table: TableInfo) {
  ctxMenu.show(event, [
    { label: t('sql.tree.ctx.viewStructure'), action: () => toggleTable(table.name) },
    { label: t('sql.tree.ctx.viewRowCount'), disabled: table.row_count == null },
    { separator: true },
    { label: t('sql.tree.ctx.copyTableName'), action: () => navigator.clipboard.writeText(table.name) },
    { label: t('sql.tree.ctx.selectStar'), action: () => emit('select-table', table.name) },
  ])
}

function handleColumnContextMenu(event: MouseEvent, col: ColumnInfo) {
  ctxMenu.show(event, [
    { label: t('sql.tree.ctx.copyColumnName'), action: () => navigator.clipboard.writeText(col.name) },
    { label: t('sql.tree.ctx.copyColumnType'), action: () => navigator.clipboard.writeText(col.data_type) },
  ])
}

function handleHeaderContextMenu(event: MouseEvent) {
  ctxMenu.show(event, [
    { label: t('sql.tree.ctx.refresh'), action: () => emit('refresh') },
    { label: t('sql.tree.ctx.copyDbName'), action: () => navigator.clipboard.writeText(props.database) },
  ])
}

function handleTreeContextMenu(event: MouseEvent) {
  ctxMenu.show(event, [
    { label: t('sql.tree.ctx.expandAll'), action: expandAll },
    { label: t('sql.tree.ctx.collapseAll'), action: collapseAll },
    { separator: true },
    { label: t('sql.tree.ctx.refreshStructure'), action: () => emit('refresh') },
  ])
}

async function expandAll() {
  for (const table of filteredTables.value) {
    expanded.value.add(table.name)
  }
  for (const table of filteredTables.value) {
    if (!columns.value.has(table.name)) {
      const cols = await listColumns(props.resourceId, props.database, table.name)
      columns.value.set(table.name, cols)
    }
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
  width: 260px;
  border-right: 1px solid var(--border);
  background: var(--bg-surface);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
}

.sql-sidebar-tabs {
  display: flex;
  border-bottom: 1px solid var(--border);
  background: var(--bg-deep);
}

.sidebar-tab {
  flex: 1;
  padding: var(--sp-xs) var(--sp-sm);
  font-size: var(--fs-xs);
  font-weight: 500;
  color: var(--text-muted);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.sidebar-tab:hover {
  color: var(--text-secondary);
}

.sidebar-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.sql-sidebar-header {
  padding: var(--sp-sm) var(--sp-md);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: var(--fs-sm);
  font-weight: 600;
  color: var(--text-secondary);
}

.sql-sidebar-search {
  padding: var(--sp-sm) var(--sp-md);
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

/* 查询文件列表 */
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
</style>
