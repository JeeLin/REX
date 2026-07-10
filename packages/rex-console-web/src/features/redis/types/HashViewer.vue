<template>
  <div class="hash-viewer">
    <div v-if="items.length === 0 && !editing" class="hash-empty">
      {{ t('redis.value.selectKey') }}
    </div>
    <div v-else>
      <table class="hash-table">
        <thead>
          <tr>
            <th class="hash-col-field">Field</th>
            <th class="hash-col-value">Value</th>
            <th v-if="editing" class="hash-col-action"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(item, index) in editing ? editItems : items" :key="index">
            <template v-if="editing">
              <td><input v-model="item.field" class="hash-edit-input" /></td>
              <td><input v-model="item.value" class="hash-edit-input" /></td>
              <td class="hash-td-action">
                <button class="redis-btn redis-btn-sm redis-btn-danger" @click="editItems.splice(index, 1)">×</button>
              </td>
            </template>
            <template v-else>
              <td class="hash-field-cell">{{ item.field }}</td>
              <td class="hash-value-cell">{{ item.value }}</td>
            </template>
          </tr>
        </tbody>
      </table>
      <div v-if="editing" class="hash-add-row">
        <button class="redis-btn redis-btn-sm" @click="editItems.push({ field: '', value: '' })">
          + {{ t('redis.keys.addField') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { RedisValue } from '@/api/redis'

const { t } = useI18n()

const props = defineProps<{
  value: RedisValue | null
}>()

const emit = defineEmits<{
  (e: 'save', added: { field: string; value: string }[], removed: string[]): void
}>()

const editing = ref(false)
const editItems = ref<{ field: string; value: string }[]>([])

const items = computed(() => {
  if (!props.value || props.value.type !== 'Array') return []
  const arr = props.value.value as RedisValue[]
  const result: { field: string; value: string }[] = []
  for (let i = 0; i < arr.length; i += 2) {
    const f = arr[i]
    const v = arr[i + 1]
    const field = f && f.type === 'Bulk' ? (f.value ?? '') : ''
    const val = v && v.type === 'Bulk' ? (v.value ?? '') : ''
    result.push({ field, value: val })
  }
  return result
})

watch(items, (newItems) => {
  if (!editing.value) {
    editItems.value = newItems.map(h => ({ ...h }))
  }
})

function startEdit() {
  editing.value = true
  editItems.value = items.value.map(h => ({ ...h }))
}

function cancelEdit() {
  editing.value = false
  editItems.value = items.value.map(h => ({ ...h }))
}

function handleSave() {
  const origFields = new Set(items.value.map(h => h.field))
  const newFields = new Set(editItems.value.map(h => h.field))
  const removed = [...origFields].filter(f => !newFields.has(f))
  const added = editItems.value.filter(h =>
    !origFields.has(h.field) || items.value.find(oh => oh.field === h.field)?.value !== h.value
  )
  emit('save', added, removed)
  editing.value = false
}
defineExpose({ startEdit, cancelEdit, handleSave })
</script>

<style scoped>
.hash-viewer {
  padding: 8px;
}

.hash-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 60px;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.hash-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
}

.hash-table th {
  text-align: left;
  padding: 4px 8px;
  border-bottom: 1px solid var(--border);
  color: var(--text-muted);
  font-weight: 600;
}

.hash-table td {
  padding: 4px 8px;
  border-bottom: 1px solid var(--border);
}

.hash-col-action {
  width: 32px;
}

.hash-td-action {
  width: 32px;
  text-align: center;
}

.hash-field-cell {
  color: var(--accent);
  font-weight: 500;
}

.hash-value-cell {
  color: var(--text-primary);
}

.hash-edit-input {
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

.hash-edit-input:focus {
  outline: none;
  border-color: var(--accent);
}

.hash-add-row {
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
