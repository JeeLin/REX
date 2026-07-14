<script setup lang="ts">
withDefaults(defineProps<{
  modelValue?: boolean
  disabled?: boolean
  size?: 'sm' | 'md'
}>(), { modelValue: false, disabled: false, size: 'md' })

const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()
</script>

<template>
  <button
    class="switch"
    :class="[`switch--${size}`, { 'switch--on': modelValue, 'switch--disabled': disabled }]"
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    @click="emit('update:modelValue', !modelValue)"
  >
    <span class="switch-thumb" />
  </button>
</template>

<style scoped>
.switch {
  position: relative;
  display: inline-flex;
  align-items: center;
  border: none;
  border-radius: var(--radius-pill);
  background: var(--border-strong);
  cursor: pointer;
  transition: background var(--transition);
  flex-shrink: 0;
}
.switch:focus-visible { box-shadow: var(--ring); }
.switch--sm { width: 32px; height: 18px; }
.switch--md { width: 40px; height: 22px; }
.switch--on { background: var(--accent); }
.switch--disabled { opacity: var(--disabled-opacity); cursor: not-allowed; }
.switch-thumb {
  position: absolute;
  border-radius: 50%;
  background: #fff;
  transition: transform var(--transition);
}
.switch--sm .switch-thumb { width: 14px; height: 14px; left: 2px; }
.switch--md .switch-thumb { width: 18px; height: 18px; left: 2px; }
.switch--on.switch--sm .switch-thumb { transform: translateX(14px); }
.switch--on.switch--md .switch-thumb { transform: translateX(18px); }
</style>
