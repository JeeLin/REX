<template>
  <div class="search-filter">
    <div class="filter-row">
      <div class="pattern-wrapper">
        <input
          v-model="pattern"
          class="filter-pattern"
          :placeholder="t('redis.keys.filter.patternPlaceholder')"
          @keydown.enter="applySearch"
          @focus="showHistory = true"
          @blur="hideHistoryDelayed"
        />
        <div v-if="showHistory && history.length > 0" class="history-dropdown">
          <div
            v-for="item in history"
            :key="item"
            class="history-item"
            @mousedown.prevent="selectHistory(item)"
          >
            <span class="history-value">{{ item }}</span>
            <span class="history-remove" @mousedown.stop="removeHistoryItem(item)">✕</span>
          </div>
        </div>
      </div>
      <button class="redis-btn-sm" @click="applySearch">
        {{ t('redis.keys.search') }}
      </button>

      <select v-model="type" class="filter-select" @change="applyFilter">
        <option value="">{{ t('redis.keys.filter.allTypes') }}</option>
        <option value="string">{{ t('redis.keys.filter.typeString') }}</option>
        <option value="hash">{{ t('redis.keys.filter.typeHash') }}</option>
        <option value="set">{{ t('redis.keys.filter.typeSet') }}</option>
        <option value="list">{{ t('redis.keys.filter.typeList') }}</option>
        <option value="zset">{{ t('redis.keys.filter.typeZset') }}</option>
        <option value="stream">{{ t('redis.keys.filter.typeStream') }}</option>
      </select>

      <input
        v-model.number="ttlMin"
        type="number"
        min="0"
        class="filter-ttl-input"
        :placeholder="t('redis.keys.filter.ttlMin')"
        @keydown.enter="applyFilter"
      />
      <span class="filter-ttl-sep">~</span>
      <input
        v-model.number="ttlMax"
        type="number"
        min="0"
        class="filter-ttl-input"
        :placeholder="t('redis.keys.filter.ttlMax')"
        @keydown.enter="applyFilter"
      />

      <button v-if="hasActiveFilter" class="filter-clear-all redis-btn-sm" @click="clearAll">
        {{ t('redis.keys.filter.clearAll') }}
      </button>
    </div>

    <div v-if="hasActiveFilter" class="filter-badges">
      <span v-if="type" class="filter-badge" @click="clearType">
        {{ type }} ✕
      </span>
      <span v-if="ttlMin !== null || ttlMax !== null" class="filter-badge" @click="clearTtl">
        TTL: {{ ttlMin ?? '*' }} ~ {{ ttlMax ?? '*' }} ✕
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const emit = defineEmits<{
  (e: 'search', pattern: string): void
  (e: 'filter', criteria: FilterCriteria): void
}>()

export interface FilterCriteria {
  type: string
  ttlMin: number | null
  ttlMax: number | null
}

const pattern = ref('*')
const type = ref('')
const ttlMin = ref<number | null>(null)
const ttlMax = ref<number | null>(null)

const hasActiveFilter = computed(() => {
  return type.value !== '' || ttlMin.value !== null || ttlMax.value !== null
})

// ── Search History ──
const HISTORY_KEY = 'rex-redis-search-history'
const MAX_HISTORY = 10

const history = ref<string[]>([])
const showHistory = ref(false)

function getHistory(): string[] {
  try {
    return JSON.parse(localStorage.getItem(HISTORY_KEY) || '[]')
  } catch {
    return []
  }
}

function addToHistory(val: string) {
  if (!val || val === '*') return
  const filtered = history.value.filter(h => h !== val)
  filtered.unshift(val)
  history.value = filtered.slice(0, MAX_HISTORY)
  localStorage.setItem(HISTORY_KEY, JSON.stringify(history.value))
}

function selectHistory(val: string) {
  pattern.value = val
  showHistory.value = false
  applySearch()
}

function removeHistoryItem(val: string) {
  history.value = history.value.filter(h => h !== val)
  localStorage.setItem(HISTORY_KEY, JSON.stringify(history.value))
}

function hideHistoryDelayed() {
  setTimeout(() => { showHistory.value = false }, 200)
}

function applySearch() {
  addToHistory(pattern.value)
  emit('search', pattern.value)
}

function applyFilter() {
  emit('filter', { type: type.value, ttlMin: ttlMin.value, ttlMax: ttlMax.value })
}

function clearType() {
  type.value = ''
  applyFilter()
}

function clearTtl() {
  ttlMin.value = null
  ttlMax.value = null
  applyFilter()
}

function clearAll() {
  type.value = ''
  ttlMin.value = null
  ttlMax.value = null
  applyFilter()
}

history.value = getHistory()
</script>

<style scoped>
.search-filter {
  margin-bottom: 1rem;
}
.filter-row {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  flex-wrap: wrap;
}
.filter-pattern {
  flex: 1;
  min-width: 150px;
  padding: 0.4rem 0.6rem;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
}
.pattern-wrapper {
  position: relative;
  flex: 1;
  min-width: 150px;
}
.pattern-wrapper .filter-pattern {
  width: 100%;
  flex: none;
}
.history-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  z-index: 50;
  margin-top: 2px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  max-height: 200px;
  overflow-y: auto;
}
.history-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.4rem 0.6rem;
  cursor: pointer;
  font-size: var(--fs-sm);
}
.history-item:hover {
  background: var(--bg-hover);
}
.history-value {
  font-family: var(--font-mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.history-remove {
  margin-left: 0.5rem;
  opacity: 0.5;
  cursor: pointer;
}
.history-remove:hover {
  opacity: 1;
  color: var(--danger);
}
.filter-select {
  padding: 0.4rem 0.6rem;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
}
.filter-ttl-input {
  width: 80px;
  padding: 0.4rem 0.6rem;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
}
.filter-ttl-sep {
  color: var(--text-secondary);
}
.filter-badges {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
}
.filter-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.2rem 0.5rem;
  background: var(--bg-badge);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  cursor: pointer;
}
.filter-badge:hover {
  background: var(--bg-badge-hover);
}
</style>