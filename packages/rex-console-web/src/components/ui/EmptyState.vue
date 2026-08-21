<script setup lang="ts">
defineProps<{
  icon?: string
  title: string
  description?: string
}>()
</script>

<template>
  <div class="empty-state">
    <div v-if="icon" class="empty-icon">{{ icon }}</div>
    <div class="empty-title">{{ title }}</div>
    <div v-if="description" class="empty-desc muted">{{ description }}</div>
    <div class="empty-action">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-8);
  text-align: center;
}
.empty-icon {
  font-size: 32px;
  margin-bottom: var(--space-4);
  opacity: 0.5;
}
.empty-title {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: var(--space-2);
}
.empty-desc {
  font-size: var(--text-sm);
  max-width: 320px;
}
.empty-action {
  margin-top: var(--space-4);
}
</style>
