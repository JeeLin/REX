<script setup lang="ts">
import { computed, ref } from 'vue'
import type { QueryResult } from '@/api/sql'

const props = defineProps<{
  result: QueryResult | null
  loading: boolean
  error: string | null
}>()

const emit = defineEmits<{
  export: []
}>()

const hasData = computed(() => props.result && props.result.rows.length > 0)
const isEmpty = computed(() => props.result && props.result.rows.length === 0 && !props.error)

// Sorting
type SortDir = 'asc' | 'desc' | null
const sortCol = ref<number | null>(null)
const sortDir = ref<SortDir>(null)

function toggleSort(colIdx: number) {
  if (sortCol.value === colIdx) {
    // Cycle: asc -> desc -> null
    if (sortDir.value === 'asc') sortDir.value = 'desc'
    else if (sortDir.value === 'desc') { sortCol.value = null; sortDir.value = null }
    else sortDir.value = 'asc'
  } else {
    sortCol.value = colIdx
    sortDir.value = 'asc'
  }
}

const sortedRows = computed(() => {
  if (!props.result || sortCol.value === null || !sortDir.value) {
    return props.result?.rows ?? []
  }
  const col = sortCol.value
  const dir = sortDir.value === 'asc' ? 1 : -1
  return [...props.result.rows].sort((a, b) => {
    const av = a[col]
    const bv = b[col]
    if (av === null || av === undefined) return 1
    if (bv === null || bv === undefined) return -1
    if (typeof av === 'number' && typeof bv === 'number') return (av - bv) * dir
    return String(av).localeCompare(String(bv)) * dir
  })
})

function sortIcon(colIdx: number): string {
  if (sortCol.value !== colIdx) return ''
  return sortDir.value === 'asc' ? ' ↑' : ' ↓'
}

function formatCell(value: unknown): string {
  if (value === null || value === undefined) return 'NULL'
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}

function cellClass(value: unknown): string {
  if (value === null || value === undefined) return 'cell-null'
  if (typeof value === 'number') return 'cell-number'
  if (typeof value === 'boolean') return 'cell-bool'
  return ''
}
</script>

<template>
  <div class="result-grid">
    <!-- Loading -->
    <div v-if="loading" class="result-grid-loading">
      <div class="spinner" />
      <span>Executing...</span>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="result-grid-error">
      <span class="error-icon">✕</span>
      <span class="error-msg">{{ error }}</span>
    </div>

    <!-- Empty -->
    <div v-else-if="isEmpty" class="result-grid-empty">No results</div>

    <!-- Data table -->
    <div v-else-if="hasData" class="result-grid-table-wrap">
      <table class="result-grid-table">
        <thead>
          <tr>
            <th class="row-num">#</th>
            <th
              v-for="(col, ci) in result!.columns"
              :key="col.name"
              class="col-header"
              @click="toggleSort(ci)"
            >
              <span class="col-name">{{ col.name }}{{ sortIcon(ci) }}</span>
              <span class="col-type muted">{{ col.data_type }}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(row, ri) in sortedRows" :key="ri" class="data-row">
            <td class="row-num">{{ ri + 1 }}</td>
            <td
              v-for="(cell, ci) in row"
              :key="ci"
              :class="['data-cell', cellClass(cell)]"
              :title="formatCell(cell)"
            >
              {{ formatCell(cell) }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Status bar -->
    <div v-if="result" class="result-grid-status">
      <span>{{ result.rows.length }} rows</span>
      <span v-if="result.affected_rows">· {{ result.affected_rows }} affected</span>
      <span>· {{ result.elapsed_ms }}ms</span>
      <span v-if="hasData" class="status-spacer" />
      <button v-if="hasData" class="export-btn" @click="emit('export')">Export</button>
    </div>
  </div>
</template>

<style scoped>
.result-grid {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  background: var(--bg-deep);
}

.result-grid-loading,
.result-grid-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  height: 100%;
  color: var(--text-muted);
  font-size: var(--text-sm);
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

.result-grid-error {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  padding: var(--space-3);
  margin: var(--space-2);
  background: rgba(248, 81, 73, 0.1);
  border: 1px solid rgba(248, 81, 73, 0.3);
  border-radius: var(--radius);
  color: var(--danger);
  font-size: var(--text-sm);
}

.error-icon {
  flex-shrink: 0;
  font-weight: 700;
}

.error-msg {
  word-break: break-word;
}

/* ---- table ---- */
.result-grid-table-wrap {
  flex: 1;
  overflow: auto;
}

.result-grid-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-sm);
  font-family: var(--font-mono);
}

.result-grid-table th,
.result-grid-table td {
  padding: var(--space-1) var(--space-3);
  text-align: left;
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.result-grid-table thead {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--bg-surface);
}

.col-header {
  font-weight: 600;
  color: var(--text-primary);
  cursor: pointer;
  user-select: none;
}

.col-header:hover {
  background: var(--bg-hover);
}

.col-name {
  margin-right: var(--space-1);
}

.col-type {
  font-size: var(--text-xs);
  font-weight: 400;
}

.row-num {
  color: var(--text-muted);
  font-size: var(--text-xs);
  text-align: right;
  width: 40px;
  min-width: 40px;
}

.data-row:hover {
  background: var(--bg-hover);
}

.cell-null {
  color: var(--text-muted);
  font-style: italic;
}

.cell-number {
  color: var(--info);
}

.cell-bool {
  color: var(--accent);
}

/* ---- status bar ---- */
.result-grid-status {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  background: var(--bg-surface);
  border-top: 1px solid var(--border);
  font-size: var(--text-xs);
  color: var(--text-muted);
  flex-shrink: 0;
}

.status-spacer {
  flex: 1;
}

.export-btn {
  padding: 2px var(--space-2);
  background: none;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: background var(--transition), border-color var(--transition);
}

.export-btn:hover {
  background: var(--bg-hover);
  border-color: var(--accent);
}

.muted {
  color: var(--text-muted);
}
</style>
