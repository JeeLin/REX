<template>
  <div class="sql-results">
    <div class="results-header">
      <div class="results-tabs">
        <span
          class="results-tab"
          :class="{ active: activeTab === 'results' }"
          @click="activeTab = 'results'"
        >{{ t('sql.resultTab') }}</span>
        <span
          class="results-tab"
          :class="{ active: activeTab === 'message' }"
          @click="activeTab = 'message'"
        >{{ t('sql.messageTab') }}</span>
        <span
          v-if="showExplainTab"
          class="results-tab"
          :class="{ active: activeTab === 'explain' }"
          @click="handleExplainTab"
        >{{ t('sql.explainTab') }}</span>
      </div>
    </div>

    <!-- Message Tab -->
    <div v-if="activeTab === 'message'" class="results-message-wrap">
      <div v-if="message" class="results-message" :class="{ 'is-error': isError }">{{ message }}</div>
      <div v-else class="results-empty">{{ t('sql.noMessage') }}</div>
    </div>

    <!-- Results Tab -->
    <div v-if="activeTab === 'results'" class="results-table-wrap">
      <table v-if="result && result.rows.length > 0" class="results-table">
        <thead>
          <tr>
            <th>
              <input
                type="checkbox"
                :checked="result && selectedRows.size === paginatedRows.length && paginatedRows.length > 0"
                @change="toggleSelectAll"
              />
            </th>
            <th>#</th>
            <th
              v-for="(col, colIdx) in result.columns"
              :key="col.name"
              class="sortable-th"
              @click="handleHeaderSort(colIdx)"
            >
              {{ col.name }}
              <span v-if="sortColumn === colIdx" class="sort-indicator">{{ sortDirection === 'asc' ? ' ↑' : ' ↓' }}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(row, i) in paginatedRows" :key="i"
            :class="{ 'row-selected': selectedRow === i, 'row-checked': selectedRows.has((currentPage - 1) * pageSize + i) }"
            @click="handleRowClick(i)"
            @contextmenu.prevent="handleRowContextMenu($event, i)"
          >
            <td class="checkbox-cell">
              <input
                type="checkbox"
                :checked="selectedRows.has((currentPage - 1) * pageSize + i)"
                @click.stop="toggleRowSelect((currentPage - 1) * pageSize + i)"
              />
            </td>
            <td class="text-muted">{{ i + 1 + (currentPage - 1) * pageSize }}</td>
            <td
              v-for="(cell, j) in row" :key="j"
              :class="cellClass(cell)"
              @contextmenu.prevent="handleCellContextMenu($event, i, j)"
              @dblclick="handleCellDblClick(i, j, cell)"
            >
              <template v-if="editingCell && editingCell.row === i && editingCell.col === j">
                <input
                  v-model="editValue"
                  class="cell-editor"
                  @keydown.enter.prevent="saveEdit"
                  @keydown.escape="cancelEdit"
                  @blur="saveEdit"
                />
              </template>
              <template v-else>
                {{ formatCell(cell) }}
              </template>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-else-if="loading" class="results-empty">
        <span class="spinner"></span>
        {{ t('sql.executing') }}
      </div>
      <div v-else-if="result && result.rows.length === 0" class="results-empty">
        {{ t('sql.noResult') }}
      </div>
      <div v-else class="results-empty">
        {{ t('sql.noResult') }}
      </div>
    </div>

    <!-- Explain Tab -->
    <div v-if="activeTab === 'explain'" class="results-table-wrap">
      <div v-if="explainLoading" class="results-empty">
        <span class="spinner"></span>
        {{ t('sql.executing') }}
      </div>
      <div v-else-if="explainError" class="results-empty" style="color: var(--danger)">
        {{ explainError }}
      </div>
      <div v-else-if="explainResult && explainResult.rows.length > 0" class="results-table-wrap">
        <table class="results-table">
          <thead>
            <tr>
              <th v-for="col in explainResult.columns" :key="col">{{ col }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, i) in explainResult.rows" :key="i">
              <td v-for="(cell, j) in row" :key="j" :class="cellClass(cell)">
                {{ formatCell(cell) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div v-else class="results-empty">
        {{ t('sql.noResult') }}
      </div>
      <!-- Raw output fallback -->
      <div v-if="explainResult && explainResult.raw_output && explainResult.rows.length === 0" class="results-message-wrap">
        <div class="results-message">{{ explainResult.raw_output }}</div>
      </div>
    </div>

    <div v-if="result" class="results-footer">
      <span>{{ t('sql.rows', { count: result.rows.length }) }} · {{ t('sql.elapsed', { time: (result.elapsed_ms / 1000).toFixed(3) }) }}</span>
      <div class="results-footer-actions">
        <button
          v-if="result.rows.length > 0"
          class="btn btn-ghost btn-xs"
          @click="handleCopyAll"
        >
          📋 {{ t('sql.result.copy') }}
        </button>
        <button
          v-if="result.rows.length > 0"
          class="btn btn-ghost btn-xs"
          @click="handleExportCsv"
        >
          ⬇ CSV
        </button>
        <button
          v-if="result.rows.length > 0"
          class="btn btn-ghost btn-xs"
          @click="handleExportJson"
        >
          ⬇ JSON
        </button>
        <template v-if="selectedRows.size > 0">
          <div class="results-sep"></div>
          <span class="selected-count">{{ t('sql.result.selected', { count: selectedRows.size }) }}</span>
          <button class="btn btn-ghost btn-xs" @click="emit('generateSql', generateBatchUpdateSql())">
            ✏ {{ t('sql.result.batchUpdate') }}
          </button>
          <button class="btn btn-ghost btn-xs" @click="emit('generateSql', generateBatchDeleteSql())">
            🗑 {{ t('sql.result.batchDelete') }}
          </button>
        </template>
      </div>
    </div>

    <!-- Pagination Controls -->
    <div v-if="result && result.rows.length > pageSize" class="results-footer">
      <div class="page-size-selector">
        <label>{{ t('sql.pagination.pageSize') }}:</label>
        <select v-model="pageSize" class="page-size-select">
          <option value="50">50</option>
          <option value="100">100</option>
          <option value="500">500</option>
        </select>
      </div>
      <div class="page-navigation">
        <button
          class="btn btn-ghost btn-xs"
          :disabled="currentPage === 1"
          @click="currentPage = Math.max(1, currentPage - 1)"
        >
          {{ t('sql.pagination.prev') }}
        </button>
        <span class="page-info">
          {{ t('sql.pagination.page', { current: currentPage, total: totalPages }) }}
        </span>
        <button
          class="btn btn-ghost btn-xs"
          :disabled="currentPage === totalPages"
          @click="currentPage = Math.min(totalPages, currentPage + 1)"
        >
          {{ t('sql.pagination.next') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'
import { explainSql, type ExplainResult, type SqlResult } from '@/api/sql'
import { exportCsv, exportJson, copyTsv } from './result-export'

const { t } = useI18n()
const { show: showMenu } = useContextMenu()

const props = defineProps<{
  result: SqlResult | null
  loading: boolean
  message?: string
  isError?: boolean
  resourceId?: string
  currentSql?: string
}>()

const emit = defineEmits<{
  (e: 'sort', column: string, direction: 'asc' | 'desc'): void
  (e: 'generateSql', sql: string): void
}>()

const activeTab = ref<'results' | 'message' | 'explain'>('results')
const currentPage = ref(1)
const pageSize = ref(50)
const sortColumn = ref<number | null>(null)
const sortDirection = ref<'asc' | 'desc'>('asc')
const selectedRow = ref<number | null>(null)

function handleRowClick(rowIdx: number) {
  selectedRow.value = selectedRow.value === rowIdx ? null : rowIdx
}

// ── Inline cell editing ──
const editingCell = ref<{ row: number; col: number } | null>(null)
const editValue = ref('')

function handleCellDblClick(rowIdx: number, colIdx: number, cell: unknown) {
  editingCell.value = { row: rowIdx, col: colIdx }
  editValue.value = cell === null ? 'NULL' : String(cell)
}

function cancelEdit() {
  editingCell.value = null
  editValue.value = ''
}

function saveEdit() {
  if (!editingCell.value || !props.result || !props.resourceId) {
    cancelEdit()
    return
  }
  const { row, col } = editingCell.value
  const colName = props.result.columns[col]?.name
  if (!colName) {
    cancelEdit()
    return
  }
  const cellValue = props.result.rows[row]?.[col]
  if (cellValue === undefined) {
    cancelEdit()
    return
  }
  // Generate UPDATE SQL
  const newVal = editValue.value === 'NULL' ? 'NULL' : `'${editValue.value.replace(/'/g, "''")}'`
  const oldVal = cellValue === null ? 'IS NULL' : `= '${String(cellValue).replace(/'/g, "''")}'`
  const sql = `UPDATE table_name SET ${colName} = ${newVal} WHERE ${colName} ${oldVal};`
  emit('generateSql', sql)
}

// ── Batch operations ──
const selectedRows = ref<Set<number>>(new Set())

function toggleRowSelect(rowIdx: number) {
  if (selectedRows.value.has(rowIdx)) {
    selectedRows.value.delete(rowIdx)
  } else {
    selectedRows.value.add(rowIdx)
  }
  selectedRows.value = new Set(selectedRows.value)
}

function toggleSelectAll() {
  if (!props.result) return
  if (selectedRows.value.size === paginatedRows.value.length) {
    selectedRows.value = new Set()
  } else {
    const allRowIndices = paginatedRows.value.map((_, i) => (currentPage.value - 1) * pageSize.value + i)
    selectedRows.value = new Set(allRowIndices)
  }
}

function generateBatchDeleteSql(): string {
  if (!props.result || selectedRows.value.size === 0) return ''
  const { columns, rows } = props.result
  const pkCol = columns[0]?.name
  if (!pkCol) return ''
  const conditions = Array.from(selectedRows.value)
    .map(i => `${pkCol} = ${rows[i]?.[0]}`)
    .join(' OR ')
  return `DELETE FROM table_name WHERE ${conditions};`
}

function generateBatchUpdateSql(): string {
  if (!props.result || selectedRows.value.size === 0) return ''
  const { columns, rows } = props.result
  const pkCol = columns[0]?.name
  if (!pkCol) return ''
  const conditions = Array.from(selectedRows.value)
    .map(i => `${pkCol} = ${rows[i]?.[0]}`)
    .join(' OR ')
  return `UPDATE table_name SET column = value WHERE ${conditions};`
}

// Explain tab state
const showExplainTab = ref(false)
const explainResult = ref<ExplainResult | null>(null)
const explainLoading = ref(false)
const explainError = ref('')

async function handleExplainTab() {
  activeTab.value = 'explain'
  if (explainResult.value || explainLoading.value) return
  if (!props.resourceId || !props.currentSql) {
    explainError.value = t('sql.explainError')
    return
  }
  explainLoading.value = true
  explainError.value = ''
  try {
    explainResult.value = await explainSql(props.resourceId, props.currentSql)
  } catch (e: unknown) {
    explainError.value = (e instanceof Error ? e.message : String(e)) || t('sql.explainError')
  } finally {
    explainLoading.value = false
  }
}

// Reset state when result changes
watch(() => props.result, () => {
  showExplainTab.value = true
  explainResult.value = null
  explainError.value = ''
  if (activeTab.value === 'explain') {
    activeTab.value = 'results'
  }
  currentPage.value = 1
  selectedRow.value = null
  sortColumn.value = null
  sortDirection.value = 'asc'
  if (props.isError) {
    activeTab.value = 'message'
  }
})

// 排序逻辑
function handleHeaderSort(colIdx: number) {
  if (sortColumn.value === colIdx) {
    if (sortDirection.value === 'asc') {
      sortDirection.value = 'desc'
    } else {
      // 第三次点击取消排序
      sortColumn.value = null
      sortDirection.value = 'asc'
    }
  } else {
    sortColumn.value = colIdx
    sortDirection.value = 'asc'
  }
  currentPage.value = 1
}

const sortedRows = computed(() => {
  if (!props.result) return []
  if (sortColumn.value === null) return props.result.rows
  const colIdx = sortColumn.value
  const dir = sortDirection.value === 'asc' ? 1 : -1
  return [...props.result.rows].sort((a, b) => {
    const va = a[colIdx]!
    const vb = b[colIdx]!
    if (va === null && vb === null) return 0
    if (va === null) return 1
    if (vb === null) return -1
    if (typeof va === 'number' && typeof vb === 'number') return (va - vb) * dir
    return String(va).localeCompare(String(vb)) * dir
  })
})

// 分页逻辑
const totalPages = computed(() => {
  if (!props.result) return 1
  return Math.ceil(sortedRows.value.length / pageSize.value)
})

const paginatedRows = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  const end = start + pageSize.value
  return sortedRows.value.slice(start, end)
})

function cellClass(cell: unknown): string {
  if (cell === null || cell === undefined) return 'cell-null'
  if (typeof cell === 'number') return 'cell-number'
  return ''
}

function formatCell(cell: unknown): string {
  if (cell === null || cell === undefined) return 'NULL'
  if (typeof cell === 'number') {
    return cell.toLocaleString()
  }
  return String(cell)
}

function copyToClipboard(text: string) {
  navigator.clipboard.writeText(text)
}

function rowToTsv(row: unknown[]): string {
  return row.map((c) => c === null ? 'NULL' : String(c)).join('\t')
}

function rowToJson(row: unknown[], columns: { name: string }[]): Record<string, unknown> {
  const obj: Record<string, unknown> = {}
  columns.forEach((c, i) => { obj[c.name] = row[i]! })
  return obj
}

function formatValStr(val: unknown): string {
  return val === null ? 'NULL' : typeof val === 'string' ? `'${val.replace(/'/g, "''")}'` : String(val)
}

function generateUpdateSql(row: unknown[], columns: { name: string }[]): string {
  const setClauses = columns.map((c, i) => `  ${c.name} = ${formatValStr(row[i]!)}`)
  return `UPDATE table_name\nSET\n${setClauses.join(',\n')}\nWHERE id = ${row[0] ?? '...'};`
}

function generateDeleteSql(row: unknown[]): string {
  return `DELETE FROM table_name\nWHERE id = ${row[0] ?? '...'};`
}

function handleExportCsv() {
  if (!props.result) return
  exportCsv(props.result.columns, props.result.rows)
}

function handleExportJson() {
  if (!props.result) return
  exportJson(props.result.columns, props.result.rows)
}

async function handleCopyAll() {
  if (!props.result) return
  const ok = await copyTsv(props.result.columns, props.result.rows)
  if (ok) {
    // Simple toast feedback
    const msg = document.createElement('div')
    msg.textContent = t('sql.result.copySuccess')
    msg.className = 'toast-notification'
    document.body.appendChild(msg)
    setTimeout(() => msg.remove(), 1500)
  }
}

function handleCellContextMenu(event: MouseEvent, paginatedIdx: number, colIdx: number) {
  if (!props.result) return
  const { columns, rows } = props.result
  const rowIdx = (currentPage.value - 1) * pageSize.value + paginatedIdx
  const row = rows[rowIdx]!
  const cell = row[colIdx]
  const colName = columns[colIdx]?.name ?? `col${colIdx}`

  showMenu(event, [
    { label: t('sql.result.ctx.copyRow'), action: () => copyToClipboard(rowToTsv(row)) },
    { label: t('sql.result.ctx.copyCell'), action: () => copyToClipboard(cell === null ? 'NULL' : String(cell)) },
    { label: t('sql.result.ctx.copyColumn'), action: () => copyToClipboard(rows.map((r) => r[colIdx]! === null ? 'NULL' : String(r[colIdx]!)).join('\n')) },
    { label: t('sql.result.ctx.copyJson'), action: () => copyToClipboard(JSON.stringify(rowToJson(row, columns), null, 2)) },
    { separator: true },
    { label: t('sql.result.ctx.sortAsc'), action: () => emit('sort', colName, 'asc') },
    { label: t('sql.result.ctx.sortDesc'), action: () => emit('sort', colName, 'desc') },
    { separator: true },
    { label: t('sql.result.ctx.exportRow'), action: () => copyToClipboard(JSON.stringify(rowToJson(row, columns))) },
    { label: t('sql.result.ctx.generateUpdate'), action: () => emit('generateSql', generateUpdateSql(row, columns)) },
    { label: t('sql.result.ctx.generateDelete'), action: () => emit('generateSql', generateDeleteSql(row)) },
  ])
}

function handleRowContextMenu(event: MouseEvent, paginatedIdx: number) {
  if (!props.result) return
  const { columns, rows } = props.result
  const rowIdx = (currentPage.value - 1) * pageSize.value + paginatedIdx
  const row = rows[rowIdx]!

  showMenu(event, [
    { label: t('sql.result.ctx.copyRow'), action: () => copyToClipboard(rowToTsv(row)) },
    { label: t('sql.result.ctx.copyJson'), action: () => copyToClipboard(JSON.stringify(rowToJson(row, columns), null, 2)) },
    { separator: true },
    { label: t('sql.result.ctx.generateUpdate'), action: () => emit('generateSql', generateUpdateSql(row, columns)) },
    { label: t('sql.result.ctx.generateDelete'), action: () => emit('generateSql', generateDeleteSql(row)) },
  ])
}
</script>

<style scoped>
.sql-results {
  flex: 1;
  min-height: 120px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.results-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-sm) var(--sp-md);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.results-tabs {
  display: flex;
  gap: var(--sp-md);
  font-size: var(--fs-sm);
}

.results-tab {
  color: var(--text-secondary);
  cursor: pointer;
}

.results-tab.active {
  color: var(--text-primary);
  font-weight: 600;
}

.results-table-wrap {
  flex: 1;
  overflow: auto;
}

.results-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--fs-sm);
  font-family: var(--font-mono);
}

