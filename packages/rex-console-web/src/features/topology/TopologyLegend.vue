<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const items = [
  { color: '#58A6FF', label: 'Environment' },
  { color: '#79C0FF', label: 'Agent' },
  { color: '#3FB950', label: 'SSH' },
  { color: '#8B5CF6', label: 'SFTP' },
  { color: '#58A6FF', label: 'SQL' },
  { color: '#F85149', label: 'Redis' },
  { color: '#79C0FF', label: 'S3' },
]
</script>

<template>
  <div class="topo-legend">
    <div v-for="item in items" :key="item.label" class="topo-legend-item">
      <span class="topo-legend-dot" :style="{ background: item.color }" />
      <span class="topo-legend-label">{{ item.label }}</span>
    </div>
  </div>
</template>

<style scoped>
.topo-legend {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 12px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
}

.topo-legend-item {
  display: flex;
  align-items: center;
  gap: 5px;
}

.topo-legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.topo-legend-label {
  font-size: 11px;
  font-family: var(--font-mono);
  color: var(--text-muted);
}
</style>
