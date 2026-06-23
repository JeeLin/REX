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
      @contextmenu.prevent="handleContextMenu"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'

const { t } = useI18n()
const { show: showMenu } = useContextMenu()

defineProps<{
  modelValue: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'execute': []
  'executeSelection': [sql: string]
  'save': []
  'showHistory': []
}>()

const editorRef = ref<HTMLTextAreaElement>()

function handleKeydown(e: KeyboardEvent) {
  // Ctrl+Enter / Cmd+Enter → 执行
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault()
    emit('execute')
    return
  }
  // Ctrl+S / Cmd+S → 保存
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    emit('save')
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

function replaceSelection(newText: string) {
  const el = editorRef.value
  if (!el) return
  const start = el.selectionStart
  const end = el.selectionEnd
  const value = el.value
  const newValue = value.substring(0, start) + newText + value.substring(end)
  emit('update:modelValue', newValue)
  requestAnimationFrame(() => {
    el.selectionStart = start
    el.selectionEnd = start + newText.length
    el.focus()
  })
}

function formatSql(): void {
  const el = editorRef.value
  if (!el) return
  const text = el.value
  const keywords = ['SELECT', 'FROM', 'WHERE', 'AND', 'OR', 'JOIN', 'LEFT', 'RIGHT',
    'INNER', 'OUTER', 'ON', 'GROUP BY', 'ORDER BY', 'HAVING', 'LIMIT', 'OFFSET',
    'INSERT', 'INTO', 'VALUES', 'UPDATE', 'SET', 'DELETE', 'CREATE', 'TABLE',
    'ALTER', 'DROP', 'INDEX', 'UNION', 'ALL', 'AS', 'DISTINCT', 'IN', 'NOT',
    'NULL', 'IS', 'BETWEEN', 'LIKE', 'EXISTS', 'CASE', 'WHEN', 'THEN', 'ELSE', 'END']
  let formatted = text.replace(/\s+/g, ' ').trim()
  for (const kw of keywords) {
    const re = new RegExp(`\\b${kw}\\b`, 'gi')
    formatted = formatted.replace(re, kw)
  }
  formatted = formatted
    .replace(/\bSELECT\b/g, '\nSELECT')
    .replace(/\bFROM\b/g, '\nFROM')
    .replace(/\bWHERE\b/g, '\nWHERE')
    .replace(/\bAND\b/g, '\n  AND')
    .replace(/\bOR\b/g, '\n  OR')
    .replace(/\bJOIN\b/g, '\nJOIN')
    .replace(/\bLEFT JOIN\b/g, '\nLEFT JOIN')
    .replace(/\bRIGHT JOIN\b/g, '\nRIGHT JOIN')
    .replace(/\bINNER JOIN\b/g, '\nINNER JOIN')
    .replace(/\bGROUP BY\b/g, '\nGROUP BY')
    .replace(/\bORDER BY\b/g, '\nORDER BY')
    .replace(/\bHAVING\b/g, '\nHAVING')
    .replace(/\bLIMIT\b/g, '\nLIMIT')
    .trim()
  emit('update:modelValue', formatted)
}

function convertCase(mode: 'upper' | 'lower' | 'title') {
  const el = editorRef.value
  if (!el) return
  const start = el.selectionStart
  const end = el.selectionEnd
  const text = el.value
  if (start === end) return
  const selected = text.substring(start, end)
  let converted: string
  if (mode === 'upper') converted = selected.toUpperCase()
  else if (mode === 'lower') converted = selected.toLowerCase()
  else converted = selected.replace(/\b\w/g, (c) => c.toUpperCase())
  emit('update:modelValue', text.substring(0, start) + converted + text.substring(end))
  requestAnimationFrame(() => {
    el.selectionStart = start
    el.selectionEnd = start + converted.length
    el.focus()
  })
}

