<template>
  <div class="s3-bucket-list">
    <div class="s3-bucket-header">
      <span class="s3-bucket-title">📦 {{ t('s3.buckets') }}</span>
      <button class="s3-btn s3-btn-sm" :disabled="loading" @click="$emit('refresh')">🔄</button>
    </div>
    <div v-if="loading" class="s3-loading">{{ t('s3.loading') }}</div>
    <div v-else-if="buckets.length === 0" class="s3-empty">{{ t('s3.noBuckets') }}</div>
    <div v-else class="s3-bucket-items">
      <div
        v-for="bucket in buckets"
        :key="bucket.name"
        class="s3-bucket-item"
        :class="{ selected: selected === bucket.name }"
        @click="$emit('select', bucket.name)"
      >
        <span class="s3-bucket-icon">🪣</span>
        <span class="s3-bucket-name">{{ bucket.name }}</span>
        <span v-if="bucket.creation_date" class="s3-bucket-date">{{ formatDate(bucket.creation_date) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { S3BucketInfo } from './useS3Session'

defineProps<{
  buckets: S3BucketInfo[]
  loading: boolean
  selected: string | null
}>()

defineEmits<{
  select: [bucket: string]
  refresh: []
}>()

const { t } = useI18n()

function formatDate(dateStr: string): string {
  try {
    return new Date(dateStr).toLocaleDateString()
  } catch {
    return dateStr
  }
}
</script>

<style scoped>
.s3-bucket-list {
  display: flex;
  flex-direction: column;
  border-bottom: 1px solid var(--border-primary);
}

.s3-bucket-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
}

.s3-bucket-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
}

.s3-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-primary);
  padding: 2px 6px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}

.s3-btn:hover:not(:disabled) { background: var(--bg-hover); }
.s3-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.s3-btn-sm { padding: 2px 6px; }

.s3-loading, .s3-empty {
  padding: 12px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 12px;
}

.s3-bucket-items {
  max-height: 200px;
  overflow-y: auto;
}

.s3-bucket-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  font-size: 12px;
  transition: background 0.15s;
}

.s3-bucket-item:hover { background: var(--bg-hover); }

.s3-bucket-item.selected {
  background: rgba(232, 145, 45, 0.1);
  color: var(--accent);
}

.s3-bucket-icon { font-size: 14px; }

.s3-bucket-name {
  font-family: 'JetBrains Mono', monospace;
  font-weight: 500;
}

.s3-bucket-date {
  margin-left: auto;
  color: var(--text-muted);
  font-size: 11px;
}
</style>
