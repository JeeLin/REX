<script setup lang="ts">
import { computed, ref, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import type { QueryResult } from '@/api/sql'

const { t } = useI18n()

const props = defineProps<{
  result: QueryResult | null
  loading: boolean
  error: string | null
}>()

const emit = defineEmits<{
  export: []
  apply: [changes: EditCell[]]
  discard: []
}>()

export interface EditCell {
  rowIndex: number
  colIndex: number
  oldValue: unknown
  newValue: unknown
}

const hasData = computed(() => props.result && props.result.rows.length > 0)
const isEmpty = computed(() => props.result && props.result.rows.length === 0 && !props.error)

// ============================================================
// Subtask 4: Multi-column sorting
// ============================================================
interface SortEntry {
  col: number
  dir: 'asc' | 'desc'
}
const sortColumns = ref<SortEntry[]>([])

function toggleSort(colIdx: number, shiftKey: boolean) {
  const existing = sortColumns.value.find((s) => s.col === colIdx)
  if (shiftKey) {
    // Multi-column: add or cycle
    if (existing) {
      if (existing.dir === 'asc') existing.dir = 'desc'
      else sortColumns.value = sortColumns.value.filter((s) => s.col !== colIdx)
    } else {
      sortColumns.value.push({ col: colIdx, dir: 'asc' })
    }
  } else {
    // Single-column: cycle asc → desc → none
    if (existing) {
      if (existing.dir === 'asc') {
        sortColumns.value = [{ col: colIdx, dir: 'desc' }]
      } else {
        sortColumns.value = []
      }
    } else {
      sortColumns.value = [{ col: colIdx, dir: 'asc' }]
    }
  }
}

function sortIndicator(colIdx: number): string {
  const idx = sortColumns.value.findIndex((s) => s.col === colIdx)
  if (idx === -1) return ''
  const entry = sortColumns.value[idx]!
  const arrow = entry.dir === 'asc' ? '↑' : '↓'
  return sortColumns.value.length > 1 ? ` ${arrow}${idx + 1}` : ` ${arrow}`
}

// ============================================================
// Subtask 5: Column-level filters
// ============================================================
type FilterOp = '=' | '!=' | 'LIKE' | '>' | '<'
interface ColumnFilter {
  op: FilterOp
  value: string
}
const columnFilters = ref<Map<number, ColumnFilter>>(new Map())
const activeFilterCol = ref<number | null>(null)
const filterPopoverOp = ref<FilterOp>('=')
const filterPopoverValue = ref('')
const filterPopoverRef = ref<HTMLDivElement | null>(null)

function openFilterPopover(colIdx: number, event: MouseEvent) {
  event.stopPropagation()
  const existing = columnFilters.value.get(colIdx)
  if (existing) {
    filterPopoverOp.value = existing.op
    filterPopoverValue.value = existing.value
  } else {
    filterPopoverOp.value = '='
    filterPopoverValue.value = ''
  }
  activeFilterCol.value = colIdx
}

function applyFilter() {
  if (activeFilterCol.value === null) return
  if (filterPopoverValue.value === '') {
    columnFilters.value.delete(activeFilterCol.value)
  } else {
    columnFilters.value.set(activeFilterCol.value, {
      op: filterPopoverOp.value,
      value: filterPopoverValue.value,
    })
  }
  columnFilters.value = new Map(columnFilters.value)
  activeFilterCol.value = null
}

function clearFilter(colIdx: number) {
  columnFilters.value.delete(colIdx)
  columnFilters.value = new Map(columnFilters.value)
  activeFilterCol.value = null
}

function clearAllFilters() {
  columnFilters.value = new Map()
}

function filterMatch(value: unknown, filter: ColumnFilter): boolean {
  const strVal = value === null || value === undefined ? 'NULL' : String(value)
  const needle = filter.value
  switch (filter.op) {
    case '=': return strVal === needle
    case '!=': return strVal !== needle
    case 'LIKE': {
      // Simple SQL-like pattern: % matches any sequence, _ matches one char
      const regex = new RegExp(
        '^' + needle.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/%/g, '.*').replace(/_/g, '.') + '$',
        'i',
      )
      return regex.test(strVal)
    }
    case '>': return strVal > needle
    case '<': return strVal < needle
    default: return true
  }
}

const hasActiveFilters = computed(() => columnFilters.value.size > 0)

// ============================================================
// Subtask 6: Multi-row selection + Cell detail panel
// ============================================================
const selectedRows = ref<Set<number>>(new Set())
const lastSelectedRow = ref<number | null>(null)
const detailCell = ref<{ row: number; col: number; value: unknown; colName: string } | null>(null)

