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
          <tr v-for="(row, i) in result.rows" :key="i">
            <td class="text-muted">{{ i + 1 }}</td>
            <td v-for="(cell, j) in row" :key="j" :class="cellClass(cell)">
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
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { SqlResult } from '@/api/sql'

const { t } = useI18n()

defineProps<{
  result: SqlResult | null
  loading: boolean
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
</style>
