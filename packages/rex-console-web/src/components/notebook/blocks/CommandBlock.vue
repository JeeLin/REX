<template>
  <div class="command-block" :data-block-id="blockId">
    <div class="command-header">
      <span class="command-icon">⚡</span>
      <span class="command-label">{{ t('notebooks.editor.blockType.command') }}</span>
      <span v-if="resourceId" class="command-resource">{{ resourceId }}</span>
      <span v-if="protocol" class="command-protocol">{{ protocol }}</span>
    </div>
    <div class="command-placeholder">
      {{ t('notebooks.editor.placeholder.command') }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  blockId: string
  content: string
  resourceId?: string
  protocol?: string
}>()

const { t } = useI18n()
</script>

<style scoped>
.command-block {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--sp-md);
  background: var(--bg-elevated);
}

.command-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  margin-bottom: var(--sp-sm);
  font-size: var(--fs-sm);
  color: var(--text-secondary);
}

.command-icon {
  font-size: var(--fs-md);
}

.command-label {
  font-weight: 500;
  color: var(--text-primary);
}

.command-resource {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--accent);
  background: var(--bg-surface);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
}

.command-protocol {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.command-placeholder {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  font-style: italic;
}
</style>
