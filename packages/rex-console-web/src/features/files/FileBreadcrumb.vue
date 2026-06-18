<template>
  <div class="breadcrumb">
    <span
      v-for="(part, index) in parts"
      :key="index"
      class="breadcrumb-item"
      :class="{ current: index === parts.length - 1 }"
      @click="navigate(index)"
    >
      {{ part }}
    </span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ path: string }>()
const emit = defineEmits<{ navigate: [path: string] }>()

const parts = computed(() => {
  if (props.path === '/') return ['/']
  return props.path.split('/').filter(Boolean).map((_, i, arr) => '/' + arr.slice(0, i + 1).join('/'))
})

function navigate(index: number) {
  if (index === parts.value.length - 1) return
  const path = parts.value[index]
  emit('navigate', path)
}
</script>

<style scoped>
.breadcrumb {
  display: flex;
  align-items: center;
  gap: 4px;
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  color: var(--text-secondary);
}

.breadcrumb-item {
  cursor: pointer;
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.breadcrumb-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.breadcrumb-item.current {
  color: var(--text-primary);
  font-weight: 500;
  cursor: default;
}

.breadcrumb-item.current:hover {
  background: transparent;
}
</style>
