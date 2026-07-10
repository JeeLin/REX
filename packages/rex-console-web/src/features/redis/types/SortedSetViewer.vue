<template>
  <div class="zset-viewer">
    <div v-if="items.length === 0 && !editing" class="zset-empty">
      {{ t('redis.value.selectKey') }}
    </div>
    <div v-else>
      <table class="zset-table">
        <thead>
          <tr>
            <th class="zset-col-member">Member</th>
            <th class="zset-col-score">Score</th>
            <th v-if="editing" class="zset-col-action"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(item, index) in editing ? editItems : items" :key="index">
            <template v-if="editing">
              <td><input v-model="item.member" class="zset-edit-input" /></td>
              <td><input v-model="item.score" type="number" class="zset-edit-input" /></td>
              <td class="zset-td-action">
                <button class="redis-btn redis-btn-sm redis-btn-danger" @click="editItems.splice(index, 1)">×</button>
              </td>
            </template>
            <template v-else>
              <td class="zset-member-cell">{{ item.member }}</td>
              <td class="zset-score-cell">{{ item.score }}</td>
            </template>
          </tr>
        </tbody>
      </table>
      <div v-if="editing" class="zset-add-row">
        <button class="redis-btn redis-btn-sm" @click="editItems.push({ member: '', score: '0' })">
          + {{ t('redis.keys.addMember') }}
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
  (e: 'save', added: { member: string; score: string }[], removed: string[]): void
}>()

const editing = ref(false)
const editItems = ref<{ member: string; score: string }[]>([])

const items = computed(() => {
  if (!props.value || props.value.type !== 'Array') return []
  const arr = props.value.value as RedisValue[]
  const result: { member: string; score: number }[] = []
  for (let i = 0; i < arr.length; i += 2) {
    const m = arr[i]
    const s = arr[i + 1]
    const member = m && m.type === 'Bulk' ? (m.value ?? '') : ''
    const score = s && s.type === 'Bulk' ? parseFloat(s.value ?? '0') : 0
    result.push({ member, score })
  }
  return result
})

watch(items, (newItems) => {
  if (!editing.value) {
    editItems.value = newItems.map(z => ({ member: z.member, score: String(z.score) }))
  }
})

function startEdit() {
  editing.value = true
  editItems.value = items.value.map(z => ({ member: z.member, score: String(z.score) }))
}

function cancelEdit() {
  editing.value = false
  editItems.value = items.value.map(z => ({ member: z.member, score: String(z.score) }))
}

function handleSave() {
  const origMembers = new Map(items.value.map(z => [z.member, String(z.score)]))
  const added = editItems.value.filter(z => !origMembers.has(z.member) || origMembers.get(z.member) !== z.score)
  const removed = [...origMembers.keys()].filter(m => !editItems.value.find(z => z.member === m))
  emit('save', added, removed)
  editing.value = false
}
defineExpose({ startEdit, cancelEdit, handleSave })
</script>

<style scoped>
.zset-viewer {
  padding: 8px;
}

.zset-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 60px;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.zset-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
}

.zset-table th {
  text-align: left;
  padding: 4px 8px;
  border-bottom: 1px solid var(--border);
  color: var(--text-muted);
  font-weight: 600;
}

.zset-table td {
  padding: 4px 8px;
  border-bottom: 1px solid var(--border);
}

.zset-col-action {
  width: 32px;
}

.zset-td-action {
  width: 32px;
  text-align: center;
}

.zset-member-cell {
  color: var(--text-primary);
}

.zset-score-cell {
  color: #e91e63;
  font-weight: 500;
}

.zset-edit-input {
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

.zset-edit-input:focus {
  outline: none;
  border-color: var(--accent);
}

.zset-add-row {
  padding: 8px 0;
}

.redis-btn-danger {
  color: #f85149;
  border-color: #f85149;
}

.redis-btn-danger:hover {
  background: #f8514922;
}
</style>
