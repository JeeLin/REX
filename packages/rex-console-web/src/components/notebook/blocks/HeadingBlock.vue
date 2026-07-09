<template>
  <component
    :is="headingTag"
    ref="el"
    :class="['heading-block', `h${level}`]"
    :contenteditable="true"
    :data-placeholder="placeholder"
    :data-block-id="blockId"
    @input="onInput"
    @keydown="onKeydown"
    @focus="$emit('focus')"
    @blur="$emit('blur')"
  />
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  blockId: string
  content: string
  level: number
  focused: boolean
  suppressKeyboard?: boolean
}>()

const emit = defineEmits<{
  'update:content': [content: string]
  'enter-pressed': []
  'backspace-empty': []
  'adjust-level': [direction: 1 | -1]
  focus: []
  blur: []
}>()

const { t } = useI18n()
const el = ref<HTMLElement>()
const headingTag = computed(() => `h${Math.min(3, Math.max(1, props.level))}`)

const placeholder = computed(() => {
  const key = `notebooks.editor.placeholder.h${props.level}`
  return t(key, t('notebooks.editor.placeholder.heading'))
})

watch(
  () => props.content,
  (val) => {
    if (el.value && el.value.textContent !== val) {
      el.value.textContent = val
    }
  }
)

onMounted(() => {
  if (el.value && !el.value.textContent) {
    el.value.textContent = props.content
  }
})

function onInput() {
  emit('update:content', el.value?.textContent ?? '')
}
function onKeydown(e: KeyboardEvent) {
  if (props.suppressKeyboard) {
    const controlKeys = new Set(['Enter', 'Backspace', 'Tab', 'ArrowUp', 'ArrowDown', 'Escape'])
    if (controlKeys.has(e.key)) {
      e.preventDefault()
    }
    return
  }
  if (e.key === 'Enter') {
    emit('enter-pressed')
  } else if (e.key === 'Backspace') {
    if (el.value?.textContent === '') {
      e.preventDefault()
      emit('backspace-empty')
    }
  } else if (e.key === 'Tab') {
    e.preventDefault()
    emit('adjust-level', e.shiftKey ? -1 : 1)
  }
}

defineExpose({ focus: () => el.value?.focus() })
</script>

<style scoped>
.heading-block {
  outline: none;
  width: 100%;
  min-height: 1.2em;
  border: none;
  background: transparent;
  font-family: var(--font-body);
  color: var(--text-primary);
  margin: 0;
  padding: 2px 0;
}

.heading-block:empty::before {
  content: attr(data-placeholder);
  color: var(--text-muted);
  pointer-events: none;
}

.h1 {
  font-size: var(--fs-2xl);
  font-weight: 700;
  line-height: var(--lh-tight);
  margin-top: 16px;
  margin-bottom: 8px;
}

.h2 {
  font-size: var(--fs-xl);
  font-weight: 600;
  line-height: var(--lh-tight);
  margin-top: 12px;
  margin-bottom: 6px;
}

.h3 {
  font-size: var(--fs-lg);
  font-weight: 600;
  line-height: var(--lh-tight);
  margin-top: 8px;
  margin-bottom: 4px;
}
</style>
