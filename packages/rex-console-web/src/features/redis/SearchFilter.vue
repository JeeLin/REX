<template>
  <div class="search-filter">
    <div class="filter-row">
      <input
        v-model="pattern"
        class="filter-pattern"
        :placeholder="t('redis.keys.filter.patternPlaceholder')"
        @keydown.enter="applySearch"
      />
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

function applySearch() {
  emit('search', pattern.value)
}

function applyFilter() {
  emit('filter', {
    type: type.value,
    ttlMin: ttlMin.value,
    ttlMax: ttlMax.value,
  })
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