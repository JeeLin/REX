<script setup lang="ts">
/**
 * DataCompare — 数据对比面板
 * 支持两个 SQL 查询结果的对比，高亮差异行和差异列。
 * 支持按主键列对比或按行序对比。
 */
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import Button from '@/components/ui/Button.vue'
import SqlEditor from './SqlEditor.vue'
import { clipboard } from '@/utils/clipboard'

const { t } = useI18n()

const props = defineProps<{
  sessionId: string
}>()

const emit = defineEmits<{
  close: []
}>()

// ── State ──────────────────────────────────────
const sqlLeft = ref('SELECT * FROM table_a ORDER BY id')
const sqlRight = ref('SELECT * FROM table_b ORDER BY id')
const keyColumns = ref('')
const isComparing = ref(false)
const errorMsg = ref('')

interface DiffRow {
  row_index: number
  diff_type: string
  column: string
  left_value: unknown
  right_value: unknown
}

interface CompareSummary {
  left_rows: number
  right_rows: number
  identical_rows: number
  modified_rows: number
  only_in_left: number
  only_in_right: number
}

interface ColumnInfo {
  name: string
  data_type: string
}

interface QueryResult {
  columns: ColumnInfo[]
  rows: unknown[][]
  affected_rows: number
  elapsed_ms: number
}

interface CompareResult {
  left: QueryResult
  right: QueryResult
  diffs: DiffRow[]
  summary: CompareSummary
}

const result = ref<CompareResult | null>(null)

// ── Diff tracking ──────────────────────────────
const diffMap = computed(() => {
  const map = new Map<number, Set<string>>()
  if (!result.value) return map
  for (const d of result.value.diffs) {
    if (!map.has(d.row_index)) map.set(d.row_index, new Set())
    map.get(d.row_index)!.add(d.column)
  }
  return map
})

function hasDiffRow(rowIndex: number): boolean {
  return diffMap.value.has(rowIndex)
}

function hasDiffCell(rowIndex: number, colName: string): boolean {
  const s = diffMap.value.get(rowIndex)
  return !!s && (s.has(colName) || s.has('*'))
}

function rowDiffType(rowIndex: number): string {
  const d = result.value?.diffs.find(d => d.row_index === rowIndex && d.column === '*')
  return d?.diff_type ?? 'modified'
}

// ── Compare ────────────────────────────────────
async function onCompare() {
  if (!sqlLeft.value.trim() || !sqlRight.value.trim()) return
  isComparing.value = true
  errorMsg.value = ''
  result.value = null

  try {
    const keys = keyColumns.value.trim()
      ? keyColumns.value.split(',').map(s => s.trim()).filter(Boolean)
      : undefined

    const resp = await fetch('/api/sql/compare', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        session_id: props.sessionId,
        sql_left: sqlLeft.value,
        sql_right: sqlRight.value,
        key_columns: keys,
      }),
    })

    if (!resp.ok) {
      const body = await resp.json().catch(() => ({}))
      throw new Error(body?.error?.message || `HTTP ${resp.status}`)
    }

    result.value = await resp.json()
  } catch (e: unknown) {
    errorMsg.value = e instanceof Error ? e.message : String(e)
  } finally {
    isComparing.value = false
  }
}

// ── Export ──────────────────────────────────────
async function copyDiffSummary() {
  if (!result.value) return
  const lines = [`Left: ${result.value.summary.left_rows} rows | Right: ${result.value.summary.right_rows} rows`]
  lines.push(`Identical: ${result.value.summary.identical_rows} | Modified: ${result.value.summary.modified_rows}`)
  lines.push(`Only in left: ${result.value.summary.only_in_left} | Only in right: ${result.value.summary.only_in_right}`)
  lines.push('')
  for (const d of result.value.diffs) {
    lines.push(`Row ${d.row_index} [${d.diff_type}] ${d.column}: ${JSON.stringify(d.left_value)} → ${JSON.stringify(d.right_value)}`)
  }
  await clipboard.writeText(lines.join('\n'))
}

// Common columns between left and right
const commonColumns = computed(() => {
  if (!result.value) return []
  const leftNames = new Set(result.value.left.columns.map(c => c.name))
  return result.value.right.columns.filter(c => leftNames.has(c.name))
})
</script>

