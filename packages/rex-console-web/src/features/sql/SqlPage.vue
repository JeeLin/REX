<script setup lang="ts">
import { ref, computed, onBeforeUnmount } from 'vue'
import { useSqlNav } from './useSqlNav'
import SqlNavTree from './SqlNavTree.vue'
import SqlEditor from './SqlEditor.vue'

const sessionId = ref<string | null>(null)
const { databases, loading, searchQuery, loadDatabases } = useSqlNav(sessionId)

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

/* ---- query tabs ---- */
interface QueryTab {
  id: number
  title: string
  sql: string
  dirty: boolean
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

function onExecute(sql: string) {
  console.log('execute', sql)
  // TODO: subtask 5 — query execution
}

function onSave(sql: string) {
  console.log('save', sql)
  // TODO: save to file
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

    <!-- Right panel: tabs + editor -->
    <div class="sql-page-content">
      <!-- Tab bar -->
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
        <button class="sql-tab-add" @click="createTab()" title="New Query (Ctrl+T)">+</button>
      </div>

      <!-- Editor area -->
      <div class="sql-editor-area">
        <SqlEditor
          v-if="activeTab"
          :key="activeTab.id"
          :model-value="activeTab.sql"
          @update:model-value="activeTab.sql = $event; activeTab.dirty = true"
          @execute="onExecute"
          @save="onSave"
        />
        <div v-else class="sql-page-placeholder">
          <div class="placeholder-icon">📋</div>
          <div class="placeholder-title">SQL Console</div>
          <div class="placeholder-desc">
            Select a table or click + to create a new query
          </div>
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

/* ---- tabs ---- */
.sql-tabs {
  display: flex;
  align-items: center;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  overflow-x: auto;
  flex-shrink: 0;
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

/* ---- editor area ---- */
.sql-editor-area {
  flex: 1;
  min-height: 0;
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

.placeholder-icon {
  font-size: 48px;
  opacity: 0.4;
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
