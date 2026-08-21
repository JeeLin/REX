<script setup lang="ts">
import { useNotificationStore } from '@/stores/notification'

const store = useNotificationStore()

function icon(type: string) {
  switch (type) {
    case 'success': return '✓'
    case 'error': return '✕'
    case 'warning': return '⚠'
    case 'info': return 'ℹ'
    default: return '•'
  }
}
</script>

<template>
  <div class="notification-container">
    <TransitionGroup name="notification">
      <div
        v-for="n in store.notifications"
        :key="n.id"
        class="notification-toast"
        :class="`notification-toast--${n.type}`"
      >
        <span class="notification-icon">{{ icon(n.type) }}</span>
        <span class="notification-message">{{ n.message }}</span>
        <button class="notification-close" @click="store.remove(n.id)">✕</button>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.notification-container {
  position: fixed;
  top: var(--space-4);
  right: var(--space-4);
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  max-width: 400px;
}
.notification-toast {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: var(--text-sm);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}
.notification-toast--success { border-left: 3px solid var(--success); }
.notification-toast--error { border-left: 3px solid var(--danger); }
.notification-toast--warning { border-left: 3px solid var(--warning); }
.notification-toast--info { border-left: 3px solid var(--info); }
.notification-icon {
  font-size: 14px;
  font-weight: 700;
}
.notification-toast--success .notification-icon { color: var(--success); }
.notification-toast--error .notification-icon { color: var(--danger); }
.notification-toast--warning .notification-icon { color: var(--warning); }
.notification-toast--info .notification-icon { color: var(--info); }
.notification-message { flex: 1; }
.notification-close {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 12px;
  padding: 2px;
}
.notification-close:hover { color: var(--text-primary); }
.notification-enter-active { transition: all 0.3s ease; }
.notification-leave-active { transition: all 0.3s ease; }
.notification-enter-from { opacity: 0; transform: translateX(100px); }
.notification-leave-to { opacity: 0; transform: translateX(100px); }
</style>
