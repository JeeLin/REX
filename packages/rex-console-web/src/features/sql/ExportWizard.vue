<script setup lang="ts">
import { ref } from 'vue'
import type { QueryResult } from '@/api/sql'

const props = defineProps<{
  result: QueryResult
  tableName?: string
}>()

const emit = defineEmits<{
  close: []
}>()

const format = ref<'csv' | 'json' | 'sql'>('csv')
const fileName = ref(props.tableName || 'export')
const includeHeaders = ref(true)
const prettyPrint = ref(false)
const includeTableName = ref(true)

function doExport() {
  const columns = props.result.columns.map(c => c.name)
  const rows = props.result.rows

  let content = ''
  let mimeType = 'text/plain'
  let ext = ''

  if (format.value === 'csv') {
    ext = 'csv'
    mimeType = 'text/csv'
    const lines: string[] = []
    if (includeHeaders.value) {
      lines.push(columns.map(escapeCsv).join(','))
    }
    for (const row of rows) {
      lines.push(row.map(v => escapeCsv(v === null || v === undefined ? '' : String(v))).join(','))
    }
    content = lines.join('\n')
  } else if (format.value === 'json') {
    ext = 'json'
    mimeType = 'application/json'
    const data = rows.map(row => {
      const obj: Record<string, unknown> = {}
      columns.forEach((col, i) => { obj[col] = row[i] })
      return obj
    })
    content = prettyPrint.value ? JSON.stringify(data, null, 2) : JSON.stringify(data)
  } else if (format.value === 'sql') {
    ext = 'sql'
    mimeType = 'text/plain'
    const table = includeTableName.value ? (props.tableName || 'table_name') : 'table_name'
    const lines: string[] = []
    for (const row of rows) {
      const values = row.map(v => {
        if (v === null || v === undefined) return 'NULL'
        if (typeof v === 'number') return String(v)
        return `'${String(v).replace(/'/g, "''")}'`
      })
      lines.push(`INSERT INTO ${table} (${columns.join(', ')}) VALUES (${values.join(', ')});`)
    }
    content = lines.join('\n')
  }

  const blob = new Blob([content], { type: mimeType })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${fileName.value}.${ext}`
  a.click()
  URL.revokeObjectURL(url)
  emit('close')
}

function escapeCsv(val: string): string {
  if (val.includes(',') || val.includes('"') || val.includes('\n')) {
    return `"${val.replace(/"/g, '""')}"`
  }
  return val
}
</script>

<template>
  <div class="export-overlay" @click.self="emit('close')">
    <div class="export-modal">
      <div class="export-header">
        <span class="export-title">Export Results</span>
        <button class="export-close" @click="emit('close')">×</button>
      </div>

      <div class="export-body">
        <div class="export-field">
          <label class="export-label">Format</label>
          <div class="export-radios">
            <label class="export-radio">
              <input v-model="format" type="radio" value="csv" />
              <span>CSV</span>
            </label>
            <label class="export-radio">
              <input v-model="format" type="radio" value="json" />
              <span>JSON</span>
            </label>
            <label class="export-radio">
              <input v-model="format" type="radio" value="sql" />
              <span>SQL</span>
            </label>
          </div>
        </div>

        <div class="export-field">
          <label class="export-label">File name</label>
          <input
            v-model="fileName"
            class="export-input mono"
            type="text"
            placeholder="export"
          />
        </div>

        <div class="export-field">
          <label class="export-label">Options</label>
          <div class="export-checkboxes">
            <label v-if="format === 'csv'" class="export-checkbox">
              <input v-model="includeHeaders" type="checkbox" />
              <span>Include headers</span>
            </label>
            <label v-if="format === 'json'" class="export-checkbox">
              <input v-model="prettyPrint" type="checkbox" />
              <span>Pretty print</span>
            </label>
            <label v-if="format === 'sql'" class="export-checkbox">
              <input v-model="includeTableName" type="checkbox" />
              <span>Include table name</span>
            </label>
          </div>
        </div>

        <div class="export-info muted">
          {{ result.rows.length }} rows × {{ result.columns.length }} columns
        </div>
      </div>

      <div class="export-footer">
        <button class="export-btn-secondary" @click="emit('close')">Cancel</button>
        <button class="export-btn-primary" @click="doExport">Export</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.export-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.export-modal {
  width: 400px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
}

.export-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3);
  border-bottom: 1px solid var(--border);
}

.export-title {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
}

.export-close {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-md);
}

.export-close:hover {
  color: var(--text-primary);
}

.export-body {
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.export-field {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.export-label {
  font-size: var(--text-xs);
  color: var(--text-muted);
  text-transform: uppercase;
}

.export-radios {
  display: flex;
  gap: var(--space-3);
}

.export-radio {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  font-size: var(--text-sm);
  color: var(--text-primary);
  cursor: pointer;
}

.export-radio input {
  accent-color: var(--accent);
}

.export-input {
  padding: var(--space-1) var(--space-2);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}

.export-input:focus {
  border-color: var(--accent);
}

.export-checkboxes {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.export-checkbox {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-sm);
  color: var(--text-primary);
  cursor: pointer;
}

.export-checkbox input {
  accent-color: var(--accent);
}

.export-info {
  font-size: var(--text-xs);
}

.export-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  padding: var(--space-3);
  border-top: 1px solid var(--border);
}

.export-btn-secondary {
  padding: var(--space-1) var(--space-3);
  background: none;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: background var(--transition);
}

.export-btn-secondary:hover {
  background: var(--bg-hover);
}

.export-btn-primary {
  padding: var(--space-1) var(--space-3);
  background: var(--accent);
  border: none;
  border-radius: var(--radius-sm);
  color: #fff;
  font-size: var(--text-sm);
  cursor: pointer;
  transition: opacity var(--transition);
}

.export-btn-primary:hover {
  opacity: 0.9;
}

.mono {
  font-family: var(--font-mono);
}

.muted {
  color: var(--text-muted);
}
</style>
