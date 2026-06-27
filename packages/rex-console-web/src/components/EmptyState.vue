<template>
  <div class="empty-state">
    <span v-if="icon" class="empty-icon">{{ icon }}</span>
    <p class="empty-title">{{ title }}</p>
    <p v-if="hint" class="empty-hint">{{ hint }}</p>
    <button v-if="action" class="btn btn-primary btn-sm" @click="action.handler">
      {{ action.label }}
    </button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  icon?: string
  title: string
  hint?: string
  action?: { label: string; handler: () => void }
}>()
</script>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--sp-md);
  padding: var(--sp-3xl);
  text-align: center;
  color: var(--text-secondary);
}

.empty-icon {
  font-size: 36px;
  opacity: 0.4;
  margin-bottom: var(--sp-xs);
}

.empty-title {
  font-size: var(--fs-base);
  font-weight: 500;
  color: var(--text-secondary);
  margin: 0;
}

.empty-hint {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  margin: 0;
}
</style>
