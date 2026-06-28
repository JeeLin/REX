<template>
  <div class="transfer-panel">
    <div class="transfer-panel-header" @click="expanded = !expanded">
      <span class="panel-title">{{ t('files.transferQueue') }}</span>
      <span v-if="activeCount > 0" class="panel-badge">{{ activeCount }}</span>
      <span class="panel-toggle">{{ expanded ? '▾' : '▴' }}</span>
    </div>
    <div v-if="expanded" class="transfer-panel-body">
      <div v-if="tasks.length === 0" class="transfer-empty">{{ t('files.noTransfers') }}</div>
      <div v-else class="transfer-list">
        <TransferItem
          v-for="task in tasks"
          :key="task.id"
          :task="task"
          @cancel="$emit('cancel', $event)"
          @remove="$emit('remove', $event)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import TransferItem from './TransferItem.vue'
import type { TransferTask } from '@/api/transfer'

const { t } = useI18n()
const props = defineProps<{ tasks: TransferTask[] }>()
defineEmits<{ cancel: [id: string]; remove: [id: string] }>()

const expanded = ref(true)

const activeCount = computed(() =>
  props.tasks.filter(t => t.status === 'pending' || t.status === 'running').length,
)
</script>

<style scoped>
.transfer-panel {
  display: flex;
  flex-direction: column;
  border-top: 1px solid var(--border);
  background: var(--bg-surface);
}

.transfer-panel-header {
  display: flex;
  align-items: center;
  padding: var(--sp-sm) var(--sp-md);
  cursor: pointer;
  user-select: none;
  gap: var(--sp-sm);
  border-bottom: 1px solid var(--border);
}

.transfer-panel-header:hover {
  background: var(--bg-hover);
}

.panel-title {
  font-size: var(--fs-sm);
  font-weight: 600;
}

.panel-badge {
  background: var(--accent);
  color: #000;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  padding: 1px 6px;
  border-radius: 10px;
  font-weight: 600;
}

.panel-toggle {
  margin-left: auto;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.transfer-panel-body {
  max-height: 240px;
  overflow-y: auto;
}

.transfer-empty {
  padding: var(--sp-lg);
  text-align: center;
  font-size: var(--fs-sm);
  color: var(--text-muted);
}

.transfer-list {
  display: flex;
  flex-direction: column;
}
</style>
