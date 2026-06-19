<template>
  <div class="sql-editor-wrap">
    <textarea
      ref="editorRef"
      :value="modelValue"
      class="sql-editor"
      spellcheck="false"
      :placeholder="t('sql.placeholder')"
      @input="$emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
      @keydown="handleKeydown"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

defineProps<{
  modelValue: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'execute': []
}>()

const editorRef = ref<HTMLTextAreaElement>()

function handleKeydown(e: KeyboardEvent) {
  // Ctrl+Enter / Cmd+Enter → 执行
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault()
    emit('execute')
    return
  }
  // Tab → 插入两个空格
  if (e.key === 'Tab') {
    e.preventDefault()
    const el = editorRef.value
    if (!el) return
    const start = el.selectionStart
    const end = el.selectionEnd
    const value = el.value
    const newValue = value.substring(0, start) + '  ' + value.substring(end)
    emit('update:modelValue', newValue)
    // 恢复光标位置
    requestAnimationFrame(() => {
      el.selectionStart = el.selectionEnd = start + 2
    })
  }
}
</script>

<style scoped>
.sql-editor-wrap {
  flex: 1;
  min-height: 180px;
  position: relative;
  display: flex;
  flex-direction: column;
  border-bottom: 3px solid var(--border);
}

.sql-editor {
  flex: 1;
  width: 100%;
  padding: var(--sp-md);
  background: #0D1117;
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.5;
  border: none;
  outline: none;
  resize: none;
  tab-size: 2;
}

.sql-editor::placeholder {
  color: var(--text-muted);
}
</style>
