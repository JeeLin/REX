<script setup lang="ts">
import { ref } from 'vue'

export interface ToastItem {
  id: number
  message: string
  tone: 'success' | 'error' | 'info'
}

const toasts = ref<ToastItem[]>([])
let seq = 0

function push(message: string, tone: ToastItem['tone'] = 'info', duration = 3000) {
  const id = ++seq
  toasts.value.push({ id, message, tone })
  setTimeout(() => dismiss(id), duration)
}
function dismiss(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id)
}

defineExpose({ push, dismiss })
</script>

<template>
  <Teleport to="body">
    <div class="toast-stack">
      <TransitionGroup name="toast">
        <div v-for="t in toasts" :key="t.id" class="toast" :class="`toast--${t.tone}`">
          {{ t.message }}
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
  min-width: 220px;
  max-width: 360px;
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius);
  border: 1px solid var(--border-strong);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: var(--text-base);
  box-shadow: var(--shadow);
}
.toast--success {
  border-left: 3px solid var(--success);
}
.toast--error {
  border-left: 3px solid var(--danger);
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
