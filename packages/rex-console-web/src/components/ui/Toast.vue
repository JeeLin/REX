<script setup lang="ts">
import { ref, onUnmounted } from 'vue'

export interface ToastItem {
  id: number
  message: string
  tone: 'success' | 'error' | 'info' | 'warning'
}

const toasts = ref<ToastItem[]>([])
let seq = 0
const timers = new Set<ReturnType<typeof setTimeout>>()

function push(message: string, tone: ToastItem['tone'] = 'info', duration = 3000) {
  const id = ++seq
  toasts.value.push({ id, message, tone })
  const timer = setTimeout(() => {
    timers.delete(timer)
    dismiss(id)
  }, duration)
  timers.add(timer)
}
function dismiss(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id)
}

defineExpose({ push, dismiss })

onUnmounted(() => {
  timers.forEach(t => clearTimeout(t))
  timers.clear()
})
</script>

<template>
  <Teleport to="body">
    <div class="toast-stack" role="status" aria-live="polite">
      <TransitionGroup name="toast">
        <div v-for="t in toasts" :key="t.id" class="toast" :class="`toast--${t.tone}`">
          <span class="toast-icon" v-if="t.tone === 'success'">✓</span>
          <span class="toast-icon" v-else-if="t.tone === 'error'">✕</span>
          <span class="toast-icon" v-else-if="t.tone === 'warning'">⚠</span>
          <span class="toast-icon" v-else>ℹ</span>
          <span class="toast-msg">{{ t.message }}</span>
          <button class="toast-close" @click="dismiss(t.id)">✕</button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  top: var(--space-4);
  right: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  z-index: 100;
}
.toast {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 220px;
  max-width: 360px;
  padding: var(--space-3) var(--space-3) var(--space-3) var(--space-4);
  border-radius: var(--radius);
  border: 1px solid var(--border-strong);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: var(--text-base);
  box-shadow: var(--shadow);
}
.toast-icon {
  flex-shrink: 0;
  font-size: var(--text-sm);
}
.toast-msg {
  flex: 1;
  min-width: 0;
}
.toast-close {
  flex-shrink: 0;
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 2px;
  font-size: var(--text-xs);
  opacity: 0;
  transition: opacity var(--transition);
}
.toast:hover .toast-close {
  opacity: 1;
}
.toast-close:hover {
  color: var(--text-primary);
}
.toast--success {
  border-left: 3px solid var(--success);
}
.toast--error {
  border-left: 3px solid var(--danger);
}
.toast--warning {
  border-left: 3px solid var(--warning);
}
.toast--info {
  border-left: 3px solid var(--info);
}
.toast-enter-active,
.toast-leave-active {
  transition: all var(--transition);
}
.toast-enter-from {
  opacity: 0;
  transform: translateX(20px);
}
.toast-leave-to {
  opacity: 0;
  transform: translateX(20px);
}
</style>
