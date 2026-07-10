<template>
  <div class="set-viewer">
    <div v-if="items.length === 0 && !editing" class="set-empty">
      {{ t('redis.value.selectKey') }}
    </div>
    <div v-else>
      <div v-for="(item, index) in editing ? editItems : items" :key="index" class="set-item">
        <template v-if="editing">
          <input v-model="editItems[index]" class="set-edit-input" />
          <button class="redis-btn redis-btn-sm redis-btn-danger" @click="editItems.splice(index, 1)">×</button>
        </template>
        <template v-else>
          <span class="set-value">{{ item }}</span>
        </template>
      </div>
      <div v-if="editing" class="set-add-row">
        <button class="redis-btn redis-btn-sm" @click="editItems.push('')">
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
  (e: 'save', added: string[], removed: string[]): void
}>()

const editing = ref(false)
const editItems = ref<string[]>([])

const items = computed(() => {
  if (!props.value || props.value.type !== 'Array') return []
  const arr = props.value.value as RedisValue[]
  return arr.map(v => v.type === 'Bulk' ? (v.value ?? '') : JSON.stringify(v))
})

watch(items, (newItems) => {
  if (!editing.value) {
    editItems.value = [...newItems]
  }
})

function startEdit() {
  editing.value = true
  editItems.value = [...items.value]
}

function cancelEdit() {
  editing.value = false
  editItems.value = [...items.value]
}

function handleSave() {
  const origSet = new Set(items.value)
  const newSet = new Set(editItems.value)
  const added = [...newSet].filter(m => !origSet.has(m))
  const removed = [...origSet].filter(m => !newSet.has(m))
  emit('save', added, removed)
  editing.value = false
}
defineExpose({ startEdit, cancelEdit, handleSave })
</script>

<style scoped>
.set-viewer {
  padding: 8px;
}

.set-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 60px;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.set-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
}

.set-value {
  color: var(--text-primary);
}

.set-edit-input {
  flex: 1;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 3px 6px;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  box-sizing: border-box;
}

.set-edit-input:focus {
  outline: none;
  border-color: var(--accent);
}

.set-add-row {
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
