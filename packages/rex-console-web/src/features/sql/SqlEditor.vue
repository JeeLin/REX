<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, shallowRef } from 'vue'
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightSpecialChars } from '@codemirror/view'
import { EditorState } from '@codemirror/state'
import { defaultKeymap, indentWithTab, history, historyKeymap } from '@codemirror/commands'
import { sql, SQLite } from '@codemirror/lang-sql'
import { searchKeymap, highlightSelectionMatches } from '@codemirror/search'
import { foldGutter, indentOnInput, bracketMatching, foldKeymap } from '@codemirror/language'
import { autocompletion, completionKeymap } from '@codemirror/autocomplete'
import { formatSql } from './sql-format'

const props = defineProps<{
  modelValue: string
  database?: string
  readOnly?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  execute: [sql: string]
  save: [sql: string]
}>()

const editorRef = ref<HTMLDivElement>()
const view = shallowRef<EditorView>()

/* ---- zoom ---- */
const fontSize = ref(13)
const MIN_FONT = 9
const MAX_FONT = 24

function zoomIn() {
  fontSize.value = Math.min(MAX_FONT, fontSize.value + 1)
  applyFontSize()
}

function zoomOut() {
  fontSize.value = Math.max(MIN_FONT, fontSize.value - 1)
  applyFontSize()
}

function zoomReset() {
  fontSize.value = 13
  applyFontSize()
}

function applyFontSize() {
  if (!editorRef.value) return
  const el = editorRef.value.querySelector('.cm-editor') as HTMLElement
  if (el) el.style.fontSize = `${fontSize.value}px`
}

/* ---- clipboard history ---- */
const clipboardHistory = ref<string[]>([])
const CLIPBOARD_MAX = 10

function addToClipboard(text: string) {
  if (!text || !text.trim()) return
  // Deduplicate
  const idx = clipboardHistory.value.indexOf(text)
  if (idx !== -1) clipboardHistory.value.splice(idx, 1)
  clipboardHistory.value.unshift(text)
  if (clipboardHistory.value.length > CLIPBOARD_MAX) clipboardHistory.value.pop()
}

function pasteFromHistory(item: string) {
  if (!view.value) return
  const { from, to } = view.value.state.selection.main
  view.value.dispatch({
    changes: { from, to, insert: item },
  })
}

function handleCopy(_e: ClipboardEvent) {
  const sel = view.value?.state.sliceDoc(
    view.value.state.selection.main.from,
    view.value.state.selection.main.to,
  )
  if (sel) addToClipboard(sel)
}

/* ---- comment toggle ---- */
function toggleComment() {
  if (!view.value) return
  const { from, to } = view.value.state.selection.main
  const doc = view.value.state.doc
  // Find line boundaries
  const lineFrom = doc.lineAt(from).from
  const lineTo = doc.lineAt(to).to
  const selectedText = doc.sliceString(lineFrom, lineTo)
  const lines = selectedText.split('\n')
  const allCommented = lines.every(l => l.trimStart().startsWith('--'))

  const newLines = lines.map(l => {
    if (allCommented) {
      // Remove '-- ' prefix
      return l.replace(/^(\s*)--\s?/, '$1')
    } else {
      // Add '-- ' prefix
      return `${l.startsWith(' ') ? '' : ' '}-- ${l}`
    }
  })

  view.value.dispatch({
    changes: { from: lineFrom, to: lineTo, insert: newLines.join('\n') },
  })
}

/* ---- case toggle ---- */
function toggleCase() {
  if (!view.value) return
  const { from, to } = view.value.state.selection.main
  if (from === to) return
  const doc = view.value.state.doc
  const selectedText = doc.sliceString(from, to)
  const isUpper = selectedText === selectedText.toUpperCase()
  const newText = isUpper ? selectedText.toLowerCase() : selectedText.toUpperCase()
  view.value.dispatch({
    changes: { from, to, insert: newText },
  })
}

/* ---- format SQL ---- */
function format() {
  if (!view.value) return
  const doc = view.value.state.doc.toString()
  const formatted = formatSql(doc)
  view.value.dispatch({
    changes: { from: 0, to: view.value.state.doc.length, insert: formatted },
  })
}

