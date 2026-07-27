<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'

interface Option {
  label: string
  value: string | number
  disabled?: boolean
}

const props = withDefaults(defineProps<{
  modelValue?: string | number
  options: Option[]
  placeholder?: string
  disabled?: boolean
  size?: 'sm' | 'md' | 'lg'
}>(), { modelValue: '', placeholder: '', size: 'md', disabled: false })

const emit = defineEmits<{ 'update:modelValue': [value: string | number] }>()

const open = ref(false)
const triggerRef = ref<HTMLElement | null>(null)
const dropdownStyle = ref<Record<string, string>>({})
const selectedLabel = computed(() => props.options.find(o => o.value === props.modelValue)?.label ?? '')

function updatePosition() {
  if (!triggerRef.value) return
  const rect = triggerRef.value.getBoundingClientRect()
  dropdownStyle.value = {
    position: 'fixed',
    top: `${rect.bottom + 4}px`,
    left: `${rect.left}px`,
    minWidth: `${rect.width}px`,
  }
}

watch(open, async (v) => {
  if (v) {
    await nextTick()
    updatePosition()
  }
})

function select(option: Option) {
  if (!option.disabled) {
    emit('update:modelValue', option.value)
    open.value = false
  }
}
</script>

<template>
  <div class="select-wrap" :class="[`select-wrap--${size}`, { 'select-wrap--disabled': disabled }]">
    <button
      ref="triggerRef"
      class="select-trigger"
      :disabled="disabled"
      @click="open = !open"
    >
      <span :class="{ 'muted': !selectedLabel }">{{ selectedLabel || placeholder }}</span>
      <span class="select-arrow" :class="{ 'select-arrow--open': open }">▾</span>
    </button>
    <Teleport to="body">
      <div v-if="open" class="select-overlay" @click="open = false" />
      <Transition name="select">
        <div v-if="open" class="select-dropdown" :class="`select-dropdown--${size}`" :style="dropdownStyle">
          <div
            v-for="option in options"
            :key="option.value"
            class="select-option"
            :class="{ 'select-option--selected': option.value === modelValue, 'select-option--disabled': option.disabled }"
            @click="select(option)"
          >
            {{ option.label }}
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.select-wrap {
  position: relative;
  display: inline-flex;
}
.select-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  min-width: 160px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-primary);
  font-family: var(--font-sans);
  font-size: var(--text-base);
  cursor: pointer;
  transition: border-color var(--transition), box-shadow var(--transition);
}
.select-trigger:focus-visible {
  outline: none;
  border-color: var(--accent);
  box-shadow: var(--ring);
}
.select-wrap--sm .select-trigger { height: var(--input-height-sm); padding: 0 var(--space-2); font-size: var(--text-sm); }
.select-wrap--md .select-trigger { height: var(--input-height-md); padding: 0 var(--space-3); }
.select-wrap--lg .select-trigger { height: var(--input-height-lg); padding: 0 var(--space-4); }
.select-wrap--disabled { opacity: var(--disabled-opacity); cursor: not-allowed; }
.select-arrow {
  font-size: 10px;
  color: var(--text-muted);
  transition: transform var(--transition);
}
.select-arrow--open { transform: rotate(180deg); }

.select-overlay {
  position: fixed;
  inset: 0;
  z-index: 90;
}
.select-dropdown {
  max-height: 240px;
  overflow-y: auto;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  z-index: 100;
  padding: var(--space-1) 0;
}
.select-option {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-base);
  color: var(--text-primary);
  cursor: pointer;
  transition: background var(--transition);
}
.select-option:hover { background: var(--bg-hover); }
.select-option--selected { color: var(--accent); background: var(--accent-soft); }
.select-option--disabled { opacity: var(--disabled-opacity); cursor: not-allowed; }
.select-enter-active, .select-leave-active { transition: opacity var(--transition), transform var(--transition); }
.select-enter-from, .select-leave-to { opacity: 0; transform: var(--slide-up); }
</style>
