<template>
  <div class="stream-viewer">
    <div v-if="entries.length === 0" class="stream-empty">
      {{ t('redis.value.selectKey') }}
    </div>
    <div v-else>
      <div class="stream-info">
        {{ entries.length }} 条消息
      </div>
      <div v-for="(entry, entryIndex) in entries" :key="entryIndex" class="stream-entry">
        <div class="stream-entry-header">
          <span class="stream-msg-id">{{ entry.id }}</span>
        </div>
        <table class="stream-fields-table">
          <thead>
            <tr>
              <th class="stream-col-field">Field</th>
              <th class="stream-col-value">Value</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(field, fieldIndex) in entry.fields" :key="fieldIndex">
              <td class="stream-field-cell">{{ field.field }}</td>
              <td class="stream-value-cell">{{ field.value }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { RedisValue } from '@/api/redis'

const { t } = useI18n()

interface StreamField {
  field: string
  value: string
}

interface StreamEntry {
  id: string
  fields: StreamField[]
}

const props = defineProps<{
  value: RedisValue | null
}>()

const entries = computed<StreamEntry[]>(() => {
  if (!props.value || props.value.type !== 'Array') return []
  const arr = props.value.value as RedisValue[]
  const result: StreamEntry[] = []
  for (const item of arr) {
    if (item.type !== 'Array' || item.value.length < 2) continue
    const entryArr = item.value as RedisValue[]
    const idVal = entryArr[0]
    const fieldsVal = entryArr[1]
    const id = idVal && idVal.type === 'Bulk' ? (idVal.value ?? '') : ''
    const fields: StreamField[] = []
    if (fieldsVal && fieldsVal.type === 'Array') {
      const fArr = fieldsVal.value as RedisValue[]
      for (let j = 0; j < fArr.length; j += 2) {
        const fname = fArr[j]
        const fval = fArr[j + 1]
        const field = fname && fname.type === 'Bulk' ? (fname.value ?? '') : ''
        const value = fval && fval.type === 'Bulk' ? (fval.value ?? '') : ''
        fields.push({ field, value })
      }
    }
    result.push({ id, fields })
  }
  return result
})
</script>

<style scoped>
.stream-viewer {
  padding: 8px;
}

.stream-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 60px;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.stream-info {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  margin-bottom: 8px;
}

.stream-entry {
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  margin-bottom: 8px;
  overflow: hidden;
}

.stream-entry-header {
  display: flex;
  align-items: center;
  padding: 6px 8px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
}

.stream-msg-id {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: #e91e63;
  font-weight: 600;
}

.stream-fields-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
}

.stream-fields-table th {
  text-align: left;
  padding: 4px 8px;
  border-bottom: 1px solid var(--border);
  color: var(--text-muted);
  font-weight: 600;
  background: var(--bg-deep);
}

.stream-fields-table td {
  padding: 4px 8px;
  border-bottom: 1px solid var(--border);
}

.stream-field-cell {
  color: var(--accent);
  font-weight: 500;
}

.stream-value-cell {
  color: var(--text-primary);
}
</style>