function toggleComment() {
  const el = editorRef.value
  if (!el) return
  const start = el.selectionStart
  const text = el.value
  const lineStart = text.lastIndexOf('\n', start - 1) + 1
  const lineEnd = text.indexOf('\n', start)
  const end = lineEnd === -1 ? text.length : lineEnd
  const line = text.substring(lineStart, end)
  const commented = line.trimStart().startsWith('-- ')
  let newLine: string
  if (commented) {
    const indent = line.match(/^\s*/)?.[0] ?? ''
    newLine = indent + line.trimStart().slice(3)
  } else {
    newLine = line.replace(/^(\s*)/, '$1-- ')
  }
  emit('update:modelValue', text.substring(0, lineStart) + newLine + text.substring(end))
}

function insertTemplate(sql: string) {
  const el = editorRef.value
  if (!el) return
  const start = el.selectionStart
  const text = el.value
  const prefix = text.substring(0, start)
  const suffix = text.substring(el.selectionEnd)
  const insert = prefix && !prefix.endsWith('\n') && !prefix.endsWith(' ') ? '\n' + sql : sql
  emit('update:modelValue', prefix + insert + suffix)
  requestAnimationFrame(() => {
    const pos = start + insert.length
    el.selectionStart = el.selectionEnd = pos
    el.focus()
  })
}

const TEMPLATES: Record<string, string> = {
  select: 'SELECT * FROM  WHERE  LIMIT 100;',
  insert: 'INSERT INTO  () VALUES ();',
  update: 'UPDATE  SET  WHERE ;',
  delete: 'DELETE FROM  WHERE ;',
  createTable: 'CREATE TABLE  (\n  id INT PRIMARY KEY AUTO_INCREMENT,\n  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);',
}

function handleContextMenu(event: MouseEvent) {
  const el = editorRef.value
  if (!el) return
  const selection = el.value.substring(el.selectionStart, el.selectionEnd)
  showMenu(event, [
    {
      label: t('sql.ctx.executeSelection'),
      action: () => { emit('executeSelection', selection) },
      disabled: !selection,
    },
    {
      label: t('sql.ctx.executeAll'),
      action: () => { emit('execute') },
    },
    { separator: true },
    {
      label: t('sql.ctx.cut'),
      action: () => {
        navigator.clipboard.writeText(selection)
        replaceSelection('')
      },
      disabled: !selection,
    },
    {
      label: t('sql.ctx.copy'),
      action: () => { navigator.clipboard.writeText(selection) },
      disabled: !selection,
    },
    {
      label: t('sql.ctx.paste'),
      action: async () => {
        const text = await navigator.clipboard.readText()
        replaceSelection(text)
      },
    },
    { separator: true },
    {
      label: t('sql.ctx.format'),
      action: () => { formatSql() },
    },
    {
      label: t('sql.ctx.caseConvert'),
      children: [
        { label: t('sql.ctx.caseUpper'), action: () => convertCase('upper') },
        { label: t('sql.ctx.caseLower'), action: () => convertCase('lower') },
        { label: t('sql.ctx.caseTitle'), action: () => convertCase('title') },
      ],
    },
    {
      label: t('sql.ctx.toggleComment'),
      action: () => { toggleComment() },
    },
    { separator: true },
    {
      label: t('sql.ctx.save'),
      action: () => { emit('save') },
    },
    {
      label: t('sql.ctx.insertTemplate'),
      children: [
        { label: 'SELECT', action: () => insertTemplate(TEMPLATES.select) },
        { label: 'INSERT', action: () => insertTemplate(TEMPLATES.insert) },
        { label: 'UPDATE', action: () => insertTemplate(TEMPLATES.update) },
        { label: 'DELETE', action: () => insertTemplate(TEMPLATES.delete) },
        { label: 'CREATE TABLE', action: () => insertTemplate(TEMPLATES.createTable) },
      ],
    },
    {
      label: t('sql.ctx.history'),
      action: () => { emit('showHistory') },
    },
  ])
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
