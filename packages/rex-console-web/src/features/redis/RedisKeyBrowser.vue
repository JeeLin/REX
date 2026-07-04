<template>
  <div class="redis-key-browser">
    <div class="key-browser-header">
      <input
        v-model="searchPattern"
        class="key-search-input"
        :placeholder="t('redis.keys.searchPlaceholder')"
        @keydown.enter="handleSearch"
      />
      <button class="redis-btn redis-btn-sm" @click="handleSearch">
        {{ t('redis.keys.search') }}
      </button>
    </div>

    <div class="key-list" v-if="keys.length > 0">
      <div
        v-for="key in keys"
        :key="key"
        class="key-item"
        :class="{ selected: selectedKey === key }"
        @click="$emit('selectKey', key)"
        @contextmenu.prevent="handleKeyContext($event, key)"
      >
        <span class="key-type-icon">{{ getKeyIcon(key) }}</span>
        <span class="key-name">{{ key }}</span>
      </div>
    </div>

    <div v-else-if="!loading" class="key-list-empty">
      {{ t('redis.keys.empty') }}
    </div>

    <div v-if="loading" class="key-list-loading">
      {{ t('redis.keys.loading') }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  connected: boolean
}>()

const emit = defineEmits<{
  (e: 'selectKey', key: string): void
  (e: 'sendCommand', command: string): void
}>()

const searchPattern = ref('*')
const keys = ref<string[]>([])
const selectedKey = ref<string | null>(null)
const loading = ref(false)

function getKeyIcon(key: string): string {
  // Simple heuristic based on common Redis key patterns
  if (key.includes(':')) return '📁'
  return '🔑'
}

function handleSearch() {
  if (!props.connected) return
  loading.value = true
  keys.value = []
  // Send SCAN command via parent
  emit('sendCommand', `SCAN 0 MATCH ${searchPattern.value} COUNT 1000`)
}

function handleKeyContext(event: MouseEvent, key: string) {
  // Future: right-click menu for key operations
}

onMounted(() => {
  if (props.connected) {
    handleSearch()
  }
})
</script>

<style scoped>
.redis-key-browser {
  width: 240px;
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  background: var(--bg-surface);
}

.key-browser-header {
  padding: 8px;
  display: flex;
  gap: 4px;
  border-bottom: 1px solid var(--border);
}

.key-search-input {
  flex: 1;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 4px 8px;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
}

.key-search-input:focus {
  outline: none;
  border-color: var(--accent);
}

.key-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
}

.key-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--fs-xs);
  color: var(--text-primary);
}

.key-item:hover {
  background: var(--bg-elevated);
}

.key-item.selected {
  background: var(--accent-muted);
  color: var(--accent);
}

.key-type-icon {
  font-size: 12px;
  flex-shrink: 0;
}

.key-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
}

.key-list-empty,
.key-list-loading {
  padding: 16px;
  text-align: center;
  color: var(--text-muted);
  font-size: var(--fs-xs);
}
</style>
