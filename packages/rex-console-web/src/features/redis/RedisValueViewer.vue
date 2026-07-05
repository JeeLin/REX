<template>
  <div class="redis-value-viewer">
    <div class="value-header">
      <span class="value-key-name">{{ keyName }}</span>
      <span class="value-type-badge" :class="valueType">{{ valueType }}</span>
      <span v-if="ttl !== null" class="value-ttl" :class="{ expired: ttl === -2 }">
        TTL: {{ formatTtl(ttl) }}
      </span>
      <div class="value-header-spacer" />
      <template v-if="editing">
        <button class="redis-btn redis-btn-sm redis-btn-save" @click="handleSave">
          {{ t('common.save') }}
        </button>
        <button class="redis-btn redis-btn-sm" @click="cancelEdit">
          {{ t('common.cancel') }}
        </button>
      </template>
      <template v-else>
        <button class="redis-btn redis-btn-sm" @click="startEdit">
          {{ t('common.edit') }}
        </button>
        <button class="redis-btn redis-btn-sm" @click="$emit('refresh')">
          {{ t('redis.value.refresh') }}
        </button>
        <button class="redis-btn redis-btn-sm redis-btn-danger" @click="$emit('deleteKey', keyName)">
          {{ t('redis.value.delete') }}
        </button>
      </template>
    </div>

    <div class="value-content">
      <!-- String type -->
      <div v-if="valueType === 'string'" class="value-string">
        <template v-if="editing">
          <textarea
            v-model="editStringValue"
            class="value-edit-textarea"
            rows="8"
          />
        </template>
        <template v-else>
          <pre v-if="isJson" class="value-json">{{ formattedJson }}</pre>
          <pre v-else class="value-text">{{ displayValue }}</pre>
        </template>
      </div>

      <!-- Hash type -->
      <div v-else-if="valueType === 'hash'" class="value-hash">
        <table class="value-table">
          <thead>
            <tr>
              <th>Field</th>
              <th>Value</th>
              <th v-if="editing" class="th-action"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, index) in editing ? editHashItems : hashItems" :key="index">
              <template v-if="editing">
                <td><input v-model="item.field" class="value-edit-input" /></td>
                <td><input v-model="item.value" class="value-edit-input" /></td>
                <td class="td-action">
                  <button class="redis-btn redis-btn-sm redis-btn-danger" @click="editHashItems.splice(index, 1)">×</button>
                </td>
              </template>
              <template v-else>
                <td class="hash-field">{{ item.field }}</td>
                <td class="hash-value">{{ item.value }}</td>
              </template>
            </tr>
          </tbody>
        </table>
        <div v-if="editing" class="value-add-row">
          <button class="redis-btn redis-btn-sm" @click="editHashItems.push({ field: '', value: '' })">
            + {{ t('redis.keys.addField') }}
          </button>
        </div>
      </div>

      <!-- List type -->
      <div v-else-if="valueType === 'list'" class="value-list">
        <div v-for="(item, index) in editing ? editListItems : listItems" :key="index" class="list-item">
          <span class="list-index">[{{ index }}]</span>
          <template v-if="editing">
            <input v-model="editListItems[index]" class="value-edit-input value-list-input" />
            <button class="redis-btn redis-btn-sm redis-btn-danger" @click="editListItems.splice(index, 1)">×</button>
          </template>
          <template v-else>
            <span class="list-value">{{ item }}</span>
          </template>
        </div>
        <div v-if="editing" class="value-add-row">
          <button class="redis-btn redis-btn-sm" @click="editListItems.push('')">
            + {{ t('redis.keys.addElement') }}
          </button>
        </div>
      </div>

      <!-- Set type -->
      <div v-else-if="valueType === 'set'" class="value-set">
        <div v-for="(item, index) in editing ? editSetItems : setItems" :key="index" class="set-item">
          <template v-if="editing">
            <input v-model="editSetItems[index]" class="value-edit-input value-list-input" />
            <button class="redis-btn redis-btn-sm redis-btn-danger" @click="editSetItems.splice(index, 1)">×</button>
          </template>
          <template v-else>
            {{ item }}
          </template>
        </div>
        <div v-if="editing" class="value-add-row">
          <button class="redis-btn redis-btn-sm" @click="editSetItems.push('')">
            + {{ t('redis.keys.addMember') }}
          </button>
        </div>
      </div>

      <!-- ZSet type -->
      <div v-else-if="valueType === 'zset'" class="value-zset">
        <table class="value-table">
          <thead>
            <tr>
              <th>Member</th>
              <th>Score</th>
              <th v-if="editing" class="th-action"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(item, index) in editing ? editZsetItems : zsetItems" :key="index">
              <template v-if="editing">
                <td><input v-model="item.member" class="value-edit-input" /></td>
                <td><input v-model="item.score" type="number" class="value-edit-input" /></td>
                <td class="td-action">
                  <button class="redis-btn redis-btn-sm redis-btn-danger" @click="editZsetItems.splice(index, 1)">×</button>
                </td>
              </template>
              <template v-else>
                <td class="zset-member">{{ item.member }}</td>
                <td class="zset-score">{{ item.score }}</td>
              </template>
            </tr>
          </tbody>
        </table>
        <div v-if="editing" class="value-add-row">
          <button class="redis-btn redis-btn-sm" @click="editZsetItems.push({ member: '', score: '0' })">
            + {{ t('redis.keys.addMember') }}
          </button>
        </div>
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
import { computed, ref } from 'vue'
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

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'deleteKey', key: string): void
  (e: 'saveString', key: string, value: string): void
  (e: 'saveHash', key: string, added: { field: string; value: string }[], removed: string[]): void
  (e: 'saveList', key: string, added: string[], removedIndices: number[]): void
  (e: 'saveSet', key: string, added: string[], removed: string[]): void
  (e: 'saveZset', key: string, added: { member: string; score: string }[], removed: string[]): void
}>()

