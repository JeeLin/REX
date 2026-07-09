<template>
  <div
    class="result-panel"
    :class="[`result-${execution.status}`, protocolClass]"
  >
    <!-- Header bar: status + duration + copy + collapse -->
    <div class="result-header" @click="expanded = !expanded">
      <span class="result-status-badge" :class="execution.status">
        {{ execution.status === 'success' ? '✓' : '✗' }}
      </span>
      <span class="result-status-text">{{ statusLabel }}</span>
      <span v-if="execution.duration_ms != null" class="result-duration">
        {{ t('notebooks.editor.command.duration', { ms: execution.duration_ms }) }}
      </span>
      <button
        class="result-copy-btn"
        :class="{ copied }"
        @click.stop="copyOutput"
        type="button"
        :title="t('notebooks.editor.result.copy')"
      >
        {{ copied ? '✓' : '⧉' }}
      </button>
      <span class="result-expand-icon" :class="{ open: expanded }">▾</span>
    </div>

    <!-- Collapsible result body -->
    <div v-show="expanded" class="result-body">
      <!-- SSH / Terminal: dark terminal style -->
      <div v-if="isSSH || isTerminal" class="result-terminal">
        <pre class="terminal-output">{{ displayOutput }}</pre>
      </div>

      <!-- SQL: tabular display -->
      <div v-else-if="isSQL" class="result-sql">
        <div v-if="parsedTable" class="sql-table-wrapper">
          <table class="sql-table">
            <thead>
              <tr>
                <th v-for="(col, i) in parsedTable.columns" :key="i">{{ col }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(row, ri) in parsedTable.rows" :key="ri">
                <td v-for="(cell, ci) in row" :key="ci">{{ cell }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <pre v-else class="sql-raw-output">{{ displayOutput }}</pre>
      </div>

      <!-- Redis: command + key-value display -->
      <div v-else-if="isRedis" class="result-redis">
        <pre class="redis-output">{{ displayOutput }}</pre>
      </div>

      <!-- S3: list/status display -->
      <div v-else-if="isS3" class="result-s3">
        <pre class="s3-output">{{ displayOutput }}</pre>
      </div>

      <!-- Default: plain pre -->
      <div v-else class="result-default">
        <pre class="default-output">{{ displayOutput }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { NotebookExecution } from '@/api/notebook'

const props = defineProps<{
  execution: NotebookExecution
  protocol?: string
}>()

const { t } = useI18n()

const expanded = ref(true)
const copied = ref(false)

// Protocol detection
const proto = computed(() => props.protocol?.toLowerCase() ?? '')
const isSSH = computed(() => proto.value === 'ssh')
const isTerminal = computed(() => proto.value === 'terminal')
const isSQL = computed(() => proto.value === 'sql')
const isRedis = computed(() => proto.value === 'redis')
const isS3 = computed(() => proto.value === 's3')

const protocolClass = computed(() => {
  if (!proto.value) return ''
  return `protocol-${proto.value}`
})

const statusLabel = computed(() => {
  switch (props.execution.status) {
    case 'success': return t('notebooks.editor.command.completed')
    case 'error': return t('notebooks.editor.command.failed')
    default: return props.execution.status
  }
})

const displayOutput = computed(() => {
  return props.execution.output || t('notebooks.editor.command.noOutput')
})

/** Attempt to parse output as a simple delimited table (pipe or tab separated). */
const parsedTable = computed(() => {
  const text = props.execution.output
  if (!text) return null

  const lines = text.trim().split('\n').filter(Boolean)
  if (lines.length < 2) return null

  // Detect delimiter: prefer pipe, fall back to tab
  const firstLine = lines[0]!
  const delimiter = firstLine.includes('|') ? '|' : '\t'

  const columns = firstLine.split(delimiter).map(c => c.trim())
  if (columns.length < 2) return null

  const rows: string[][] = []
  for (let i = 1; i < lines.length; i++) {
    const cells = lines[i]!.split(delimiter).map(c => c.trim())
    rows.push(cells)
  }

  return { columns, rows }
})

async function copyOutput() {
  try {
    await navigator.clipboard.writeText(props.execution.output)
    copied.value = true
    setTimeout(() => { copied.value = false }, 2000)
  } catch {
    // Fallback: no-op if clipboard API unavailable
  }
}
</script>

<style scoped>
.result-panel {
  border-top: 1px solid var(--border);
  background: var(--bg-surface);
  border-radius: 0 0 var(--radius-md) var(--radius-md);
}

/* ── Header ────────────────────────────── */
.result-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-xs) var(--sp-md);
  cursor: pointer;
  user-select: none;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  border-bottom: 1px solid var(--border);
  transition: background var(--transition-fast);
}

.result-header:hover {
  background: var(--bg-elevated);
}

.result-status-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  font-size: 11px;
  font-weight: 600;
  flex-shrink: 0;
}

.result-status-badge.success {
  background: var(--success, #10b981);
  color: white;
}

.result-status-badge.error,
.result-status-badge.failed {
  background: var(--danger, #ef4444);
  color: white;
}

.result-status-text {
  color: var(--text-primary);
  font-weight: 500;
}

.result-duration {
  margin-left: auto;
  font-family: var(--font-mono);
  color: var(--text-muted);
}

.result-copy-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--fs-sm);
  transition: all var(--transition-fast);
}

.result-copy-btn:hover {
  color: var(--text-primary);
  border-color: var(--accent);
}

.result-copy-btn.copied {
  color: var(--success, #10b981);
  border-color: var(--success, #10b981);
}

.result-expand-icon {
  font-size: 10px;
  transition: transform var(--transition-fast);
}

.result-expand-icon.open {
  transform: rotate(0deg);
}

/* ── Body ──────────────────────────────── */
.result-body {
  overflow: hidden;
}

/* SSH / Terminal – dark terminal style */
.result-terminal {
  background: #0d0d1a;
  padding: var(--sp-sm) var(--sp-md);
}

.terminal-output {
  margin: 0;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  line-height: var(--lh-relaxed);
  color: #00ff88;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 400px;
  overflow-y: auto;
}

/* SQL – tabular */
.result-sql {
  padding: var(--sp-sm) var(--sp-md);
  overflow-x: auto;
}

.sql-table-wrapper {
  overflow-x: auto;
}

.sql-table {
  width: 100%;
  border-collapse: collapse;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
}

.sql-table th,
.sql-table td {
  padding: 4px 10px;
  text-align: left;
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
}

.sql-table th {
  background: var(--bg-elevated);
  color: var(--text-secondary);
  font-weight: 600;
  position: sticky;
  top: 0;
}

.sql-table td {
  color: var(--text-primary);
}

.sql-table tbody tr:hover {
  background: var(--bg-elevated);
}

.sql-raw-output {
  margin: 0;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  line-height: var(--lh-relaxed);
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 400px;
  overflow-y: auto;
}

/* Redis */
.result-redis {
  background: #1a0a0a;
  padding: var(--sp-sm) var(--sp-md);
}

.redis-output {
  margin: 0;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  line-height: var(--lh-relaxed);
  color: #ff6b6b;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 400px;
  overflow-y: auto;
}

/* S3 */
.result-s3 {
  padding: var(--sp-sm) var(--sp-md);
}

.s3-output {
  margin: 0;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  line-height: var(--lh-relaxed);
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 400px;
  overflow-y: auto;
}

/* Default fallback */
.result-default {
  padding: var(--sp-sm) var(--sp-md);
}

.default-output {
  margin: 0;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  line-height: var(--lh-relaxed);
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 400px;
  overflow-y: auto;
}
</style>
