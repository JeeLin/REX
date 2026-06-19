<template>
  <div class="sql-sidebar">
    <div class="sql-sidebar-header">
      <span>{{ database }}</span>
      <button class="btn btn-ghost btn-sm" @click="$emit('refresh')">↻</button>
    </div>
    <div class="sql-sidebar-search">
      <input type="text" v-model="search" :placeholder="t('sql.searchPlaceholder')" />
    </div>
    <div class="sql-tree">
      <div v-for="table in filteredTables" :key="table.name" class="tree-group">
        <div class="tree-group-header" @click="toggleTable(table.name)">
          <span class="tree-icon">{{ expanded.has(table.name) ? '▾' : '▸' }}</span>
          <span>⊞</span>
          <span>{{ table.name }}</span>
          <span class="tree-count" v-if="table.row_count != null">{{ table.row_count.toLocaleString() }}</span>
        </div>
        <div v-if="expanded.has(table.name)" class="tree-children">
          <div v-for="col in columns.get(table.name)" :key="col.name" class="tree-col-item">
            <span v-if="col.is_primary_key" class="col-key">PK</span>
            <span v-else class="col-key" style="visibility:hidden">_</span>
            <span class="col-name">{{ col.name }}</span>
            <span class="col-type">{{ col.data_type }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { listTables, listColumns } from '@/api/sql'
import type { TableInfo, ColumnInfo } from '@/api/sql'

const { t } = useI18n()

const props = defineProps<{
  resourceId: string
  database: string
}>()

const emit = defineEmits<{
  'select-table': [table: string]
  'refresh': []
}>()

const search = ref('')
const tables = ref<TableInfo[]>([])
const columns = ref<Map<string, ColumnInfo[]>>(new Map())
const expanded = ref<Set<string>>(new Set())

const filteredTables = computed(() => {
  if (!search.value) return tables.value
  const q = search.value.toLowerCase()
  return tables.value.filter(t => t.name.toLowerCase().includes(q))
})

watch(() => props.database, loadTables, { immediate: true })

async function loadTables() {
  if (!props.database) return
  tables.value = await listTables(props.resourceId, props.database)
  columns.value = new Map()
  expanded.value = new Set()
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
</style>
