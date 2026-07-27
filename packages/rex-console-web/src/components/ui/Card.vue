<script setup lang="ts">
withDefaults(defineProps<{ title?: string; padded?: boolean; hoverable?: boolean }>(), { title: '', padded: true, hoverable: false })
</script>

<template>
  <div class="card" :class="{ 'card--flush': !padded, 'card--hoverable': hoverable }">
    <div v-if="title || $slots.header" class="card-header">
      <slot name="header">
        <h3 class="card-title">{{ title }}</h3>
      </slot>
    </div>
    <div class="card-body">
      <slot />
    </div>
    <div v-if="$slots.footer" class="card-footer">
      <slot name="footer" />
    </div>
  </div>
</template>

<style scoped>
.card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
  transition: border-color var(--transition);
}
.card--hoverable:hover {
  border-color: var(--accent);
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
.card-footer {
  padding: var(--space-3) var(--space-4);
  border-top: 1px solid var(--border);
}
.card--flush .card-body {
  padding: 0;
}
</style>
