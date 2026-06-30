<template>
  <div class="transfer-item" @click="expanded = !expanded">
    <div class="transfer-item-row">
      <span class="transfer-status-icon">{{ statusIcon }}</span>
      <span class="transfer-filename">{{ filename }}</span>
      <span class="transfer-status-text">{{ statusText }}</span>
      <div class="transfer-actions">
        <button
          v-if="task.status === 'pending' || task.status === 'running'"
          class="btn-icon"
          :title="t('common.close')"
          @click.stop="$emit('cancel', task.id)"
        >
          ✕
        </button>
        <button
          v-if="task.status === 'completed' || task.status === 'failed' || task.status === 'cancelled'"
          class="btn-icon"
          :title="t('files.delete')"
          @click.stop="$emit('remove', task.id)"
        >
          🗑
        </button>
      </div>
    </div>
    <div v-if="task.status === 'running' || task.status === 'pending'" class="transfer-progress">
      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: percent + '%' }"></div>
      </div>
      <span class="progress-text">{{ percent }}%</span>
      <span v-if="speed > 0" class="transfer-speed">{{ formatSpeed(speed) }}</span>
      <span v-if="eta > 0" class="transfer-eta">{{ formatEta(eta) }}</span>
    </div>
    <div v-if="expanded" class="transfer-details">
      <div class="detail-row">
        <span class="detail-label">{{ t('files.transfer.source') }}</span>
        <span class="detail-value">{{ task.source.path }}</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">{{ t('files.transfer.target') }}</span>
        <span class="detail-value">{{ task.target.path }}</span>
      </div>
      <div v-if="task.status_detail" class="detail-row">
        <span class="detail-label">{{ t('files.transfer.error') }}</span>
        <span class="detail-value" style="color: var(--danger)">{{ task.status_detail }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { TransferTask } from '@/api/transfer'

const { t } = useI18n()
const props = defineProps<{
  task: TransferTask
  speed?: number
  eta?: number
}>()
defineEmits<{ cancel: [id: string]; remove: [id: string] }>()

const expanded = ref(false)

const filename = computed(() => {
  const parts = props.task.target.path.split('/')
  return parts[parts.length - 1] || parts[parts.length - 2] || props.task.target.path
})

const percent = computed(() => {
  const { total_bytes, transferred_bytes } = props.task.progress
  if (total_bytes === 0) return 0
  return Math.round((transferred_bytes / total_bytes) * 100)
})

const statusIcon = computed(() => {
  switch (props.task.status) {
    case 'completed': return '✓'
    case 'running': return '⬆'
    case 'pending': return '⏳'
    case 'failed': return '✗'
    case 'cancelled': return '⊘'
    default: return '?'
  }
})

const statusText = computed(() => {
  switch (props.task.status) {
    case 'completed': return t('files.transfer.completed')
    case 'running': return `${percent.value}%`
    case 'pending': return t('files.transfer.pending')
    case 'failed': return t('files.transfer.failed')
    case 'cancelled': return t('files.transfer.cancelled')
    default: return props.task.status
  }
})

const speed = computed(() => props.speed ?? 0)
const eta = computed(() => props.eta ?? 0)

function formatSpeed(bytesPerSec: number): string {
  if (bytesPerSec < 1024) return `${Math.round(bytesPerSec)} B/s`
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`
}

function formatEta(seconds: number): string {
  if (seconds < 60) return t('files.transfer.etaLessThanMinute')
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return t('files.transfer.eta', { time: `${m}:${String(s).padStart(2, '0')}` })
}
</script>

<style scoped>
.transfer-item {
  padding: var(--sp-sm) var(--sp-md);
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.transfer-item:hover {
  background: var(--bg-hover);
}

.transfer-item-row {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.transfer-status-icon {
  font-size: var(--fs-sm);
  width: 16px;
  text-align: center;
  flex-shrink: 0;
}

.transfer-filename {
  flex: 1;
  font-size: var(--fs-sm);
  font-family: var(--font-mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.transfer-status-text {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
  flex-shrink: 0;
}

.transfer-actions {
  display: flex;
  gap: var(--sp-xs);
  flex-shrink: 0;
}

.btn-icon {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--fs-sm);
  padding: 2px 4px;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  transition: background var(--transition-fast);
}

.btn-icon:hover {
  background: var(--bg-hover);
}

.transfer-progress {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  margin-top: var(--sp-xs);
}

.progress-bar {
  flex: 1;
  height: 4px;
  background: var(--bg-deep);
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 2px;
  transition: width 0.3s ease;
}

.progress-text {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
  min-width: 32px;
  text-align: right;
}

.transfer-speed {
  font-size: var(--fs-xs);
  color: var(--text-secondary);
  font-family: var(--font-mono);
}

.transfer-eta {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.transfer-details {
  margin-top: var(--sp-sm);
  padding: var(--sp-sm);
  background: var(--bg-deep);
  border-radius: var(--radius-sm);
}

.detail-row {
  display: flex;
  gap: var(--sp-sm);
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  margin-bottom: 2px;
}

.detail-label {
  color: var(--text-muted);
  flex-shrink: 0;
}

.detail-value {
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
