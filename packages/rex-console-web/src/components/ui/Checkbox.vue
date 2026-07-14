<script setup lang="ts">
const props = withDefaults(defineProps<{
  modelValue?: boolean
  label?: string
  disabled?: boolean
}>(), { modelValue: false, disabled: false })

const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()
</script>

<template>
  <label class="checkbox" :class="{ 'checkbox--disabled': disabled }">
    <input
      type="checkbox"
      class="checkbox-input"
      :checked="modelValue"
      :disabled="disabled"
      @change="emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    />
    <span class="checkbox-box">
      <svg v-if="modelValue" class="checkbox-icon" viewBox="0 0 12 12" fill="none">
        <path d="M2 6l3 3 5-5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </span>
    <span v-if="label" class="checkbox-label">{{ label }}</span>
    <slot />
  </label>
</template>

<style scoped>
.checkbox {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  cursor: pointer;
  font-size: var(--text-base);
  color: var(--text-primary);
  user-select: none;
}
.checkbox--disabled { opacity: var(--disabled-opacity); cursor: not-allowed; }
.checkbox-input { position: absolute; opacity: 0; width: 0; height: 0; }
.checkbox-box {
  width: 16px;
  height: 16px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background var(--transition), border-color var(--transition);
}
.checkbox-input:checked + .checkbox-box {
  background: var(--accent);
  border-color: var(--accent);
  color: var(--text-on-accent);
}
.checkbox-input:focus-visible + .checkbox-box {
  box-shadow: var(--ring);
}
.checkbox-icon { width: 12px; height: 12px; }
</style>
