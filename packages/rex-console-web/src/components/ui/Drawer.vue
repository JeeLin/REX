<script setup lang="ts">
import { onUnmounted, watch } from 'vue'

const props = withDefaults(
  defineProps<{ modelValue: boolean; side?: 'left' | 'right' | 'bottom'; title?: string; width?: string }>(),
  { side: 'right', title: '', width: '420px' },
)
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('update:modelValue', false)
}

watch(() => props.modelValue, (v) => {
  if (v) document.addEventListener('keydown', onKeydown)
  else document.removeEventListener('keydown', onKeydown)
}, { immediate: true })

onUnmounted(() => document.removeEventListener('keydown', onKeydown))
</script>

<template>
  <Teleport to="body">
    <Transition name="overlay">
      <div v-if="modelValue" class="overlay" @click="emit('update:modelValue', false)" />
    </Transition>
    <Transition :name="`drawer-${side}`">
      <aside v-if="modelValue" class="drawer" :class="`drawer--${side}`" :style="side !== 'bottom' ? { width } : {}">
        <header v-if="title || $slots.header" class="drawer-header">
          <slot name="header"><h3 class="drawer-title">{{ title }}</h3></slot>
          <button class="drawer-close" @click="emit('update:modelValue', false)">✕</button>
        </header>
        <div class="drawer-body">
          <slot />
        </div>
      </aside>
    </Transition>
  </Teleport>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  z-index: 40;
}
.drawer {
  position: fixed;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  z-index: 50;
  display: flex;
  flex-direction: column;
}
.drawer--right {
  top: 0;
  right: 0;
  bottom: 0;
  max-width: 90vw;
  border-left: 1px solid var(--border);
}
.drawer--left {
  top: 0;
  left: 0;
  bottom: 0;
  max-width: 90vw;
}
.drawer--bottom {
  left: 0;
  right: 0;
  bottom: 0;
  max-height: 60vh;
  border-top: 1px solid var(--border);
}
.drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}
.drawer-title {
  font-size: var(--text-md);
  font-weight: 600;
}
.drawer-close {
  background: transparent;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-md);
}
.drawer-close:hover {
  color: var(--text-primary);
}
.drawer-body {
  flex: 1;
  overflow: auto;
  padding: var(--space-4);
}
.overlay-enter-active,
.overlay-leave-active {
  transition: opacity var(--transition);
}
.overlay-enter-from,
.overlay-leave-to {
  opacity: 0;
}
.drawer-right-enter-active,
.drawer-right-leave-active,
.drawer-left-enter-active,
.drawer-left-leave-active {
  transition: transform var(--transition);
}
.drawer-right-enter-from,
.drawer-right-leave-to {
  transform: translateX(100%);
}
.drawer-left-enter-from,
.drawer-left-leave-to {
  transform: translateX(-100%);
}
.drawer-bottom-enter-active,
.drawer-bottom-leave-active {
  transition: transform var(--transition);
}
.drawer-bottom-enter-from,
.drawer-bottom-leave-to {
  transform: translateY(100%);
}
</style>
