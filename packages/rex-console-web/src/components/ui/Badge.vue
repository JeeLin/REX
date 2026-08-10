<script setup lang="ts">
withDefaults(
  defineProps<{
    tone?: 'neutral' | 'accent' | 'success' | 'danger' | 'info' | 'purple' | 'warning'
    size?: 'sm' | 'md'
    dot?: boolean
  }>(),
  { tone: 'neutral', size: 'md', dot: false },
)
</script>

<template>
  <span class="badge" :class="[`badge--${tone}`, `badge--${size}`, { 'badge--dot': dot }]">
    <span v-if="dot" class="badge-dot" />
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
.badge--sm {
  padding: 1px 6px;
  font-size: var(--text-xs);
}
.badge--md {
  padding: 2px 8px;
  font-size: var(--text-sm);
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
  background: var(--success-soft);
  color: var(--success);
}
.badge--success.badge--dot .badge-dot {
  animation: pulse-green 2s ease-in-out infinite;
}
.badge--danger {
  background: var(--danger-soft);
  color: var(--danger);
}
.badge--info {
  background: var(--info-soft);
  color: var(--info);
}
.badge--purple {
  background: var(--purple-soft);
  color: var(--purple);
}
.badge--warning {
  background: var(--warning-soft);
  color: var(--warning);
}
</style>
