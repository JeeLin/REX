<template>
  <div class="redis-result">
    <span v-if="value.type === 'Status'" class="redis-status">{{ value.value }}</span>
    <span v-else-if="value.type === 'Error'" class="redis-error">{{ value.value }}</span>
    <span v-else-if="value.type === 'Integer'" class="redis-integer">{{ value.value }}</span>
    <span v-else-if="value.type === 'Bulk'" class="redis-bulk">{{ value.value !== null ? value.value : '(nil)' }}</span>
    <span v-else-if="value.type === 'Null'" class="redis-null">(nil)</span>
    <div v-else-if="value.type === 'Array'" class="redis-array">
      <template v-if="value.value.length === 0">
        <span class="redis-null">(empty array)</span>
      </template>
      <template v-else>
        <div v-for="(item, i) in value.value" :key="i" class="redis-array-item">
          <span class="redis-array-index">{{ i + 1 }})</span>
          <RedisResult :value="item" />
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { RedisValue } from '@/api/redis'

defineProps<{
  value: RedisValue
}>()
</script>

<style scoped>
.redis-result {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 13px;
  line-height: 1.5;
}
.redis-status { color: #3fb950; }
.redis-error { color: #f85149; }
.redis-integer { color: #d29922; }
.redis-bulk { color: #e6edf3; }
.redis-null { color: #7d8590; font-style: italic; }
.redis-array { padding-left: 1em; }
.redis-array-item { display: flex; gap: 0.5em; }
.redis-array-index { color: #7d8590; min-width: 2em; text-align: right; }
</style>
