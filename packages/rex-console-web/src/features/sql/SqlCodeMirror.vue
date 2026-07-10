<template>
  <div ref="editorContainer" class="codemirror-wrap" />
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import { EditorState } from '@codemirror/state'
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightSpecialChars, drawSelection, rectangularSelection } from '@codemirror/view'
import { sql, SQLite, MySQL, PostgreSQL } from '@codemirror/lang-sql'
import { oneDark } from '@codemirror/theme-one-dark'
import { defaultKeymap, indentWithTab, history, historyKeymap } from '@codemirror/commands'
import { autocompletion, completionKeymap, closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete'
import { searchKeymap, highlightSelectionMatches } from '@codemirror/search'
import { lintKeymap } from '@codemirror/lint'
import { bracketMatching, indentOnInput, foldGutter, foldKeymap, syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language'

const lightTheme = EditorView.theme({
  '&': { backgroundColor: '#FFFFFF', color: '#1F2328' },
  '.cm-gutters': { backgroundColor: '#F6F8FA', color: '#656D76', borderRight: '1px solid #D0D7DE' },
  '.cm-activeLineGutter': { backgroundColor: '#F0F2F5' },
  '.cm-activeLine': { backgroundColor: '#F6F8FA' },
  '.cm-selectionBackground': { backgroundColor: '#ADD6FF !important' },
  '.cm-cursor': { borderLeftColor: '#BF5700' },
  '.cm-matchingBracket': { backgroundColor: '#BBF0FF', outline: '1px solid #96D3FF' },
})

function getCurrentTheme() {
  return document.documentElement.getAttribute('data-theme') || 'dark'
}

const props = defineProps<{
  modelValue: string
  placeholder?: string
  dialect?: 'mysql' | 'postgresql' | 'sqlite' | 'sql'
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'execute': []
  'executeSelection': [sql: string]
  'save': []
}>()

const editorContainer = ref<HTMLDivElement>()
let view: EditorView | null = null

function getSqlDialect() {
  switch (props.dialect) {
    case 'mysql': return MySQL
    case 'postgresql': return PostgreSQL
    case 'sqlite': return SQLite
    default: return undefined
  }
}

function createExtensions() {
  return [
    lineNumbers(),
    highlightActiveLine(),
    highlightSpecialChars(),
    drawSelection(),
    rectangularSelection(),
    history(),
    foldGutter(),
    indentOnInput(),
    bracketMatching(),
    closeBrackets(),
    highlightSelectionMatches(),
    autocompletion(),
    getCurrentTheme() === 'dark' ? oneDark : lightTheme,
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    keymap.of([
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...foldKeymap,
      ...completionKeymap,
      ...lintKeymap,
      indentWithTab,
      { key: 'Mod-Enter', run: () => { emit('execute'); return true } },
      { key: 'Mod-s', run: () => { emit('save'); return true } },
      { key: 'Mod-Shift-f', run: () => { formatSql(); return true } },
    ]),
    EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        emit('update:modelValue', update.state.doc.toString())
      }
    }),
    EditorView.theme({
      '&': {
        height: '100%',
        fontSize: '13px',
      },
      '.cm-scroller': {
        fontFamily: 'var(--font-mono)',
        overflow: 'auto',
      },
      '.cm-content': {
        padding: '8px 0',
      },
      '.cm-gutters': {
        borderRight: '1px solid var(--border)',
      },
    }),
  ]
}

let themeObserver: MutationObserver | null = null

onMounted(() => {
  if (!editorContainer.value) return
  const state = EditorState.create({
    doc: props.modelValue,
    extensions: createExtensions(),
  })
  view = new EditorView({
    state,
    parent: editorContainer.value,
  })

  themeObserver = new MutationObserver(() => {
    if (!view) return
    const currentDoc = view.state.doc.toString()
    view.destroy()
    const newState = EditorState.create({
      doc: currentDoc,
      extensions: createExtensions(),
    })
    view = new EditorView({
      state: newState,
      parent: editorContainer.value!,
    })
  })
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  })
})


onBeforeUnmount(() => {
  view?.destroy()
  view = null
  themeObserver?.disconnect()
  themeObserver = null
})


watch(() => props.modelValue, (newVal) => {
  if (view && view.state.doc.toString() !== newVal) {
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: newVal },
    })
  }
})

watch(() => props.dialect, () => {
  if (!view) return
  const currentDoc = view.state.doc.toString()
  view.destroy()
  const state = EditorState.create({
    doc: currentDoc,
    extensions: createExtensions(),
  })
  view = new EditorView({
    state,
    parent: editorContainer.value!,
  })
})

function insertText(text: string) {
  if (!view) return
  const pos = view.state.selection.main.head
  view.dispatch({
    changes: { from: pos, insert: text },
    selection: { anchor: pos + text.length },
  })
  view.focus()
}

function getSelection(): string {
  if (!view) return ''
  const { from, to } = view.state.selection.main
  return view.state.sliceDoc(from, to)
}

function replaceSelection(text: string) {
  if (!view) return
  const { from, to } = view.state.selection.main
  view.dispatch({
    changes: { from, to, insert: text },
    selection: { anchor: from + text.length },
  })
  view.focus()
}

function formatSql() {
  if (!view) return
  const text = view.state.doc.toString()
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
  view.dispatch({
    changes: { from: 0, to: view.state.doc.length, insert: formatted },
  })
}

function convertCase(mode: 'upper' | 'lower' | 'title') {
  if (!view) return
  const { from, to } = view.state.selection.main
  if (from === to) return
  const selected = view.state.sliceDoc(from, to)
  let converted: string
  if (mode === 'upper') converted = selected.toUpperCase()
  else if (mode === 'lower') converted = selected.toLowerCase()
  else converted = selected.replace(/\b\w/g, (c) => c.toUpperCase())
  view.dispatch({
    changes: { from, to, insert: converted },
    selection: { anchor: from, head: from + converted.length },
  })
  view.focus()
}

function toggleComment() {
  if (!view) return
  const { from } = view.state.selection.main
  const line = view.state.doc.lineAt(from)
  const lineText = line.text
  const commented = lineText.trimStart().startsWith('-- ')
  let newText: string
  if (commented) {
    const indent = lineText.match(/^\s*/)?.[0] ?? ''
    newText = indent + lineText.trimStart().slice(3)
  } else {
    newText = lineText.replace(/^(\s*)/, '$1-- ')
  }
  view.dispatch({
    changes: { from: line.from, to: line.to, insert: newText },
  })
}

defineExpose({
  insertText,
  getSelection,
  replaceSelection,
  formatSql,
  convertCase,
  toggleComment,
})
</script>

<style scoped>
.codemirror-wrap {
  flex: 1;
  min-height: 120px;
  overflow: hidden;
}

.codemirror-wrap :deep(.cm-editor) {
  height: 100%;
}
</style>
