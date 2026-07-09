<template>
  <div
    ref="el"
    class="paragraph-block"
    :contenteditable="true"
    :data-placeholder="t('notebooks.editor.placeholder.paragraph')"
    :data-block-id="blockId"
    @input="onInput"
    @keydown="onKeydown"
    @focus="$emit('focus')"
    @blur="$emit('blur')"
  />
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  blockId: string
  content: string
  focused: boolean
  suppressKeyboard?: boolean
}>()

const emit = defineEmits<{
  'update:content': [content: string]
  'enter-pressed': []
  'backspace-empty': []
  focus: []
  blur: []
}>()

const { t } = useI18n()
const el = ref<HTMLElement>()

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
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    emit('enter-pressed')
  } else if (e.key === 'Backspace') {
    if (el.value?.textContent === '') {
      e.preventDefault()
      emit('backspace-empty')
    }
  }
}

defineExpose({ focus: () => el.value?.focus() })
</script>

<style scoped>
.paragraph-block {
  outline: none;
  width: 100%;
  min-height: 1.5em;
  border: none;
  background: transparent;
  font-family: var(--font-body);
  font-size: var(--fs-base);
  line-height: var(--lh-base);
  color: var(--text-primary);
  padding: 2px 0;
}

.paragraph-block:empty::before {
  content: attr(data-placeholder);
  color: var(--text-muted);
  pointer-events: none;
}
</style>
