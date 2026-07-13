<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'
import { listHistory, clearHistory, type HistoryRecord } from '@/api/sql'

const { t } = useI18n()
const { show: showMenu } = useContextMenu()

const props = defineProps<{
  resourceId: string
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
  select: [record: HistoryRecord]
  'open-sql-tab': [title: string, sql: string]
}>()

const records = ref<HistoryRecord[]>([])
const search = ref('')
const databaseFilter = ref('')
const loading = ref(false)

const uniqueDatabases = computed(() => {
  const dbs = new Set<string>()
  for (const r of records.value) {
    if (r.database) dbs.add(r.database)
  }
  return Array.from(dbs).sort()
})

const filtered = computed(() => {
  let result = records.value
  if (databaseFilter.value) {
    result = result.filter((r) => r.database === databaseFilter.value)
  }
  if (search.value.trim()) {
    const q = search.value.toLowerCase()
    result = result.filter(
      (r) => r.sql.toLowerCase().includes(q) || r.database.toLowerCase().includes(q),
    )
  }
  return result
})

interface TimeGroup {
  label: string
  items: HistoryRecord[]
}

const grouped = computed<TimeGroup[]>(() => {
  const now = new Date()
  const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate())
  const yesterdayStart = new Date(todayStart)
  yesterdayStart.setDate(yesterdayStart.getDate() - 1)

  const groups: Record<string, HistoryRecord[]> = {
    today: [],
    yesterday: [],
    earlier: [],
  }

  for (const r of filtered.value) {
    const d = new Date(parseInt(r.executed_at, 10) * 1000)
    if (d >= todayStart) {
      groups.today!.push(r)
    } else if (d >= yesterdayStart) {
      groups.yesterday!.push(r)
    } else {
      groups.earlier!.push(r)
    }
  }

  const result: TimeGroup[] = []
  if (groups.today!.length) result.push({ label: t('sql.history.group.today'), items: groups.today! })
  if (groups.yesterday!.length) result.push({ label: t('sql.history.group.yesterday'), items: groups.yesterday! })
  if (groups.earlier!.length) result.push({ label: t('sql.history.group.earlier'), items: groups.earlier! })
  return result
})

function truncate(sql: string, max: number): string {
  const oneLine = sql.replace(/\s+/g, ' ').trim()
  return oneLine.length > max ? oneLine.slice(0, max) + '…' : oneLine
}

function formatTime(ts: string): string {
  const sec = parseInt(ts, 10)
  const d = new Date(sec * 1000)
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function handleContextMenu(event: MouseEvent, item: HistoryRecord) {
  showMenu(event, [
    { label: t('sql.history.ctx.copySql'), action: () => copySql(item.sql) },
    { label: t('sql.history.ctx.openNewTab'), action: () => emit('open-sql-tab', truncate(item.sql, 40), item.sql) },
    { separator: true },
    { label: t('sql.history.ctx.delete'), action: () => deleteRecord(item.id) },
  ])
}

function copySql(sql: string) {
  navigator.clipboard.writeText(sql)
}

function deleteRecord(id: string) {
  records.value = records.value.filter((r) => r.id !== id)
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
  <div v-if="visible" class="history-panel">
    <div class="history-header">
      <span class="history-title">{{ t('sql.history.title') }}</span>
      <div class="history-actions">
        <select
          v-if="uniqueDatabases.length > 1"
          v-model="databaseFilter"
          class="history-db-filter"
        >
          <option value="">{{ t('sql.history.allDatabases') }}</option>
          <option v-for="db in uniqueDatabases" :key="db" :value="db">{{ db }}</option>
        </select>
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
      <template v-if="grouped.length > 0">
        <div v-for="group in grouped" :key="group.label" class="history-group">
          <div class="history-group-label">{{ group.label }}</div>
          <div
            v-for="item in group.items"
            :key="item.id"
            class="history-item"
            @click="emit('select', item)"
            @contextmenu.prevent="handleContextMenu($event, item)"
          >
            <code class="history-sql">{{ truncate(item.sql, 80) }}</code>
            <div class="history-meta">
              <span>{{ item.database }}</span>
              <span>{{ formatTime(item.executed_at) }}</span>
              <span>{{ item.elapsed_ms }}ms</span>
              <span>{{ item.row_count }} rows</span>
            </div>
          </div>
        </div>
      </template>
      <div v-else class="history-empty">
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

.history-db-filter {
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 2px 6px;
  font-size: 12px;
  color: var(--text-primary);
  outline: none;
  cursor: pointer;
  max-width: 120px;
}

.history-db-filter:focus {
  border-color: var(--accent);
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

.history-group-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  padding: 6px 12px 2px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
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