<template>
  <div class="data-compare">
    <!-- Header -->
    <div class="dc-header">
      <span class="dc-title">⚖️ {{ t('sql.dataCompare', 'Data Compare') }}</span>
      <button class="dc-close" @click="emit('close')">✕</button>
    </div>

    <!-- Query inputs -->
    <div class="dc-queries">
      <div class="dc-query-pane">
        <div class="dc-query-label">{{ t('sql.leftQuery', 'Left Query') }}</div>
        <SqlEditor v-model="sqlLeft" />
      </div>
      <div class="dc-query-pane">
        <div class="dc-query-label">{{ t('sql.rightQuery', 'Right Query') }}</div>
        <SqlEditor v-model="sqlRight" />
      </div>
    </div>

    <!-- Controls -->
    <div class="dc-controls">
      <div class="dc-key-input">
        <label>{{ t('sql.keyColumns', 'Key Columns') }}:</label>
        <input
          v-model="keyColumns"
          class="dc-input"
          :placeholder="t('sql.keyColumnsPlaceholder', 'e.g. id (comma-separated, empty = row index)')"
        />
      </div>
      <Button
        :disabled="isComparing || !sqlLeft.trim() || !sqlRight.trim()"
        @click="onCompare"
      >
        {{ isComparing ? t('sql.comparing', 'Comparing...') : t('sql.compare', 'Compare') }}
      </Button>
    </div>

    <!-- Error -->
    <div v-if="errorMsg" class="dc-error">❌ {{ errorMsg }}</div>

    <!-- Summary -->
    <div v-if="result" class="dc-summary">
      <span class="dc-stat">{{ t('sql.leftRows', 'Left') }}: <b>{{ result.summary.left_rows }}</b></span>
      <span class="dc-stat">{{ t('sql.rightRows', 'Right') }}: <b>{{ result.summary.right_rows }}</b></span>
      <span class="dc-stat dc-stat-ok">✓ {{ result.summary.identical_rows }} identical</span>
      <span class="dc-stat dc-stat-mod">✎ {{ result.summary.modified_rows }} modified</span>
      <span class="dc-stat dc-stat-left">← {{ result.summary.only_in_left }} only-left</span>
      <span class="dc-stat dc-stat-right">→ {{ result.summary.only_in_right }} only-right</span>
      <button class="dc-copy-btn" :title="t('sql.copyDiffSummary', 'Copy diff summary')" @click="copyDiffSummary">📋</button>
    </div>

    <!-- Side-by-side results -->
    <div v-if="result" class="dc-results">
      <div class="dc-result-pane">
        <div class="dc-result-label">{{ t('sql.leftResult', 'Left') }} ({{ result.left.elapsed_ms }}ms)</div>
        <div class="dc-table-wrap">
          <table class="dc-table">
            <thead>
              <tr>
                <th class="dc-row-num">#</th>
                <th v-for="col in result.left.columns" :key="col.name">{{ col.name }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(row, ri) in result.left.rows"
                :key="ri"
                :class="{ 'dc-row-diff': hasDiffRow(ri), 'dc-row-only': rowDiffType(ri) === 'only_in_left' }"
              >
                <td class="dc-row-num">{{ ri }}</td>
                <td
                  v-for="(cell, ci) in row"
                  :key="ci"
                  :class="{ 'dc-cell-diff': hasDiffCell(ri, result.left.columns[ci]?.name ?? '') }"
                >{{ cell === null ? 'NULL' : cell }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
      <div class="dc-result-pane">
        <div class="dc-result-label">{{ t('sql.rightResult', 'Right') }} ({{ result.right.elapsed_ms }}ms)</div>
        <div class="dc-table-wrap">
          <table class="dc-table">
            <thead>
              <tr>
                <th class="dc-row-num">#</th>
                <th v-for="col in result.right.columns" :key="col.name">{{ col.name }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(row, ri) in result.right.rows"
                :key="ri"
                :class="{ 'dc-row-diff': hasDiffRow(ri), 'dc-row-only': rowDiffType(ri) === 'only_in_right' }"
              >
                <td class="dc-row-num">{{ ri }}</td>
                <td
                  v-for="(cell, ci) in row"
                  :key="ci"
                  :class="{ 'dc-cell-diff': hasDiffCell(ri, result.right.columns[ci]?.name ?? '') }"
                >{{ cell === null ? 'NULL' : cell }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- Diff detail list -->
    <div v-if="result && result.diffs.length > 0" class="dc-diff-list">
      <div class="dc-result-label">{{ t('sql.diffDetails', 'Diff Details') }} ({{ result.diffs.length }})</div>
      <table class="dc-diff-table">
        <thead>
          <tr>
            <th>#</th>
            <th>{{ t('sql.row', 'Row') }}</th>
            <th>{{ t('sql.type', 'Type') }}</th>
            <th>{{ t('sql.column', 'Column') }}</th>
            <th>{{ t('sql.leftValue', 'Left') }}</th>
            <th>{{ t('sql.rightValue', 'Right') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(d, i) in result.diffs" :key="i" :class="`dc-row-${d.diff_type}`">
            <td>{{ i + 1 }}</td>
            <td>{{ d.row_index }}</td>
            <td><span :class="`dc-badge dc-badge-${d.diff_type}`">{{ d.diff_type }}</span></td>
            <td class="mono">{{ d.column }}</td>
            <td class="mono dc-val-left">{{ JSON.stringify(d.left_value) }}</td>
            <td class="mono dc-val-right">{{ JSON.stringify(d.right_value) }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- No diffs -->
    <div v-if="result && result.diffs.length === 0" class="dc-no-diff">
      ✅ {{ t('sql.noDiffs', 'Results are identical') }}
    </div>
  </div>
</template>

<style scoped>
.data-compare {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3);
  height: 100%;
  overflow-y: auto;
}

.dc-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.dc-title { font-size: 14px; font-weight: 600; }
.dc-close { background: none; border: none; color: var(--text-secondary); cursor: pointer; font-size: 16px; padding: 4px; }

.dc-queries {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-3);
}

.dc-query-pane { display: flex; flex-direction: column; gap: var(--space-1); }

.dc-query-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.dc-controls {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.dc-key-input { display: flex; align-items: center; gap: var(--space-2); flex: 1; }
.dc-key-input label { font-size: 12px; color: var(--text-secondary); white-space: nowrap; }

.dc-input {
  flex: 1;
  padding: 4px 8px;
  font-size: 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-primary);
  font-family: monospace;
}

.dc-error {
  padding: var(--space-2);
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  border-radius: 4px;
  color: #ef4444;
  font-size: 12px;
}

.dc-summary {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  background: var(--bg-secondary);
  border-radius: 6px;
  font-size: 12px;
  flex-wrap: wrap;
}

.dc-stat { color: var(--text-secondary); white-space: nowrap; }
.dc-stat b { color: var(--text-primary); }
.dc-stat-ok { color: #22c55e; }
.dc-stat-mod { color: #eab308; }
.dc-stat-left { color: #3b82f6; }
.dc-stat-right { color: #a855f7; }
.dc-copy-btn { margin-left: auto; background: none; border: none; cursor: pointer; font-size: 14px; opacity: 0.6; }
.dc-copy-btn:hover { opacity: 1; }

.dc-results {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-3);
  flex: 1;
  min-height: 200px;
}

.dc-result-pane {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}

.dc-result-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  padding: var(--space-1) var(--space-2);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
}

.dc-table-wrap { overflow: auto; flex: 1; max-height: 400px; }

.dc-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
  font-family: monospace;
}

.dc-table th {
  text-align: left;
  padding: 4px 8px;
  background: var(--bg-tertiary, var(--bg-secondary));
  border-bottom: 1px solid var(--border);
  position: sticky;
  top: 0;
  z-index: 1;
  font-weight: 500;
  font-size: 11px;
  color: var(--text-secondary);
  white-space: nowrap;
}

.dc-table td {
  padding: 3px 8px;
  border-bottom: 1px solid var(--border);
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dc-row-num { color: var(--text-secondary); opacity: 0.5; width: 30px; text-align: right; }
.dc-row-diff { background: rgba(234, 179, 8, 0.08); }
.dc-row-only { background: rgba(59, 130, 246, 0.08); }
.dc-cell-diff { background: rgba(234, 179, 8, 0.15) !important; font-weight: 600; }

.dc-diff-list { overflow-x: auto; }

.dc-diff-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.dc-diff-table th {
  text-align: left;
  padding: 6px 8px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  font-weight: 500;
  color: var(--text-secondary);
  white-space: nowrap;
}
.dc-diff-table td { padding: 4px 8px; border-bottom: 1px solid var(--border); }

.dc-row-only_in_left { background: rgba(59, 130, 246, 0.05); }
.dc-row-only_in_right { background: rgba(168, 85, 247, 0.05); }
.dc-row-modified { background: rgba(234, 179, 8, 0.05); }

.dc-badge { display: inline-block; padding: 1px 6px; border-radius: 3px; font-size: 10px; font-weight: 600; text-transform: uppercase; }
.dc-badge-modified { background: rgba(234, 179, 8, 0.15); color: #ca8a04; }
.dc-badge-only_in_left { background: rgba(59, 130, 246, 0.15); color: #2563eb; }
.dc-badge-only_in_right { background: rgba(168, 85, 247, 0.15); color: #9333ea; }

.dc-val-left { color: #3b82f6; max-width: 300px; }
.dc-val-right { color: #a855f7; max-width: 300px; }

.dc-no-diff { text-align: center; padding: var(--space-4); color: #22c55e; font-size: 14px; }

.mono { font-family: monospace; }
</style>
