<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, shallowRef } from 'vue'
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightSpecialChars } from '@codemirror/view'
import { EditorState } from '@codemirror/state'
import { defaultKeymap, indentWithTab, history, historyKeymap } from '@codemirror/commands'
import { sql, SQLite, MySQL, PostgreSQL } from '@codemirror/lang-sql'
import { searchKeymap, highlightSelectionMatches } from '@codemirror/search'
import { foldGutter, indentOnInput, bracketMatching, foldKeymap } from '@codemirror/language'
import { autocompletion, completionKeymap } from '@codemirror/autocomplete'

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

function createTheme() {
  return EditorView.theme({
    '&': {
      backgroundColor: 'var(--bg-deep)',
      color: 'var(--text-primary)',
      fontSize: 'var(--text-sm)',
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
})

onBeforeUnmount(() => {
  view.value?.destroy()
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

defineExpose({ focus })
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
