<template>
  <div class="transfer-panel">
    <div class="transfer-panel-header" @click="expanded = !expanded">
      <span class="panel-title">{{ t('files.transferQueue') }}</span>
      <span v-if="activeCount > 0" class="panel-badge">{{ activeCount }}</span>
      <span v-if="stats" class="panel-stats">
        {{ stats.active_transfers }}/{{ stats.max_concurrent }}
      </span>
      <span class="panel-toggle">{{ expanded ? '▾' : '▴' }}</span>
    </div>
    <div v-if="expanded" class="transfer-panel-body">
      <div v-if="tasks.length === 0" class="transfer-empty">{{ t('files.noTransfers') }}</div>
      <div v-else class="transfer-list">
        <TransferItem
          v-for="task in tasks"
          :key="task.id"
          :task="task"
          :speed="speeds?.get(task.id)"
          :eta="etas?.get(task.id)"
          @cancel="$emit('cancel', $event)"
          @remove="$emit('remove', $event)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import TransferItem from './TransferItem.vue'
import type { TransferTask, TransferStats } from '@/api/transfer'
import { getTransferStats } from '@/api/transfer'

const { t } = useI18n()
const props = defineProps<{
  tasks: TransferTask[]
  speeds?: Map<string, number>
  etas?: Map<string, number>
}>()
defineEmits<{ cancel: [id: string]; remove: [id: string] }>()

const expanded = ref(true)
const stats = ref<TransferStats | null>(null)

const activeCount = computed(() =>
  props.tasks.filter(t => t.status === 'pending' || t.status === 'running').length,
)

// Poll stats every 2 seconds
let statsInterval: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  const fetchStats = async () => {
    try {
      stats.value = await getTransferStats()
    } catch {
      // ignore
    }
  }
  fetchStats()
  statsInterval = setInterval(fetchStats, 2000)
})

onUnmounted(() => {
  if (statsInterval) clearInterval(statsInterval)
})
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

.panel-stats {
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  color: var(--text-muted);
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--bg-hover);
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
