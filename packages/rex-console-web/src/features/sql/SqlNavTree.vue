<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { onClickOutside } from '@vueuse/core'
import type { DatabaseNode } from './useSqlNav'

const { t } = useI18n()

const props = defineProps<{
  databases: DatabaseNode[]
  loading: boolean
  searchQuery: string
}>()

const emit = defineEmits<{
  selectTable: [db: string, table: string]
  designTable: [db: string, table: string]
  viewDdl: [db: string, table: string]
  copyDdl: [db: string, table: string]
  newQuery: [db: string, table: string]
  properties: [db: string, table: string]
  importData: [db: string, table: string]
  refresh: []
  'update:searchQuery': [value: string]
}>()

/* ---- search filter ---- */
const filteredDatabases = computed(() => {
  if (!props.searchQuery) return props.databases
  const q = props.searchQuery.toLowerCase()
  return props.databases.filter(
    (db) =>
      db.name.toLowerCase().includes(q) ||
      db.tables.some((t) => t.name.toLowerCase().includes(q)),
  )
})

function tablesForDb(db: DatabaseNode) {
  if (!props.searchQuery) return db.tables
  const q = props.searchQuery.toLowerCase()
  return db.tables.filter((t) => t.name.toLowerCase().includes(q))
}

function baseTables(db: DatabaseNode) {
  return tablesForDb(db).filter((t) => t.table_type === 'BASE TABLE')
}

function views(db: DatabaseNode) {
  return tablesForDb(db).filter((t) => t.table_type === 'VIEW')
}

function onDblClick(db: DatabaseNode, tableName: string) {
  emit('selectTable', db.name, tableName)
}

/* ---- context menu ---- */
const ctxMenu = ref<{ show: boolean; x: number; y: number; dbName: string; tableName: string }>({
  show: false, x: 0, y: 0, dbName: '', tableName: '',
})
const ctxMenuRef = ref<HTMLElement | null>(null)

onClickOutside(ctxMenuRef, () => { ctxMenu.value.show = false })

function onContextMenu(e: MouseEvent, dbName: string, tableName: string) {
  e.preventDefault()
  ctxMenu.value = { show: true, x: e.clientX, y: e.clientY, dbName, tableName }
}

function ctxAction(action: string) {
  if (action === 'copyTableName') {
    navigator.clipboard?.writeText(ctxMenu.value.tableName)
  } else if (action === 'copyDdl') {
    emit('copyDdl', ctxMenu.value.dbName, ctxMenu.value.tableName)
  } else if (action === 'newQuery') {
    emit('newQuery', ctxMenu.value.dbName, ctxMenu.value.tableName)
  } else if (action === 'refresh') {
    emit('refresh')
  } else if (action === 'viewDdl') {
    emit('viewDdl', ctxMenu.value.dbName, ctxMenu.value.tableName)
  } else if (action === 'properties') {
    emit('properties', ctxMenu.value.dbName, ctxMenu.value.tableName)
  }
  ctxMenu.value.show = false
}
</script>

