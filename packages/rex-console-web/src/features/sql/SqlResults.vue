<template>
  <div class="sql-results">
    <div class="results-header">
      <div class="results-tabs">
        <span class="results-tab active">{{ t('sql.resultTab') }}</span>
      </div>
    </div>
    <div class="results-table-wrap">
      <table v-if="result && result.rows.length > 0" class="results-table">
        <thead>
          <tr>
            <th>#</th>
            <th v-for="col in result.columns" :key="col.name">{{ col.name }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(row, i) in result.rows" :key="i" @contextmenu.prevent="handleRowContextMenu($event, i)">
            <td class="text-muted">{{ i + 1 }}</td>
            <td v-for="(cell, j) in row" :key="j"
              :class="cellClass(cell)"
              @contextmenu.prevent="handleCellContextMenu($event, i, j)">
              {{ formatCell(cell) }}
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
    <div v-if="result" class="results-footer">
      <span>{{ t('sql.rows', { count: result.rows.length }) }} · {{ t('sql.elapsed', { time: (result.elapsed_ms / 1000).toFixed(3) }) }}</span>
      <div class="results-footer-actions">
        <button
          v-if="result.rows.length > 0"
          class="btn btn-ghost btn-xs"
          @click="handleExportCsv"
        >⬇ CSV</button>
        <button
          v-if="result.rows.length > 0"
          class="btn btn-ghost btn-xs"
          @click="handleExportJson"
        >⬇ JSON</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'
import type { SqlResult } from '@/api/sql'
import { exportCsv, exportJson } from './result-export'

const { t } = useI18n()
const { show: showMenu } = useContextMenu()

const props = defineProps<{
  result: SqlResult | null
  loading: boolean
}>()

const emit = defineEmits<{
  (e: 'sort', column: string, direction: 'asc' | 'desc'): void
  (e: 'generateSql', sql: string): void
}>()

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
  columns.forEach((c, i) => { obj[c.name] = row[i] })
  return obj
}

function formatValStr(val: unknown): string {
  return val === null ? 'NULL' : typeof val === 'string' ? `'${val.replace(/'/g, "''")}'` : String(val)
}

function generateUpdateSql(row: unknown[], columns: { name: string }[]): string {
  const setClauses = columns.map((c, i) => `  ${c.name} = ${formatValStr(row[i])}`)
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

function handleCellContextMenu(event: MouseEvent, rowIdx: number, colIdx: number) {
  if (!props.result) return
  const { columns, rows } = props.result
  const row = rows[rowIdx]
  const cell = row[colIdx]
  const colName = columns[colIdx]?.name ?? `col${colIdx}`

  showMenu(event, [
    { label: t('sql.result.ctx.copyRow'), action: () => copyToClipboard(rowToTsv(row)) },
    { label: t('sql.result.ctx.copyCell'), action: () => copyToClipboard(cell === null ? 'NULL' : String(cell)) },
    { label: t('sql.result.ctx.copyColumn'), action: () => copyToClipboard(rows.map((r) => r[colIdx] === null ? 'NULL' : String(r[colIdx])).join('\n')) },
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

function handleRowContextMenu(event: MouseEvent, rowIdx: number) {
  if (!props.result) return
  const { columns, rows } = props.result
  const row = rows[rowIdx]

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
</style>