function toggleRowSelect(rowIdx: number, ctrlKey: boolean, shiftKey: boolean) {
  if (shiftKey && lastSelectedRow.value !== null) {
    // Range select
    const start = Math.min(lastSelectedRow.value, rowIdx)
    const end = Math.max(lastSelectedRow.value, rowIdx)
    const newSet = new Set(selectedRows.value)
    for (let i = start; i <= end; i++) newSet.add(i)
    selectedRows.value = newSet
  } else if (ctrlKey) {
    // Toggle single row
    const newSet = new Set(selectedRows.value)
    if (newSet.has(rowIdx)) newSet.delete(rowIdx)
    else newSet.add(rowIdx)
    selectedRows.value = newSet
  } else {
    // Single select
    if (selectedRows.value.size === 1 && selectedRows.value.has(rowIdx)) {
      selectedRows.value = new Set()
    } else {
      selectedRows.value = new Set([rowIdx])
    }
  }
  lastSelectedRow.value = rowIdx
}

function isRowSelected(rowIdx: number): boolean {
  return selectedRows.value.has(rowIdx)
}

function exportSelectedRows() {
  if (!props.result) return
  const rows = Array.from(selectedRows.value).sort((a, b) => a - b)
  const columns = props.result.columns.map((c) => c.name)
  const lines = [columns.join('\t')]
  for (const idx of rows) {
    const row = props.result.rows[idx]
    if (row) lines.push(row.map((cell) => formatCell(cell)).join('\t'))
  }
  const blob = new Blob([lines.join('\n')], { type: 'text/tab-separated-values;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'selected-rows.tsv'
  a.click()
  URL.revokeObjectURL(url)
}

function openCellDetail(row: number, col: number, value: unknown, colName: string) {
  detailCell.value = { row, col, value, colName }
}

function closeCellDetail() {
  detailCell.value = null
}

function formatJson(str: string): string {
  try {
    return JSON.stringify(JSON.parse(str), null, 2)
  } catch {
    return str
  }
}

function isJsonValue(value: unknown): boolean {
  if (typeof value !== 'string') return false
  const trimmed = value.trim()
  if (!trimmed) return false
  return (trimmed.startsWith('{') && trimmed.endsWith('}')) ||
    (trimmed.startsWith('[') && trimmed.endsWith(']'))
}

// ============================================================
// Processed rows: sort → filter
// ============================================================
const filteredRows = computed(() => {
  if (!props.result) return [] as unknown[][]
  let rows = props.result.rows
  // Apply column filters
  if (columnFilters.value.size > 0) {
    rows = rows.filter((row) => {
      for (const [colIdx, filter] of columnFilters.value) {
        if (!filterMatch(row[colIdx], filter)) return false
      }
      return true
    })
  }
  return rows
})

const processedRows = computed(() => {
  const rows = filteredRows.value
  if (sortColumns.value.length === 0) return rows
  const sorters = sortColumns.value.map((s) => ({
    col: s.col,
    dir: s.dir === 'asc' ? 1 : -1,
  }))
  return [...rows].sort((a, b) => {
    for (const { col, dir } of sorters) {
      const av = a[col]
      const bv = b[col]
      if (av === null || av === undefined) return 1
      if (bv === null || bv === undefined) return -1
      if (typeof av === 'number' && typeof bv === 'number') {
        if (av !== bv) return (av - bv) * dir
      } else {
        const cmp = String(av).localeCompare(String(bv))
        if (cmp !== 0) return cmp * dir
      }
    }
    return 0
  })
})

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

// ============================================================
// Inline editing (preserved from original)
// ============================================================
const editingCell = ref<{ row: number; col: number } | null>(null)
const editValue = ref('')
const editInput = ref<HTMLInputElement | null>(null)
const editHistory = ref<Map<string, unknown>>(new Map())

function cellKey(row: number, col: number): string {
  return `${row}:${col}`
}

function isEditing(row: number, col: number): boolean {
  return editingCell.value?.row === row && editingCell.value?.col === col
}

function isEdited(row: number, col: number): boolean {
  return editHistory.value.has(cellKey(row, col))
}

function onCellDblClick(row: number, col: number, value: unknown) {
  editingCell.value = { row, col }
  editValue.value = value === null || value === undefined ? '' : String(value)
  nextTick(() => {
    editInput.value?.focus()
    editInput.value?.select()
  })
}

function onEditKeyDown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    finishEdit()
  } else if (e.key === 'Escape') {
    cancelEdit()
  }
}

function finishEdit() {
  if (!editingCell.value || !props.result) return
  const { row, col } = editingCell.value
  const oldValue = props.result.rows[row]?.[col]
  const newValue = editValue.value

  // Only record if value actually changed
  if (String(oldValue) !== newValue) {
    const key = cellKey(row, col)
    editHistory.value.set(key, { oldValue, newValue })
  }

  editingCell.value = null
}

function cancelEdit() {
  editingCell.value = null
}

