<script setup lang="ts">
import { watch, onBeforeUnmount } from 'vue'

const props = withDefaults(defineProps<{ modelValue: boolean; title?: string; width?: string }>(), {
  title: '',
  width: '480px',
})
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.modelValue) {
    emit('update:modelValue', false)
  }
}

watch(() => props.modelValue, (open) => {
  if (open) {
    document.addEventListener('keydown', onKeydown)
    document.body.style.overflow = 'hidden'
  } else {
    document.removeEventListener('keydown', onKeydown)
    document.body.style.overflow = ''
  }
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown)
  document.body.style.overflow = ''
})
</script>

<template>
  <Teleport to="body">
    <Transition name="overlay">
      <div v-if="modelValue" class="overlay" @click="emit('update:modelValue', false)" />
    </Transition>
    <Transition name="modal">
      <div v-if="modelValue" class="modal" :style="{ width }" role="dialog" aria-modal="true" :aria-labelledby="title ? 'modal-title' : undefined">
        <header v-if="title || $slots.header" class="modal-header">
          <slot name="header"><h3 id="modal-title" class="modal-title">{{ title }}</h3></slot>
          <button class="modal-close" :aria-label="$t('common.close')" @click="emit('update:modelValue', false)">✕</button>
        </header>
        <div class="modal-body">
          <slot />
        </div>
        <footer v-if="$slots.footer" class="modal-footer">
          <slot name="footer" />
        </footer>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(2px);
  z-index: 60;
}
.modal {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  max-width: 92vw;
  max-height: 86vh;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  z-index: 70;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}
.modal-title {
  font-size: var(--text-md);
  font-weight: 600;
}
.modal-close {
  background: transparent;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-md);
}
.modal-close:hover {
  color: var(--text-primary);
}
.modal-body {
  padding: var(--space-4);
  overflow: auto;
}
.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-top: 1px solid var(--border);
}
.overlay-enter-active,
.overlay-leave-active {
  transition: opacity var(--transition);
}
.overlay-enter-from,
.overlay-leave-to {
  opacity: 0;
}
.modal-enter-active,
.modal-leave-active {
  transition: opacity var(--transition), transform var(--transition);
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
  transform: translate(-50%, -50%) scale(0.96);
}
</style>
