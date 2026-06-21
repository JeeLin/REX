<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { listHistory, clearHistory, type HistoryRecord } from '@/api/sql'

const { t } = useI18n()

const props = defineProps<{
  resourceId: string
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
  select: [record: HistoryRecord]
}>()

const records = ref<HistoryRecord[]>([])
const search = ref('')
const loading = ref(false)

const filtered = computed(() => {
  if (!search.value.trim()) return records.value
  const q = search.value.toLowerCase()
  return records.value.filter(
    (r) => r.sql.toLowerCase().includes(q) || r.database.toLowerCase().includes(q),
  )
})

function truncate(sql: string, max: number): string {
  const oneLine = sql.replace(/\s+/g, ' ').trim()
  return oneLine.length > max ? oneLine.slice(0, max) + '…' : oneLine
}

function formatTime(ts: string): string {
  const sec = parseInt(ts, 10)
  const d = new Date(sec * 1000)
  const now = new Date()
  const isToday =
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate()
  const time = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  if (isToday) return time
  return `${d.getMonth() + 1}/${d.getDate()} ${time}`
}

async function loadHistory() {
  if (!props.visible) return
  loading.value = true
  try {
    records.value = await listHistory(props.resourceId)
  } catch {
    records.value = []
  } finally {
    loading.value = false
  }
}

async function handleClear() {
  try {
    await clearHistory(props.resourceId)
    records.value = []
  } catch {
    // ignore
  }
}

watch(
  () => props.visible,
  (v) => {
    if (v) loadHistory()
  },
)
</script>

<template>
  <div class="history-panel" v-if="visible">
    <div class="history-header">
      <span class="history-title">{{ t('sql.history.title') }}</span>
      <div class="history-actions">
        <input
          v-model="search"
          class="history-search"
          :placeholder="t('sql.history.search')"
        />
        <button class="btn btn-ghost btn-xs" @click="handleClear">
          {{ t('sql.history.clear') }}
        </button>
        <button class="btn btn-ghost btn-xs" @click="emit('close')">✕</button>
      </div>
    </div>
    <div class="history-list">
      <div
        v-for="item in filtered"
        :key="item.id"
        class="history-item"
        @click="emit('select', item)"
      >
        <code class="history-sql">{{ truncate(item.sql, 80) }}</code>
        <div class="history-meta">
          <span>{{ item.database }}</span>
          <span>{{ formatTime(item.executed_at) }}</span>
          <span>{{ item.elapsed_ms }}ms</span>
          <span>{{ item.row_count }} rows</span>
        </div>
      </div>
      <div v-if="filtered.length === 0" class="history-empty">
        {{ t('sql.history.empty') }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.history-panel {
  border-top: 1px solid var(--border);
  background: var(--bg-secondary);
  display: flex;
  flex-direction: column;
  max-height: 260px;
  min-height: 120px;
}

.history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.history-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
}

.history-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.history-search {
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 2px 8px;
  font-size: 12px;
  color: var(--text-primary);
  width: 160px;
  outline: none;
}

.history-search:focus {
  border-color: var(--accent);
}

.history-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.history-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 12px;
  cursor: pointer;
  transition: background 0.1s;
}

.history-item:hover {
  background: var(--bg-hover);
}

.history-sql {
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 12px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.history-meta {
  display: flex;
  gap: 12px;
  font-size: 11px;
  color: var(--text-tertiary);
}

.history-empty {
  text-align: center;
  padding: 24px 12px;
  color: var(--text-tertiary);
  font-size: 13px;
}
</style>
