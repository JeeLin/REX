<script setup lang="ts">
interface Option {
  label: string
  value: string | number
  icon?: string
}

withDefaults(defineProps<{
  modelValue?: string | number
  options: Option[]
  size?: 'sm' | 'md'
}>(), { size: 'md' })

const emit = defineEmits<{ 'update:modelValue': [value: string | number] }>()
</script>

<template>
  <div class="toggle-group" :class="`toggle-group--${size}`">
    <button
      v-for="option in options"
      :key="option.value"
      class="toggle-item"
      :class="{ 'toggle-item--active': option.value === modelValue }"
      @click="emit('update:modelValue', option.value)"
    >
      <span v-if="option.icon" class="toggle-icon">{{ option.icon }}</span>
      {{ option.label }}
    </button>
  </div>
</template>

<style scoped>
.toggle-group {
  display: inline-flex;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
}
.toggle-item {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background var(--transition), color var(--transition);
  white-space: nowrap;
}
.toggle-group--sm .toggle-item { padding: var(--space-1) var(--space-2); font-size: var(--text-sm); }
.toggle-group--md .toggle-item { padding: var(--space-2) var(--space-3); font-size: var(--text-base); }
.toggle-item:hover { background: var(--bg-hover); color: var(--text-primary); }
.toggle-item--active {
  background: var(--accent-soft);
  color: var(--accent);
}
.toggle-icon { font-size: 14px; }
</style>
