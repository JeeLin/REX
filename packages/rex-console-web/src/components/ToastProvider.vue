<template>
  <div class="toast-container" role="status" aria-live="polite">
    <TransitionGroup name="toast">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        class="toast-item"
        :class="toast.type"
        role="alert"
        @click="remove(toast.id)"
      >
        <span class="toast-icon">{{ iconFor(toast.type) }}</span>
        <span class="toast-msg">{{ toast.message }}</span>
        <button
          class="toast-close"
          :aria-label="t('common.close')"
          @click.stop="remove(toast.id)"
        >×</button>
      </div>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useToast } from '@/composables/useToast'

const { t } = useI18n()
const { toasts, remove } = useToast()

function iconFor(type: string) {
  switch (type) {
    case 'success': return '✓'
    case 'error': return '✗'
    case 'warning': return '!'
    case 'info': return 'i'
    default: return ''
  }
}
</script>

<style scoped>
.toast-container {
  position: fixed;
  top: var(--sp-lg);
  right: var(--sp-lg);
  z-index: var(--z-toast);
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
  pointer-events: none;
}

.toast-item {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-sm) var(--sp-md);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  color: var(--text-primary);
  font-size: var(--fs-sm);
  pointer-events: auto;
  cursor: pointer;
  min-width: 240px;
  max-width: 400px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.toast-icon {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-weight: 700;
}

.toast-item.success .toast-icon { background: rgba(63, 185, 80, 0.15); color: var(--success); }
.toast-item.error .toast-icon { background: rgba(248, 81, 73, 0.15); color: var(--danger); }
.toast-item.warning .toast-icon { background: rgba(210, 153, 34, 0.15); color: var(--warning); }
.toast-item.info .toast-icon { background: rgba(88, 166, 255, 0.15); color: var(--info); }

.toast-msg {
  flex: 1;
}

.toast-close {
  flex-shrink: 0;
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 16px;
  cursor: pointer;
  padding: 0 2px;
}

.toast-close:hover { color: var(--text-primary); }

.toast-enter-active { transition: all 0.3s ease; }
.toast-leave-active { transition: all 0.2s ease; }
.toast-enter-from { opacity: 0; transform: translateX(100%); }
.toast-leave-to { opacity: 0; transform: translateX(100%); }
.toast-move { transition: transform 0.2s ease; }

@media (max-width: 767px) {
  .toast-container {
    top: auto;
    bottom: 60px; /* above bottom nav bar */
    right: var(--sp-md);
    left: var(--sp-md);
  }
  .toast-item {
    min-width: auto;
    max-width: none;
    width: 100%;
  }
}
</style>