function createTheme() {
  return EditorView.theme({
    '&': {
      backgroundColor: 'var(--bg-deep)',
      color: 'var(--text-primary)',
      fontSize: `${fontSize.value}px`,
      fontFamily: 'var(--font-mono)',
      height: '100%',
    },
    '.cm-content': {
      caretColor: 'var(--accent)',
      padding: 'var(--space-2) 0',
    },
    '.cm-cursor': {
      borderLeftColor: 'var(--accent)',
    },
    '&.cm-focused .cm-cursor': {
      borderLeftColor: 'var(--accent)',
    },
    '.cm-activeLine': {
      backgroundColor: 'rgba(255, 255, 255, 0.03)',
    },
    '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
      backgroundColor: 'rgba(88, 166, 255, 0.2) !important',
    },
    '.cm-gutters': {
      backgroundColor: 'var(--bg-deep)',
      color: 'var(--text-muted)',
      border: 'none',
      borderRight: '1px solid var(--border)',
    },
    '.cm-activeLineGutter': {
      backgroundColor: 'rgba(255, 255, 255, 0.05)',
    },
    '.cm-foldPlaceholder': {
      backgroundColor: 'var(--bg-elevated)',
      color: 'var(--text-muted)',
      border: '1px solid var(--border)',
    },
    '.cm-matchingBracket': {
      backgroundColor: 'rgba(232, 145, 45, 0.2)',
      outline: '1px solid var(--accent)',
    },
    '.cm-searchMatch': {
      backgroundColor: 'rgba(232, 145, 45, 0.3)',
      outline: '1px solid var(--accent)',
    },
    '.cm-searchMatch.cm-searchMatch-selected': {
      backgroundColor: 'rgba(232, 145, 45, 0.5)',
    },
  })
}

function createExtensions() {
  return [
    lineNumbers(),
    highlightActiveLine(),
    highlightSpecialChars(),
    history(),
    foldGutter(),
    indentOnInput(),
    bracketMatching(),
    autocompletion(),
    highlightSelectionMatches(),
    keymap.of([
      ...defaultKeymap,
      ...historyKeymap,
      ...foldKeymap,
      ...searchKeymap,
      ...completionKeymap,
      indentWithTab,
      {
        key: 'Ctrl-Enter',
        run: () => {
          emit('execute', view.value?.state.doc.toString() || '')
          return true
        },
      },
      {
        key: 'Cmd-Enter',
        run: () => {
          emit('execute', view.value?.state.doc.toString() || '')
          return true
        },
      },
      {
        key: 'Ctrl-s',
        run: () => {
          emit('save', view.value?.state.doc.toString() || '')
          return true
        },
      },
      {
        key: 'Cmd-s',
        run: () => {
          emit('save', view.value?.state.doc.toString() || '')
          return true
        },
      },
      {
        key: 'Ctrl-/',
        run: () => { toggleComment(); return true },
      },
      {
        key: 'Cmd-/',
        run: () => { toggleComment(); return true },
      },
      {
        key: 'Ctrl-Shift-u',
        run: () => { toggleCase(); return true },
      },
      {
        key: 'Ctrl-Shift-U',
        run: () => { toggleCase(); return true },
      },
    ]),
    sql({
      dialect: SQLite,
      upperCaseKeywords: true,
    }),
    EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        emit('update:modelValue', update.state.doc.toString())
      }
    }),
    createTheme(),
  ]
}

onMounted(() => {
  if (!editorRef.value) return
  const state = EditorState.create({
    doc: props.modelValue,
    extensions: createExtensions(),
  })
  view.value = new EditorView({
    state,
    parent: editorRef.value,
  })
  // Listen for copy events to track clipboard history
  document.addEventListener('copy', handleCopy)
})

onBeforeUnmount(() => {
  view.value?.destroy()
  document.removeEventListener('copy', handleCopy)
})

watch(
  () => props.modelValue,
  (val) => {
    if (view.value && val !== view.value.state.doc.toString()) {
      view.value.dispatch({
        changes: { from: 0, to: view.value.state.doc.length, insert: val },
      })
    }
  },
)

function focus() {
  view.value?.focus()
}

function getCursorPos(): number | undefined {
  return view.value?.state.selection.main.head
}

function getSelectedText(): string | undefined {
  if (!view.value) return undefined
  const { from, to } = view.value.state.selection.main
  if (from === to) return undefined
  return view.value.state.sliceDoc(from, to) || undefined
}

defineExpose({
  focus,
  format,
  toggleComment,
  toggleCase,
  zoomIn,
  zoomOut,
  zoomReset,
  clipboardHistory,
  pasteFromHistory,
  getCursorPos,
  getSelectedText,
})
</script>

<template>
  <div ref="editorRef" class="sql-editor" />
</template>

<style scoped>
.sql-editor {
  height: 100%;
  overflow: hidden;
}

.sql-editor :deep(.cm-editor) {
  height: 100%;
}

.sql-editor :deep(.cm-scroller) {
  overflow: auto;
}
</style>