// Edit state
const editing = ref(false)
const editStringValue = ref('')
const editHashItems = ref<{ field: string; value: string }[]>([])
const editListItems = ref<string[]>([])
const editSetItems = ref<string[]>([])
const editZsetItems = ref<{ member: string; score: string }[]>([])

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

function startEdit() {
  editing.value = true
  switch (props.valueType) {
    case 'string':
      editStringValue.value = displayValue.value
      break
    case 'hash':
      editHashItems.value = hashItems.value.map(h => ({ ...h }))
      break
    case 'list':
      editListItems.value = [...listItems.value]
      break
    case 'set':
      editSetItems.value = [...setItems.value]
      break
    case 'zset':
      editZsetItems.value = zsetItems.value.map(z => ({ member: z.member, score: String(z.score) }))
      break
  }
}

function cancelEdit() {
  editing.value = false
}

function handleSave() {
  switch (props.valueType) {
    case 'string':
      emit('saveString', props.keyName, editStringValue.value)
      break
    case 'hash': {
      // Compare original vs edited to find added and removed
      const origFields = new Set(hashItems.value.map(h => h.field))
      const newFields = new Set(editHashItems.value.map(h => h.field))
      const removed = [...origFields].filter(f => !newFields.has(f))
      const added = editHashItems.value.filter(h => !origFields.has(h.field) || hashItems.value.find(oh => oh.field === h.field)?.value !== h.value)
      emit('saveHash', props.keyName, added, removed)
      break
    }
    case 'list': {
      const removedIndices: number[] = []
      const origLen = listItems.value.length
      const newLen = editListItems.value.length
      // Items at indices >= newLen that existed in original are removed
      for (let i = newLen; i < origLen; i++) {
        removedIndices.push(i)
      }
      const added = editListItems.value.slice(origLen)
      emit('saveList', props.keyName, added, removedIndices)
      break
    }
    case 'set': {
      const origSet = new Set(setItems.value)
      const newSet = new Set(editSetItems.value)
      const added = [...newSet].filter(m => !origSet.has(m))
      const removed = [...origSet].filter(m => !newSet.has(m))
      emit('saveSet', props.keyName, added, removed)
      break
    }
    case 'zset': {
      const origMembers = new Map(zsetItems.value.map(z => [z.member, String(z.score)]))
      const added = editZsetItems.value.filter(z => !origMembers.has(z.member) || origMembers.get(z.member) !== z.score)
      const removed = [...origMembers.keys()].filter(m => !editZsetItems.value.find(z => z.member === m))
      emit('saveZset', props.keyName, added, removed)
      break
    }
  }
  editing.value = false
}

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

.th-action { width: 32px; }
.td-action { width: 32px; text-align: center; }

.list-item, .set-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
}

.list-index {
  color: var(--text-muted);
  flex-shrink: 0;
}

.value-loading, .value-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

/* Edit mode */
.value-edit-textarea {
  width: 100%;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 8px;
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  resize: vertical;
  box-sizing: border-box;
}

.value-edit-textarea:focus {
  outline: none;
  border-color: var(--accent);
}

.value-edit-input {
  width: 100%;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 3px 6px;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  box-sizing: border-box;
}

.value-edit-input:focus {
  outline: none;
  border-color: var(--accent);
}

.value-list-input {
  flex: 1;
}

.value-add-row {
  padding: 8px 0;
}

.redis-btn-save {
  border-color: #3fb950;
  color: #3fb950;
}

.redis-btn-danger {
  color: #f85149;
  border-color: #f85149;
}

.redis-btn-danger:hover {
  background: #f8514922;
}
</style>
