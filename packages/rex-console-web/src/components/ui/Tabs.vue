<script setup lang="ts">
defineProps<{ modelValue: string; tabs: string[] }>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
</script>

<template>
  <div class="tabs">
    <div class="tabs-bar">
      <button
        v-for="tab in tabs"
        :key="tab"
        class="tab"
        :class="{ 'tab--active': tab === modelValue }"
        @click="emit('update:modelValue', tab)"
      >
        <slot name="item" :tab="tab" :active="tab === modelValue">
          {{ tab }}
        </slot>
      </button>
    </div>
    <div class="tabs-body">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.tabs {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.tabs-bar {
  display: flex;
  gap: 2px;
  border-bottom: 1px solid var(--border);
  overflow-x: auto;
}
.tab {
  padding: var(--space-2) var(--space-4);
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--text-secondary);
  font-size: var(--text-base);
  white-space: nowrap;
  transition: color var(--transition);
}
.tab:hover {
  color: var(--text-primary);
}
.tab--active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}
.tabs-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
</style>