function getEditedValue(row: number, col: number, original: unknown): unknown {
  const key = cellKey(row, col)
  const edit = editHistory.value.get(key)
  if (edit && typeof edit === 'object' && 'newValue' in edit) {
    return (edit as { newValue: unknown }).newValue
  }
  return original
}

function getChanges(): EditCell[] {
  if (!props.result) return []
  const changes: EditCell[] = []
  editHistory.value.forEach((edit, key) => {
    const parts = key.split(':')
    const row = parseInt(parts[0] || '0', 10)
    const col = parseInt(parts[1] || '0', 10)
    if (edit && typeof edit === 'object' && 'oldValue' in edit && 'newValue' in edit) {
      const typedEdit = edit as { oldValue: unknown; newValue: unknown }
      changes.push({
        rowIndex: row,
        colIndex: col,
        oldValue: typedEdit.oldValue,
        newValue: typedEdit.newValue,
      })
    }
  })
  return changes
}

function applyChanges() {
  const changes = getChanges()
  if (changes.length > 0) {
    emit('apply', changes)
    editHistory.value.clear()
  }
}

function discardChanges() {
  editHistory.value.clear()
  emit('discard')
}

const hasChanges = computed(() => editHistory.value.size > 0)
</script>

<template>
  <div class="result-grid" @click.self="activeFilterCol = null">
    <!-- Loading -->
    <div v-if="loading" class="result-grid-loading">
      <div class="spinner" />
      <span>{{ t('sql.executing') }}</span>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="result-grid-error">
      <span class="error-icon">✕</span>
      <span class="error-msg">{{ error }}</span>
    </div>

    <!-- Empty -->
    <div v-else-if="isEmpty" class="result-grid-empty">{{ t('sql.noResults') }}</div>

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
              @click="toggleSort(ci, $event.shiftKey)"
            >
              <span class="col-name">
                {{ col.name }}<span class="sort-indicator">{{ sortIndicator(ci) }}</span>
              </span>
              <span
                class="filter-icon"
                title="Filter"
                @click="openFilterPopover(ci, $event)"
              >🔍</span>
              <span class="col-type muted">{{ col.data_type }}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(row, ri) in processedRows"
            :key="ri"
            class="data-row"
            :class="{ 'row-selected': isRowSelected(ri) }"
            @click="toggleRowSelect(ri, $event.ctrlKey || $event.metaKey, $event.shiftKey)"
          >
            <td class="row-num">{{ ri + 1 }}</td>
            <td
              v-for="(cell, ci) in row"
              :key="ci"
              :class="['data-cell', cellClass(cell), { 'cell-edited': isEdited(ri, ci) }]"
              :title="formatCell(cell)"
              @dblclick.stop="openCellDetail(ri, ci, cell, result!.columns[ci]?.name ?? '')"
            >
              <template v-if="isEditing(ri, ci)">
                <input
                  ref="editInput"
                  class="cell-edit-input"
                  :value="editValue"
                  @input="editValue = ($event.target as HTMLInputElement).value"
                  @keydown="onEditKeyDown"
                  @blur="finishEdit"
                />
              </template>
              <template v-else>
                {{ formatCell(getEditedValue(ri, ci, cell)) }}
              </template>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Filter popover -->
    <div
      v-if="activeFilterCol !== null"
      class="filter-popover-overlay"
      @click="activeFilterCol = null"
    >
      <div ref="filterPopoverRef" class="filter-popover" @click.stop>
        <div class="filter-popover-title">Filter: {{ result!.columns[activeFilterCol]?.name }}</div>
        <div class="filter-popover-body">
          <select v-model="filterPopoverOp" class="filter-select">
            <option value="=">=</option>
            <option value="!=">≠ (!=)</option>
            <option value="LIKE">LIKE</option>
            <option value=">">&gt;</option>
            <option value="<">&lt;</option>
          </select>
          <input
            v-model="filterPopoverValue"
            class="filter-input"
            placeholder="Value…"
            @keydown.enter="applyFilter"
          />
        </div>
        <div class="filter-popover-actions">
          <button class="filter-btn-clear" @click="clearFilter(activeFilterCol!)">Clear</button>
          <button class="filter-btn-apply" @click="applyFilter">Apply</button>
        </div>
      </div>
    </div>

    <!-- Cell detail panel -->
    <div v-if="detailCell" class="detail-overlay" @click="closeCellDetail">
      <div class="detail-panel" @click.stop>
        <div class="detail-header">
          <span class="detail-title">{{ detailCell.colName }} (Row {{ detailCell.row + 1 }})</span>
          <button class="detail-close" @click="closeCellDetail">✕</button>
        </div>
        <div class="detail-body">
          <pre v-if="isJsonValue(detailCell.value)" class="detail-json">{{ formatJson(String(detailCell.value)) }}</pre>
          <pre v-else class="detail-text">{{ formatCell(detailCell.value) }}</pre>
        </div>
      </div>
    </div>

    <!-- Status bar -->
    <div v-if="result" class="result-grid-status">
      <span>{{ result.rows.length }} {{ t('sql.rows') }}</span>
      <span v-if="hasActiveFilters" class="filter-status"> · Filtered {{ processedRows.length }}/{{ result.rows.length }} rows</span>
      <span v-if="selectedRows.size > 0" class="selection-status"> · {{ selectedRows.size }} rows selected</span>
      <span v-if="result.affected_rows">· {{ result.affected_rows }} {{ t('sql.affected') }}</span>
      <span>· {{ result.elapsed_ms }}ms</span>
      <span class="status-spacer" />
      <button v-if="hasActiveFilters" class="clear-filters-btn" @click="clearAllFilters">Clear Filters</button>
      <button v-if="selectedRows.size > 0" class="export-selected-btn" @click="exportSelectedRows">Export Selected</button>
      <template v-if="hasData && hasChanges">
        <button class="apply-btn" @click="applyChanges">{{ t('sql.apply') }}</button>
        <button class="discard-btn" @click="discardChanges">{{ t('sql.discard') }}</button>
      </template>
      <button v-if="hasData" class="export-btn" @click="emit('export')">{{ t('sql.export') }}</button>
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

