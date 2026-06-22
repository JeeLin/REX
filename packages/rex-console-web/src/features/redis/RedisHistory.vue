<template>
  <div class="redis-history">
    <div class="redis-history-header">
      <span>{{ t('redis.history') }}</span>
      <button class="redis-history-clear" @click="$emit('clear')">{{ t('common.delete') }}</button>
    </div>
    <div class="redis-history-list">
      <div
        v-for="entry in history"
        :key="entry.id"
        class="redis-history-item"
        @click="$emit('select', entry.command)"
      >
        <span class="redis-history-command">{{ entry.command }}</span>
        <span class="redis-history-time">{{ formatTime(entry.timestamp) }}</span>
      </div>
      <div v-if="history.length === 0" class="redis-history-empty">
        {{ t('common.noData') }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { RedisHistoryEntry } from '@/api/redis'

const { t } = useI18n()

defineProps<{
  history: RedisHistoryEntry[]
}>()

defineEmits<{
  select: [command: string]
  clear: []
}>()

function formatTime(ts: number): string {
  const d = new Date(ts)
  return d.toLocaleTimeString()
}
</script>

<style scoped>
.redis-history {
  display: flex;
  flex-direction: column;
  height: 100%;
  font-size: 13px;
}
.redis-history-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-primary);
  font-weight: 500;
}
.redis-history-clear {
  background: none;
  border: 1px solid var(--border-primary);
  color: var(--text-secondary);
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}
.redis-history-clear:hover { background: var(--bg-hover); }
.redis-history-list {
  flex: 1;
  overflow-y: auto;
}
.redis-history-item {
  display: flex;
  justify-content: space-between;
  padding: 6px 12px;
  cursor: pointer;
  border-bottom: 1px solid var(--border-secondary);
}
.redis-history-item:hover { background: var(--bg-hover); }
.redis-history-command {
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}
.redis-history-time {
  color: var(--text-secondary);
  font-size: 11px;
  margin-left: 8px;
  flex-shrink: 0;
}
.redis-history-empty {
  padding: 20px;
  text-align: center;
  color: var(--text-secondary);
}
</style>
