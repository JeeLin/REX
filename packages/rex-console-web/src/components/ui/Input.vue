<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue?: string
    placeholder?: string
    disabled?: boolean
    clearable?: boolean
    error?: string
    size?: 'sm' | 'md' | 'lg'
  }>(),
  { modelValue: '', size: 'md', disabled: false, clearable: false },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
  clear: []
}>()

const hasValue = computed(() => props.modelValue.length > 0)

function onClear() {
  emit('update:modelValue', '')
  emit('clear')
}
</script>

<template>
  <div class="input-wrap" :class="[`input-wrap--${size}`, { 'input-wrap--error': error, 'input-wrap--disabled': disabled }]">
    <slot name="prefix" />
    <input
      class="input"
      type="text"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />
    <button
      v-if="clearable && hasValue"
      class="input-clear"
      @click="onClear"
      tabindex="-1"
    >
      ×
    </button>
    <slot name="suffix" />
  </div>
  <div v-if="error" class="input-error">{{ error }}</div>
</template>

<style scoped>
.input-wrap {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  transition: border-color var(--transition), box-shadow var(--transition);
}
.input-wrap:focus-within {
  border-color: var(--accent);
  box-shadow: var(--ring);
}
.input-wrap--error {
  border-color: var(--danger);
}
.input-wrap--error:focus-within {
  box-shadow: 0 0 0 2px var(--danger);
}
.input-wrap--disabled {
  opacity: var(--disabled-opacity);
  cursor: not-allowed;
}
.input-wrap--sm { height: var(--input-height-sm); padding: 0 var(--space-2); }
.input-wrap--md { height: var(--input-height-md); padding: 0 var(--space-3); }
.input-wrap--lg { height: var(--input-height-lg); padding: 0 var(--space-4); }

.input {
  flex: 1;
  min-width: 0;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-primary);
  font-family: var(--font-sans);
  font-size: var(--text-base);
}
.input::placeholder {
  color: var(--text-muted);
}
.input:disabled {
  cursor: not-allowed;
}

.input-clear {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  background: var(--bg-elevated);
  border: none;
  border-radius: 50%;
  color: var(--text-muted);
  font-size: 12px;
  cursor: pointer;
  flex-shrink: 0;
  transition: color var(--transition), background var(--transition);
}
.input-clear:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.input-error {
  margin-top: var(--space-1);
  font-size: var(--text-xs);
  color: var(--danger);
}
</style>