.sort-indicator {
  font-size: var(--text-xs);
  color: var(--accent);
}

.filter-icon {
  font-size: 10px;
  margin-left: 2px;
  cursor: pointer;
  opacity: 0.5;
  transition: opacity var(--transition);
  vertical-align: middle;
}

.filter-icon:hover {
  opacity: 1;
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

.row-selected {
  background: rgba(56, 139, 253, 0.12) !important;
}

.row-selected:hover {
  background: rgba(56, 139, 253, 0.18) !important;
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

.filter-status {
  color: var(--accent);
}

.selection-status {
  color: var(--info);
}

.clear-filters-btn,
.export-selected-btn {
  padding: 2px var(--space-2);
  background: none;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: background var(--transition), border-color var(--transition);
}

.clear-filters-btn:hover,
.export-selected-btn:hover {
  background: var(--bg-hover);
  border-color: var(--accent);
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

/* ---- inline editing ---- */
.cell-edited {
  background: rgba(210, 153, 34, 0.15) !important;
}

.data-cell {
  cursor: default;
  position: relative;
}

.data-cell:hover {
  background: var(--bg-hover);
}

.cell-edit-input {
  width: 100%;
  padding: 0;
  margin: 0;
  background: var(--bg-surface);
  border: 1px solid var(--accent);
  border-radius: 2px;
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  outline: none;
}

.apply-btn {
  padding: 2px var(--space-2);
  background: var(--success);
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-on-accent);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: opacity var(--transition);
}

.apply-btn:hover {
  opacity: 0.9;
}

.discard-btn {
  padding: 2px var(--space-2);
  background: none;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: background var(--transition);
}

.discard-btn:hover {
  background: var(--bg-hover);
}

/* ---- filter popover ---- */
.filter-popover-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
}

.filter-popover {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--space-3);
  min-width: 240px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  z-index: 101;
}

.filter-popover-title {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: var(--space-2);
}

.filter-popover-body {
  display: flex;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
}

.filter-select {
  padding: 4px 6px;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  outline: none;
}

.filter-select:focus {
  border-color: var(--accent);
}

.filter-input {
  flex: 1;
  padding: 4px 6px;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  outline: none;
}

.filter-input:focus {
  border-color: var(--accent);
}

.filter-popover-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
}

.filter-btn-clear,
.filter-btn-apply {
  padding: 3px var(--space-2);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  cursor: pointer;
  border: 1px solid var(--border);
  transition: background var(--transition);
}

.filter-btn-clear {
  background: none;
  color: var(--text-primary);
}

.filter-btn-clear:hover {
  background: var(--bg-hover);
}

.filter-btn-apply {
  background: var(--accent);
  color: var(--text-on-accent);
  border-color: var(--accent);
}

.filter-btn-apply:hover {
  opacity: 0.9;
}

/* ---- cell detail panel ---- */
.detail-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.detail-panel {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  width: 600px;
  max-width: 90vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
}

.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3);
  border-bottom: 1px solid var(--border);
}

.detail-title {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
}

.detail-close {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-lg);
  cursor: pointer;
  padding: 0 var(--space-1);
  line-height: 1;
  transition: color var(--transition);
}

.detail-close:hover {
  color: var(--text-primary);
}

.detail-body {
  flex: 1;
  overflow: auto;
  padding: var(--space-3);
}

.detail-json {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--info);
  white-space: pre-wrap;
  word-break: break-word;
  margin: 0;
  line-height: 1.5;
}

.detail-text {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
  margin: 0;
  line-height: 1.5;
}
</style>
