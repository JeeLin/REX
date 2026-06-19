<template>
  <span class="version-badge" :class="{ 'needs-update': needsUpdate }">
    {{ version }}
    <span v-if="needsUpdate" class="update-icon" :title="t('agent.needsUpdate')">⚠</span>
  </span>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

defineProps<{
  version: string
  needsUpdate: boolean
}>()
</script>

<style scoped>
.version-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  color: var(--text-secondary);
  background: var(--bg-deep);
  border-radius: var(--radius-sm);
}

.version-badge.needs-update {
  color: var(--warning, #ffc107);
  background: rgba(255, 193, 7, 0.1);
}

.update-icon {
  font-size: 12px;
}
</style>