.results-table th {
  position: sticky;
  top: 0;
  background: var(--bg-elevated);
  padding: var(--sp-sm) var(--sp-md);
  text-align: left;
  font-weight: 600;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
}

.results-table th.sortable-th {
  cursor: pointer;
  user-select: none;
}

.results-table th.sortable-th:hover {
  color: var(--text-primary);
}

.sort-indicator {
  color: var(--accent);
  font-size: 10px;
}

.checkbox-cell {
  width: 36px;
  text-align: center;
  padding: var(--sp-sm) !important;
}

.checkbox-cell input[type="checkbox"] {
  cursor: pointer;
  accent-color: var(--accent);
}

.row-checked td {
  background: rgba(232, 145, 45, 0.08) !important;
}

.results-sep {
  width: 1px;
  height: 16px;
  background: var(--border);
  margin: 0 var(--sp-sm);
}

.selected-count {
  font-size: var(--fs-xs);
  color: var(--accent);
  margin-right: var(--sp-xs);
}

.results-table td {
  padding: var(--sp-sm) var(--sp-md);
  border-bottom: 1px solid var(--border);
  color: var(--text-primary);
  white-space: nowrap;
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.results-table tr:hover td {
  background: var(--bg-hover);
}

.results-table tbody tr:nth-child(even) td {
  background: rgba(255, 255, 255, 0.02);
}

.results-table tr.row-selected td {
  background: rgba(88, 166, 255, 0.1) !important;
}

.results-table .cell-null {
  color: var(--text-muted);
  font-style: italic;
}

.results-table .cell-number {
  color: var(--accent);
}

.text-muted {
  color: var(--text-muted);
}

.cell-editor {
  width: 100%;
  min-width: 80px;
  padding: 2px 4px;
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  background: var(--bg-deep);
  border: 1px solid var(--accent);
  border-radius: 3px;
  color: var(--text);
  outline: none;
}

.cell-editor:focus {
  box-shadow: 0 0 0 2px rgba(232, 145, 45, 0.2);
}

.results-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  color: var(--text-muted);
  font-size: var(--fs-sm);
  gap: var(--sp-sm);
  padding: var(--sp-xl);
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

.results-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-xs) var(--sp-md);
  background: var(--bg-surface);
  border-top: 1px solid var(--border);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  flex-shrink: 0;
}

.results-footer-actions {
  display: flex;
  gap: var(--sp-xs);
}

.btn-xs {
  height: 22px;
  padding: 0 var(--sp-sm);
  font-size: 11px;
}

.results-message-wrap {
  flex: 1;
  overflow: auto;
  padding: var(--sp-md);
}

.results-message {
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  color: var(--success);
  white-space: pre-wrap;
  word-break: break-all;
}

.results-message.is-error {
  color: var(--danger);
}

/* Pagination Controls */
.page-size-selector {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
}

.page-size-selector label {
  color: var(--text-muted);
}

.page-size-select {
  padding: 2px var(--sp-xs);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-deep);
  color: var(--text-primary);
  font-size: var(--fs-xs);
  outline: none;
  cursor: pointer;
}

.page-navigation {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.page-info {
  color: var(--text-secondary);
  font-size: var(--fs-xs);
}
</style>
