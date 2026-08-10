<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'primary' | 'secondary' | 'danger' | 'ghost'
    size?: 'sm' | 'md' | 'lg'
    disabled?: boolean
    loading?: boolean
    block?: boolean
    icon?: boolean
    ariaLabel?: string
  }>(),
  { variant: 'secondary', size: 'md', disabled: false, loading: false, block: false, icon: false, ariaLabel: '' },
)

function handleClick(e: MouseEvent) {
  const btn = e.currentTarget as HTMLElement
  const rect = btn.getBoundingClientRect()
  const x = e.clientX - rect.left
  const y = e.clientY - rect.top
  btn.style.setProperty('--ripple-x', `${x}px`)
  btn.style.setProperty('--ripple-y', `${y}px`)
  btn.classList.add('btn--ripple')
  setTimeout(() => btn.classList.remove('btn--ripple'), 400)
}
</script>

<template>
  <button
    class="btn"
    :class="[`btn--${variant}`, `btn--${size}`, { 'btn--block': block, 'btn--loading': loading, 'btn--icon': icon }]"
    :disabled="disabled || loading"
    :aria-label="ariaLabel || undefined"
    :aria-busy="loading || undefined"
    @click="handleClick"
  >
    <span v-if="loading" class="btn-spinner" />
    <slot />
    <span class="btn-ripple" />
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
  position: relative;
  overflow: hidden;
  transition: background var(--transition), border-color var(--transition), box-shadow var(--transition), transform var(--duration-fast) ease;
}
.btn:active:not(:disabled) {
  transform: scale(0.97);
}
.btn:focus-visible {
  outline: none;
  box-shadow: var(--ring);
}
.btn:disabled {
  opacity: var(--disabled-opacity);
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
.btn--block {
  width: 100%;
}
.btn--loading {
  position: relative;
  color: transparent;
}
.btn-spinner {
  position: absolute;
  width: 14px;
  height: 14px;
  border: 2px solid var(--text-secondary);
  border-right-color: transparent;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}
.btn--primary .btn-spinner {
  border-color: var(--text-on-accent);
  border-right-color: transparent;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
.btn--icon {
  padding: 0;
  width: 36px;
  height: 36px;
}
.btn--icon.btn--sm { width: 28px; height: 28px; }
.btn--icon.btn--lg { width: 42px; height: 42px; }
.btn-ripple {
  position: absolute;
  inset: 0;
  pointer-events: none;
  border-radius: inherit;
}
.btn--ripple .btn-ripple {
  background: radial-gradient(circle at var(--ripple-x, 50%) var(--ripple-y, 50%), rgba(255,255,255,0.2) 0%, transparent 60%);
  animation: ripple-expand 0.4s ease-out;
}
@keyframes ripple-expand {
  0% { opacity: 1; transform: scale(0); }
  100% { opacity: 0; transform: scale(2.5); }
}
</style>
