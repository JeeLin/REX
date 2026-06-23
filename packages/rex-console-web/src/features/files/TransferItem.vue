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
          title="取消"
          @click.stop="$emit('cancel', task.id)"
        >
          ✕
        </button>
        <button
          v-if="task.status === 'completed' || task.status === 'failed' || task.status === 'cancelled'"
          class="btn-icon"
          title="移除"
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
    </div>
    <div v-if="expanded" class="transfer-details">
      <div class="detail-row">
        <span class="detail-label">源：</span>
        <span class="detail-value">{{ task.source.path }}</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">目标：</span>
        <span class="detail-value">{{ task.target.path }}</span>
      </div>
      <div v-if="task.status_detail" class="detail-row">
        <span class="detail-label">错误：</span>
        <span class="detail-value" style="color: var(--danger)">{{ task.status_detail }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { TransferTask } from '@/api/transfer'

const props = defineProps<{ task: TransferTask }>()
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
    case 'completed': return '完成'
    case 'running': return `${percent.value}%`
    case 'pending': return '等待中'
    case 'failed': return '失败'
    case 'cancelled': return '已取消'
    default: return props.task.status
  }
})
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
