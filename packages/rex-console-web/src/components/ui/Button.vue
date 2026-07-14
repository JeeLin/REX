<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'primary' | 'secondary' | 'danger' | 'ghost'
    size?: 'sm' | 'md' | 'lg'
    disabled?: boolean
  }>(),
  { variant: 'secondary', size: 'md', disabled: false },
)
</script>

<template>
  <button class="btn" :class="[`btn--${variant}`, `btn--${size}`]" :disabled="disabled">
    <slot />
  </button>
</template>

<style scoped>
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  border: 1px solid transparent;
  border-radius: var(--radius);
  font-family: var(--font-sans);
  font-weight: 500;
  white-space: nowrap;
  cursor: pointer;
  transition: background var(--transition), border-color var(--transition);
}
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.btn--sm {
  height: 28px;
  padding: 0 var(--space-3);
  font-size: var(--text-sm);
}
.btn--md {
  height: 36px;
  padding: 0 var(--space-4);
  font-size: var(--text-base);
}
.btn--lg {
  height: 42px;
  padding: 0 var(--space-5);
  font-size: var(--text-md);
}
.btn--primary {
  background: var(--accent);
  color: var(--text-on-accent);
}
.btn--primary:hover:not(:disabled) {
  background: var(--accent-hover);
}
.btn--secondary {
  background: var(--bg-elevated);
  border-color: var(--border-strong);
  color: var(--text-primary);
}
.btn--secondary:hover:not(:disabled) {
  background: var(--bg-hover);
  border-color: var(--text-muted);
}
.btn--danger {
  background: var(--danger);
  color: #fff;
}
.btn--danger:hover:not(:disabled) {
  filter: brightness(1.1);
}
.btn--ghost {
  background: transparent;
  color: var(--text-secondary);
}
.btn--ghost:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
}
</style>