<template>
  <div class="sql-nav">
    <!-- Search -->
    <div class="sql-nav-search">
      <input
        class="sql-nav-search-input mono"
        type="text"
        placeholder="Search..."
        :value="searchQuery"
        @input="emit('update:searchQuery', ($event.target as HTMLInputElement).value)"
      />
      <button class="sql-nav-refresh" title="Refresh" @click="emit('refresh')">↻</button>
    </div>

    <!-- Tree -->
    <div class="sql-nav-tree">
      <div v-if="loading" class="sql-nav-loading">{{ t('sql.loading') }}</div>

      <div v-for="db in filteredDatabases" :key="db.name" class="sql-nav-db">
        <!-- Database header -->
        <div class="sql-nav-item sql-nav-db-header" @click="db.expanded = !db.expanded">
          <span class="sql-nav-arrow" :class="{ 'sql-nav-arrow--open': db.expanded }">▸</span>
          <span class="sql-nav-icon" style="color: var(--info)">dB</span>
          <span class="sql-nav-label">{{ db.name }}</span>
          <span v-if="db.loading" class="sql-nav-spinner" />
        </div>

        <!-- Database content -->
        <div v-if="db.expanded" class="sql-nav-children">
          <!-- Tables group -->
          <div v-if="baseTables(db).length" class="sql-nav-group">
            <div class="sql-nav-item sql-nav-group-header">
              <span class="sql-nav-label muted">{{ t('sql.tables') }}</span>
              <span class="sql-nav-badge">{{ baseTables(db).length }}</span>
            </div>
            <div
              v-for="table in baseTables(db)"
              :key="table.name"
              class="sql-nav-item sql-nav-leaf"
              @dblclick="onDblClick(db, table.name)"
              @contextmenu="onContextMenu($event, db.name, table.name)"
            >
              <span class="sql-nav-icon" style="color: var(--accent)">⊞</span>
              <span class="sql-nav-label">{{ table.name }}</span>
            </div>
          </div>

          <!-- Views group -->
          <div v-if="views(db).length" class="sql-nav-group">
            <div class="sql-nav-item sql-nav-group-header">
              <span class="sql-nav-label muted">{{ t('sql.views') }}</span>
              <span class="sql-nav-badge">{{ views(db).length }}</span>
            </div>
            <div
              v-for="view in views(db)"
              :key="view.name"
              class="sql-nav-item sql-nav-leaf"
              @dblclick="onDblClick(db, view.name)"
              @contextmenu="onContextMenu($event, db.name, view.name)"
            >
              <span class="sql-nav-icon" style="color: var(--purple)">◎</span>
              <span class="sql-nav-label">{{ view.name }}</span>
            </div>
          </div>

          <div
            v-if="!baseTables(db).length && !views(db).length && !db.loading"
            class="sql-nav-item sql-nav-empty"
          >
            No tables
          </div>
        </div>
      </div>

      <div v-if="!loading && !filteredDatabases.length" class="sql-nav-empty">
        No databases
      </div>
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <div
        v-if="ctxMenu.show"
        ref="ctxMenuRef"
        class="sql-ctx-menu"
        :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
      >
        <button class="sql-ctx-item" @click="ctxAction('copyTableName')">
          <span class="sql-ctx-icon">📋</span>{{ t('sql.copyTableName') }}
        </button>
        <button class="sql-ctx-item" @click="ctxAction('copyDdl')">
          <span class="sql-ctx-icon">📋</span>{{ t('sql.copyDDL') }}
        </button>
        <button class="sql-ctx-item" @click="ctxAction('newQuery')">
          <span class="sql-ctx-icon">🔍</span>{{ t('sql.newQuery') }}
        </button>
        <button class="sql-ctx-item" @click="ctxAction('refresh')">
          <span class="sql-ctx-icon">🔄</span>{{ t('sql.refresh') }}
        </button>
        <div class="sql-ctx-separator" />
        <button class="sql-ctx-item" @click="ctxAction('viewDdl')">
          <span class="sql-ctx-icon">📄</span>{{ t('sql.viewDDL') }}
        </button>
        <button class="sql-ctx-item" @click="ctxAction('properties')">
          <span class="sql-ctx-icon">⚙</span>{{ t('sql.properties') }}
        </button>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.sql-nav {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-deep);
  border-right: 1px solid var(--border);
  overflow: hidden;
}

.sql-nav-search {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-2);
  border-bottom: 1px solid var(--border);
}

.sql-nav-search-input {
  flex: 1;
  padding: var(--space-1) var(--space-2);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}

.sql-nav-search-input:focus {
  border-color: var(--accent);
}

.sql-nav-refresh {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-md);
  padding: var(--space-1);
  border-radius: var(--radius-sm);
  transition: color var(--transition);
}

.sql-nav-refresh:hover {
  color: var(--text-primary);
}

.sql-nav-tree {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-1) 0;
}

.sql-nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  cursor: pointer;
  font-size: var(--text-sm);
  color: var(--text-primary);
  transition: background var(--transition);
  white-space: nowrap;
}

.sql-nav-item:hover {
  background: var(--bg-hover);
}

.sql-nav-db-header {
  font-weight: 500;
}

.sql-nav-leaf {
  padding-left: var(--space-6);
}

.sql-nav-group-header {
  cursor: default;
}

.sql-nav-group-header:hover {
  background: none;
}

.sql-nav-children {
  /* subtle indent */
}

.sql-nav-arrow {
  font-size: 10px;
  color: var(--text-muted);
  transition: transform var(--transition);
  width: 12px;
  text-align: center;
}

.sql-nav-arrow--open {
  transform: rotate(90deg);
}

.sql-nav-icon {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  font-weight: 600;
  width: 20px;
  text-align: center;
}

.sql-nav-label {
  overflow: hidden;
  text-overflow: ellipsis;
}

.sql-nav-label.muted {
  color: var(--text-muted);
  font-size: var(--text-xs);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.sql-nav-badge {
  margin-left: auto;
  font-size: var(--text-xs);
  color: var(--text-muted);
  background: var(--bg-elevated);
  padding: 1px 6px;
  border-radius: 10px;
}

.sql-nav-spinner {
  margin-left: auto;
  width: 12px;
  height: 12px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.sql-nav-empty {
  padding: var(--space-4);
  text-align: center;
  color: var(--text-muted);
  font-size: var(--text-sm);
}

.sql-nav-loading {
  padding: var(--space-4);
  text-align: center;
  color: var(--text-muted);
  font-size: var(--text-sm);
}

.sql-ctx-menu {
  position: fixed;
  min-width: 160px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  padding: var(--space-1);
  z-index: 80;
}

.sql-ctx-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: var(--space-1) var(--space-3);
  background: none;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  text-align: left;
  cursor: pointer;
  transition: background var(--transition);
}

.sql-ctx-icon {
  width: 18px;
  text-align: center;
  flex-shrink: 0;
}

.sql-ctx-item:hover {
  background: var(--bg-hover);
}

.sql-ctx-separator {
  height: 1px;
  background: var(--border);
  margin: var(--space-1) 0;
}
</style>
