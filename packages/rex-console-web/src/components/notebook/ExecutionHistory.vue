<template>
  <div class="execution-history">
    <div class="history-header">
      <span class="history-title">{{ t('notebooks.editor.history.title') }}</span>
      <span v-if="!loading && executions.length" class="history-count">
        {{ executions.length }}
      </span>
      <button
        v-if="!loading && executions.length"
        class="history-refresh-btn"
        @click="fetchHistory"
        type="button"
        :title="t('notebooks.editor.history.refresh')"
      >
        ↻
      </button>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="history-loading">
      {{ t('notebooks.editor.history.loading') }}
    </div>

    <!-- Empty -->
    <div v-else-if="executions.length === 0" class="history-empty">
      {{ t('notebooks.editor.history.empty') }}
    </div>

    <!-- Execution list -->
    <div v-else class="history-list">
      <div
        v-for="(exec, index) in executions"
        :key="exec.id"
        class="history-item"
        :class="{
          'history-item--selected': selectedId === exec.id,
          'history-item--expanded': expandedIds.has(exec.id),
        }"
      >
        <!-- Timeline row -->
        <div
          class="history-row"
          @click="toggleExpand(exec.id)"
        >
          <span class="history-timeline-dot" :class="exec.status" />
          <span class="history-timeline-line" v-if="index < executions.length - 1" />
          <div class="history-info">
            <span class="history-time">{{ formatTime(exec.executed_at) }}</span>
            <span class="history-status" :class="exec.status">
              {{ exec.status === 'success' ? '✓' : '✗' }}
            </span>
            <span v-if="exec.duration_ms != null" class="history-duration">
              {{ exec.duration_ms }}ms
            </span>
          </div>
        </div>

        <!-- Expanded result -->
        <div v-show="expandedIds.has(exec.id)" class="history-result">
          <ResultPanel :execution="exec" :protocol="protocol" />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listExecutions } from '@/api/notebook'
import type { NotebookExecution } from '@/api/notebook'
import ResultPanel from './ResultPanel.vue'

const props = defineProps<{
  blockId: string
  protocol?: string
}>()

const { t } = useI18n()

const loading = ref(false)
const executions = ref<NotebookExecution[]>([])
const expandedIds = ref<Set<string>>(new Set())
const selectedId = ref<string | null>(null)

async function fetchHistory() {
  loading.value = true
  try {
    const list = await listExecutions(props.blockId)
    // Server returns newest first; display in that order
    executions.value = list
    // Expand the first (latest) execution by default
    expandedIds.value = new Set(list.length > 0 ? [list[0]!.id] : [])
  } catch {
    executions.value = []
  } finally {
    loading.value = false
  }
}

function toggleExpand(id: string) {
  selectedId.value = id
  const set = new Set(expandedIds.value)
  if (set.has(id)) {
    set.delete(id)
  } else {
    set.add(id)
  }
  expandedIds.value = set
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso)
    return d.toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    })
  } catch {
    return iso
  }
}

onMounted(() => {
  fetchHistory()
})

defineExpose({ refresh: fetchHistory })
</script>

<style scoped>
.execution-history {
  border-top: 1px solid var(--border);
  background: var(--bg-surface);
}

/* ── Header ────────────────────────────── */
.history-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-xs) var(--sp-md);
  background: var(--bg-elevated);
  border-bottom: 1px solid var(--border);
  font-size: var(--fs-xs);
  color: var(--text-secondary);
}

.history-title {
  font-weight: 500;
}

.history-count {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  background: var(--bg-surface);
  padding: 0 6px;
  border-radius: var(--radius-sm);
}

.history-refresh-btn {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--fs-sm);
  transition: all var(--transition-fast);
}

.history-refresh-btn:hover {
  color: var(--text-primary);
  border-color: var(--accent);
}

/* ── Loading / Empty ───────────────────── */
.history-loading,
.history-empty {
  padding: var(--sp-md);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  text-align: center;
}

/* ── List ──────────────────────────────── */
.history-list {
  max-height: 400px;
  overflow-y: auto;
}

.history-item {
  position: relative;
}

.history-item--selected {
  background: var(--bg-elevated);
}

/* Timeline row */
.history-row {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-xs) var(--sp-md);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.history-row:hover {
  background: var(--bg-elevated);
}

.history-timeline-dot {
  position: relative;
  z-index: 1;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.history-timeline-dot.success {
  background: var(--success, #10b981);
}

.history-timeline-dot.error,
.history-timeline-dot.failed {
  background: var(--danger, #ef4444);
}

.history-timeline-line {
  position: absolute;
  left: calc(var(--sp-md) + 4px);
  top: 24px;
  bottom: -1px;
  width: 2px;
  background: var(--border);
}

.history-info {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.history-time {
  font-family: var(--font-mono);
  color: var(--text-secondary);
}

.history-status {
  font-weight: 600;
}

.history-status.success {
  color: var(--success, #10b981);
}

.history-status.error,
.history-status.failed {
  color: var(--danger, #ef4444);
}

.history-duration {
  font-family: var(--font-mono);
  color: var(--text-muted);
}

/* Expanded result */
.history-result {
  border-top: 1px solid var(--border);
}
</style>
