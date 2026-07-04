<template>
  <div class="redis-value-viewer">
    <div class="value-header">
      <span class="value-key-name">{{ keyName }}</span>
      <span class="value-type-badge" :class="valueType">{{ valueType }}</span>
      <span v-if="ttl !== null" class="value-ttl" :class="{ expired: ttl === -2 }">
        TTL: {{ formatTtl(ttl) }}
      </span>
      <div class="value-header-spacer" />
      <button class="redis-btn redis-btn-sm" @click="$emit('refresh')">
        {{ t('redis.value.refresh') }}
      </button>
      <button class="redis-btn redis-btn-sm redis-btn-danger" @click="$emit('deleteKey', keyName)">
        {{ t('redis.value.delete') }}
      </button>
    </div>

    <div class="value-content">
      <!-- String type -->
      <div v-if="valueType === 'string'" class="value-string">
        <pre v-if="isJson" class="value-json">{{ formattedJson }}</pre>
        <pre v-else class="value-text">{{ displayValue }}</pre>
      </div>

      <!-- Hash type -->
      <div v-else-if="valueType === 'hash'" class="value-hash">
        <table class="value-table">
          <thead>
            <tr>
              <th>Field</th>
              <th>Value</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, index) in hashItems" :key="index">
              <td class="hash-field">{{ item.field }}</td>
              <td class="hash-value">{{ item.value }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- List type -->
      <div v-else-if="valueType === 'list'" class="value-list">
        <div v-for="(item, index) in listItems" :key="index" class="list-item">
          <span class="list-index">[{{ index }}]</span>
          <span class="list-value">{{ item }}</span>
        </div>
      </div>

      <!-- Set type -->
      <div v-else-if="valueType === 'set'" class="value-set">
        <div v-for="(item, index) in setItems" :key="index" class="set-item">
          {{ item }}
        </div>
      </div>

      <!-- ZSet type -->
      <div v-else-if="valueType === 'zset'" class="value-zset">
        <table class="value-table">
          <thead>
            <tr>
              <th>Member</th>
              <th>Score</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, index) in zsetItems" :key="index">
              <td class="zset-member">{{ item.member }}</td>
              <td class="zset-score">{{ item.score }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Loading -->
      <div v-else-if="loading" class="value-loading">
        {{ t('redis.value.loading') }}
      </div>

      <!-- No value -->
      <div v-else class="value-empty">
        {{ t('redis.value.selectKey') }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { RedisValue } from '@/api/redis'

const { t } = useI18n()

const props = defineProps<{
  keyName: string
  valueType: string
  value: RedisValue | null
  ttl: number | null
  loading: boolean
}>()

defineEmits<{
  (e: 'refresh'): void
  (e: 'deleteKey', key: string): void
}>()

const displayValue = computed(() => {
  if (!props.value) return ''
  if (props.value.type === 'Bulk') return props.value.value ?? '(nil)'
  if (props.value.type === 'Status') return props.value.value
  if (props.value.type === 'Integer') return String(props.value.value)
  return JSON.stringify(props.value)
})

const isJson = computed(() => {
  if (props.valueType !== 'string') return false
  if (!props.value || props.value.type !== 'Bulk') return false
  const val = props.value.value
  if (!val) return false
  try {
    JSON.parse(val)
    return true
  } catch {
    return false
  }
})

const formattedJson = computed(() => {
  try {
    return JSON.stringify(JSON.parse(displayValue.value), null, 2)
  } catch {
    return displayValue.value
  }
})

const hashItems = computed(() => {
  if (!props.value || props.value.type !== 'Array') return []
  const items: { field: string; value: string }[] = []
  const arr = props.value.value as RedisValue[]
  for (let i = 0; i < arr.length; i += 2) {
    const f = arr[i]
    const v = arr[i + 1]
    const field = f && f.type === 'Bulk' ? (f.value ?? '') : ''
    const val = v && v.type === 'Bulk' ? (v.value ?? '') : ''
    items.push({ field, value: val })
  }
  return items
})

const listItems = computed(() => {
  if (!props.value || props.value.type !== 'Array') return []
  const arr = props.value.value as RedisValue[]
  return arr.map(v => v.type === 'Bulk' ? (v.value ?? '') : JSON.stringify(v))
})

const setItems = computed(() => {
  if (!props.value || props.value.type !== 'Array') return []
  const arr = props.value.value as RedisValue[]
  return arr.map(v => v.type === 'Bulk' ? (v.value ?? '') : JSON.stringify(v))
})

const zsetItems = computed(() => {
  if (!props.value || props.value.type !== 'Array') return []
  const arr = props.value.value as RedisValue[]
  const items: { member: string; score: number }[] = []
  for (let i = 0; i < arr.length; i += 2) {
    const m = arr[i]
    const s = arr[i + 1]
    const member = m && m.type === 'Bulk' ? (m.value ?? '') : ''
    const score = s && s.type === 'Bulk' ? parseFloat(s.value ?? '0') : 0
    items.push({ member, score })
  }
  return items
})

function formatTtl(ttl: number): string {
  if (ttl === -1) return '∞'
  if (ttl === -2) return 'expired'
  if (ttl < 60) return `${ttl}s`
  if (ttl < 3600) return `${Math.floor(ttl / 60)}m`
  if (ttl < 86400) return `${Math.floor(ttl / 3600)}h`
  return `${Math.floor(ttl / 86400)}d`
}
</script>

<style scoped>
.redis-value-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  border-left: 1px solid var(--border);
}

.value-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-surface);
}

.value-key-name {
  font-family: var(--font-mono);
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.value-type-badge {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 3px;
  font-weight: 600;
  text-transform: uppercase;
}

.value-type-badge.string { background: rgba(0, 200, 150, 0.15); color: #00c896; }
.value-type-badge.hash { background: rgba(255, 152, 0, 0.15); color: #ff9800; }
.value-type-badge.list { background: rgba(33, 150, 243, 0.15); color: #2196f3; }
.value-type-badge.set { background: rgba(156, 39, 176, 0.15); color: #9c27b0; }
.value-type-badge.zset { background: rgba(233, 30, 99, 0.15); color: #e91e63; }

.value-ttl {
  font-size: 11px;
  color: var(--text-muted);
}

.value-ttl.expired {
  color: var(--danger);
}

.value-header-spacer { flex: 1; }

.value-content {
  flex: 1;
  overflow: auto;
  padding: 12px;
}

.value-json, .value-text {
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
}

.value-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
}

.value-table th {
  text-align: left;
  padding: 4px 8px;
  border-bottom: 1px solid var(--border);
  color: var(--text-muted);
  font-weight: 600;
}

.value-table td {
  padding: 4px 8px;
  border-bottom: 1px solid var(--border);
}

.list-item, .set-item {
  padding: 4px 0;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
}

.list-index {
  color: var(--text-muted);
  margin-right: 8px;
}

.value-loading, .value-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}
</style>
