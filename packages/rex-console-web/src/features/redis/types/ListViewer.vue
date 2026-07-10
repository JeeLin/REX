<template>
  <div class="list-viewer">
    <div v-if="items.length === 0 && !editing" class="list-empty">
      {{ t('redis.value.selectKey') }}
    </div>
    <div v-else>
      <div v-for="(item, index) in editing ? editItems : items" :key="index" class="list-item">
        <span class="list-index">[{{ index }}]</span>
        <template v-if="editing">
          <input v-model="editItems[index]" class="list-edit-input" />
          <button class="redis-btn redis-btn-sm redis-btn-danger" @click="editItems.splice(index, 1)">×</button>
        </template>
        <template v-else>
          <span class="list-value">{{ item }}</span>
        </template>
      </div>
      <div v-if="editing" class="list-add-row">
        <button class="redis-btn redis-btn-sm" @click="editItems.push('')">
          + {{ t('redis.keys.addElement') }} (尾部)
        </button>
        <button class="redis-btn redis-btn-sm" @click="editItems.unshift('')">
          + {{ t('redis.keys.addElement') }} (头部)
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
  (e: 'save', added: string[], removedIndices: number[]): void
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
  const origLen = items.value.length
  const newLen = editItems.value.length
  const removedIndices: number[] = []
  for (let i = newLen; i < origLen; i++) {
    removedIndices.push(i)
  }
  const added = editItems.value.slice(origLen)
  emit('save', added, removedIndices)
  editing.value = false
}
defineExpose({ startEdit, cancelEdit, handleSave })
</script>

<style scoped>
.list-viewer {
  padding: 8px;
}

.list-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 60px;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.list-item {
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

.list-value {
  color: var(--text-primary);
}

.list-edit-input {
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

.list-edit-input:focus {
  outline: none;
  border-color: var(--accent);
}

.list-add-row {
  display: flex;
  gap: 8px;
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
