<script setup lang="ts">
withDefaults(
  defineProps<{
    tone?: 'neutral' | 'accent' | 'success' | 'danger' | 'info' | 'purple' | 'warning'
  }>(),
  { tone: 'neutral' },
)
</script>

<template>
  <span class="badge" :class="`badge--${tone}`">
    <slot />
  </span>
</template>

<style scoped>
.badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: var(--radius-pill);
  font-size: var(--text-sm);
  font-weight: 500;
  line-height: 1.5;
  border: 1px solid transparent;
}
.badge--neutral {
  background: var(--bg-elevated);
  color: var(--text-secondary);
  border-color: var(--border);
}
.badge--accent {
  background: var(--accent-soft);
  color: var(--accent);
}
.badge--success {
  background: rgba(63, 185, 80, 0.14);
  color: var(--success);
}
.badge--danger {
  background: rgba(248, 81, 73, 0.14);
  color: var(--danger);
}
.badge--info {
  background: rgba(88, 166, 255, 0.14);
  color: var(--info);
}
.badge--purple {
  background: rgba(139, 92, 246, 0.14);
  color: var(--purple);
}
.badge--warning {
  background: rgba(210, 153, 34, 0.14);
  color: var(--warning);
}
</style>
