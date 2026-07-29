<script setup lang="ts">
import { ref, computed, watch } from 'vue'

const props = defineProps<{ modelValue: boolean; x: number; y: number }>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  select: [action: string]
}>()

const menuRef = ref<HTMLElement | null>(null)
const dims = ref({ w: 0, h: 0 })

const style = computed(() => {
  const margin = 8
  const vw = window.innerWidth
  const vh = window.innerHeight
  let left = props.x
  let top = props.y
  if (left + dims.value.w + margin > vw) left = vw - dims.value.w - margin
  if (top + dims.value.h + margin > vh) top = vh - dims.value.h - margin
  return { left: `${Math.max(margin, left)}px`, top: `${Math.max(margin, top)}px` }
})

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      requestAnimationFrame(() => {
        if (menuRef.value) {
          dims.value = { w: menuRef.value.offsetWidth, h: menuRef.value.offsetHeight }
        }
      })
    }
  },
)

function choose(action: string) {
  emit('select', action)
  emit('update:modelValue', false)
}
</script>

<template>
  <Teleport to="body">
    <Transition name="menu">
      <div
        v-if="modelValue"
        ref="menuRef"
        class="ctx-menu"
        :style="style"
        @contextmenu.prevent
      >
        <slot :choose="choose" />
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.ctx-menu {
  position: fixed;
  min-width: 180px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  padding: var(--space-1);
  z-index: 80;
}
.ctx-menu :deep(.ctx-item),
.ctx-menu :deep(.tab-ctx-item) {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.ctx-menu :deep(.ctx-item) :first-child,
.ctx-menu :deep(.tab-ctx-item) :first-child {
  font-size: 14px;
  width: 18px;
  text-align: center;
  flex-shrink: 0;
}
.menu-enter-active,
.menu-leave-active {
  transition: opacity var(--transition);
}
.menu-enter-from,
.menu-leave-to {
  opacity: 0;
}
</style>
