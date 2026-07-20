<script setup lang="ts">
import { ref, onMounted } from 'vue'
import ColumnEditor from './ColumnEditor.vue'
import { getColumns, getIndexes, getForeignKeys, getDdl, type ColumnInfo, type IndexInfo, type ForeignKeyInfo, type DdlResult } from '@/api/sql'

const props = defineProps<{
  sessionId: string
  db: string
  table: string
}>()

const emit = defineEmits<{
  close: []
}>()

const activeTab = ref<'columns' | 'indexes' | 'fk' | 'ddl'>('columns')

const columns = ref<ColumnInfo[]>([])
const indexes = ref<IndexInfo[]>([])
const foreignKeys = ref<ForeignKeyInfo[]>([])
const ddlResult = ref<DdlResult | null>(null)
const loading = ref(false)
const error = ref<string | null>(null)

// Designer state for columns
interface DesignerColumn {
  name: string
  type: string
  nullable: boolean
  is_primary_key: boolean
  is_new?: boolean
}

const designerColumns = ref<DesignerColumn[]>([])

onMounted(async () => {
  loading.value = true
  error.value = null
  try {
    const [cols, idx, fks, ddl] = await Promise.all([
      getColumns(props.sessionId, props.db, props.table),
      getIndexes(props.sessionId, props.db, props.table),
      getForeignKeys(props.sessionId, props.db, props.table),
      getDdl(props.sessionId, props.db, props.table),
    ])
    columns.value = cols
    indexes.value = idx
    foreignKeys.value = fks
    ddlResult.value = ddl
    // Initialize designer columns from fetched data
    designerColumns.value = cols.map(c => ({
      name: c.name,
      type: c.data_type,
      nullable: c.nullable,
      is_primary_key: c.is_primary_key,
    }))
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
})

function addColumn() {
  designerColumns.value.push({
    name: '',
    type: 'VARCHAR',
    nullable: true,
    is_primary_key: false,
    is_new: true,
  })
}

function removeColumn(index: number) {
  designerColumns.value.splice(index, 1)
}
</script>

<template>
  <div class="designer">
    <!-- Header -->
    <div class="designer-header">
      <span class="designer-title mono">Table: {{ table }}</span>
      <button class="designer-close" @click="emit('close')" title="Close">×</button>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="designer-loading">
      <div class="spinner" />
      <span>Loading table structure...</span>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="designer-error">
      <span>{{ error }}</span>
    </div>

    <!-- Content -->
    <template v-else>
      <!-- Tabs -->
      <div class="designer-tabs">
        <button
          class="designer-tab"
          :class="{ 'designer-tab--active': activeTab === 'columns' }"
          @click="activeTab = 'columns'"
        >
          Columns
        </button>
        <button
          class="designer-tab"
          :class="{ 'designer-tab--active': activeTab === 'indexes' }"
          @click="activeTab = 'indexes'"
        >
          Indexes ({{ indexes.length }})
        </button>
        <button
          class="designer-tab"
          :class="{ 'designer-tab--active': activeTab === 'fk' }"
          @click="activeTab = 'fk'"
        >
          Foreign Keys ({{ foreignKeys.length }})
        </button>
        <button
          class="designer-tab"
          :class="{ 'designer-tab--active': activeTab === 'ddl' }"
          @click="activeTab = 'ddl'"
        >
          DDL
        </button>
      </div>

      <!-- Columns Tab -->
      <div v-if="activeTab === 'columns'" class="designer-content">
        <div class="designer-toolbar">
          <button class="designer-btn" @click="addColumn">+ Add Column</button>
        </div>
        <ColumnEditor
          v-model="designerColumns"
          @remove="removeColumn"
        />
      </div>

      <!-- Indexes Tab -->
      <div v-if="activeTab === 'indexes'" class="designer-content">
        <div v-if="!indexes.length" class="designer-empty">No indexes</div>
        <table v-else class="designer-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Columns</th>
              <th>Unique</th>
              <th>Type</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="idx in indexes" :key="idx.name">
              <td class="mono">{{ idx.name }}</td>
              <td class="mono">{{ idx.columns.join(', ') }}</td>
              <td>{{ idx.unique ? '✓' : '' }}</td>
              <td>{{ idx.index_type }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Foreign Keys Tab -->
      <div v-if="activeTab === 'fk'" class="designer-content">
        <div v-if="!foreignKeys.length" class="designer-empty">No foreign keys</div>
        <table v-else class="designer-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Columns</th>
              <th>References</th>
              <th>On Delete</th>
              <th>On Update</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="fk in foreignKeys" :key="fk.name">
              <td class="mono">{{ fk.name }}</td>
              <td class="mono">{{ fk.columns.join(', ') }}</td>
              <td class="mono">{{ fk.ref_table }}({{ fk.ref_columns.join(', ') }})</td>
              <td>{{ fk.on_delete }}</td>
              <td>{{ fk.on_update }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- DDL Tab -->
      <div v-if="activeTab === 'ddl'" class="designer-content">
        <pre v-if="ddlResult" class="designer-ddl mono">{{ ddlResult.ddl }}</pre>
        <div v-else class="designer-empty">No DDL available</div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.designer {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-deep);
}

.designer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-2) var(--space-3);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.designer-title {
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.designer-close {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-md);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  transition: color var(--transition);
}

.designer-close:hover {
  color: var(--danger);
}

.designer-loading,
.designer-error,
.designer-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  height: 100%;
  color: var(--text-muted);
  font-size: var(--text-sm);
}

.designer-error {
  color: var(--danger);
}

.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.designer-tabs {
  display: flex;
  gap: 0;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.designer-tab {
  padding: var(--space-2) var(--space-3);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--text-muted);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: color var(--transition), border-color var(--transition);
}

.designer-tab:hover {
  color: var(--text-primary);
}

.designer-tab--active {
  color: var(--text-primary);
  border-bottom-color: var(--accent);
}

.designer-content {
  flex: 1;
  overflow: auto;
}

.designer-toolbar {
  display: flex;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
}

.designer-btn {
  padding: var(--space-1) var(--space-3);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: background var(--transition), border-color var(--transition);
}

.designer-btn:hover {
  background: var(--bg-hover);
  border-color: var(--accent);
}

.designer-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-sm);
}

.designer-table th,
.designer-table td {
  padding: var(--space-1) var(--space-3);
  text-align: left;
  border-bottom: 1px solid var(--border);
}

.designer-table th {
  background: var(--bg-surface);
  font-weight: 600;
  color: var(--text-muted);
  font-size: var(--text-xs);
  text-transform: uppercase;
  position: sticky;
  top: 0;
}

.designer-table tr:hover td {
  background: var(--bg-hover);
}

.designer-ddl {
  padding: var(--space-3);
  margin: 0;
  font-size: var(--text-sm);
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}

.mono {
  font-family: var(--font-mono);
}
</style>
