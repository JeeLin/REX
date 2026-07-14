<script setup lang="ts">
import { ref } from 'vue'

withDefaults(defineProps<{
  type?: 'info' | 'success' | 'warning' | 'error'
  title?: string
  closable?: boolean
}>(), { type: 'info', closable: false })

const visible = ref(true)
</script>

<template>
  <Transition name="alert">
    <div v-if="visible" class="alert" :class="`alert--${type}`">
      <div class="alert-content">
        <div v-if="title" class="alert-title">{{ title }}</div>
        <div class="alert-body"><slot /></div>
      </div>
      <button v-if="closable" class="alert-close" @click="visible = false">×</button>
    </div>
  </Transition>
</template>

<style scoped>
.alert {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius);
  border: 1px solid;
  font-size: var(--text-base);
}
.alert--info { background: rgba(88, 166, 255, 0.08); border-color: rgba(88, 166, 255, 0.3); color: var(--info); }
.alert--success { background: rgba(63, 185, 80, 0.08); border-color: rgba(63, 185, 80, 0.3); color: var(--success); }
.alert--warning { background: rgba(210, 153, 34, 0.08); border-color: rgba(210, 153, 34, 0.3); color: var(--warning); }
.alert--error { background: rgba(248, 81, 73, 0.08); border-color: rgba(248, 81, 73, 0.3); color: var(--danger); }
.alert-content { flex: 1; }
.alert-title { font-weight: 600; margin-bottom: var(--space-1); }
.alert-close {
  background: none;
  border: none;
  color: inherit;
  font-size: var(--text-lg);
  cursor: pointer;
  padding: 0;
  line-height: 1;
  opacity: 0.7;
}
.alert-close:hover { opacity: 1; }
.alert-enter-active, .alert-leave-active { transition: opacity var(--transition), transform var(--transition); }
.alert-enter-from, .alert-leave-to { opacity: 0; transform: var(--slide-up); }
</style>
