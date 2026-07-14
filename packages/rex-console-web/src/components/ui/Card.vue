<script setup lang="ts">
withDefaults(defineProps<{ title?: string; padded?: boolean }>(), { padded: true })
</script>

<template>
  <div class="card" :class="{ 'card--flush': !padded }">
    <div v-if="title || $slots.header" class="card-header">
      <slot name="header">
        <h3 class="card-title">{{ title }}</h3>
      </slot>
    </div>
    <div class="card-body">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
}
.card-header {
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}
.card-title {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
}
.card-body {
  padding: var(--space-4);
}
.card--flush .card-body {
  padding: 0;
}
</style>
