<script setup lang="ts">
import { ref, watch, onBeforeUnmount, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightSpecialChars } from '@codemirror/view'
import { EditorState } from '@codemirror/state'
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands'
import { bracketMatching, foldGutter, foldKeymap } from '@codemirror/language'
import { searchKeymap, highlightSelectionMatches } from '@codemirror/search'
import { autocompletion, completionKeymap } from '@codemirror/autocomplete'
import { oneDark } from '@codemirror/theme-one-dark'
import { javascript } from '@codemirror/lang-javascript'
import { python } from '@codemirror/lang-python'
import { sql } from '@codemirror/lang-sql'
import { json } from '@codemirror/lang-json'
import { html } from '@codemirror/lang-html'
import { css } from '@codemirror/lang-css'
import { markdown } from '@codemirror/lang-markdown'
import { xml } from '@codemirror/lang-xml'
import { rust } from '@codemirror/lang-rust'
import { cpp } from '@codemirror/lang-cpp'
import { java } from '@codemirror/lang-java'
import { yaml } from '@codemirror/lang-yaml'
import * as filesApi from '@/api/files'
import Button from '@/components/ui/Button.vue'
const { t } = useI18n()


const props = defineProps<{
  visible: boolean
  sessionId: string
  filePath: string
  protocol: 'sftp' | 's3'
}>()

const emit = defineEmits<{
  close: []
  saved: []
}>()

const loading = ref(false)
const saving = ref(false)
const error = ref('')
const filename = ref('')
const fileSize = ref(0)
const editorContainer = ref<HTMLElement | null>(null)
let editorView: EditorView | null = null

const LANG_MAP: Record<string, () => ReturnType<typeof javascript>> = {
  '.ts': () => javascript({ typescript: true }),
  '.js': () => javascript(),
  '.jsx': () => javascript({ jsx: true }),
  '.tsx': () => javascript({ typescript: true, jsx: true }),
  '.vue': () => html(),
  '.py': () => python(),
  '.sql': () => sql(),
  '.json': () => json(),
  '.html': () => html(),
  '.htm': () => html(),
  '.css': () => css(),
  '.scss': () => css(),
  '.md': () => markdown(),
  '.xml': () => xml(),
  '.rs': () => rust(),
  '.c': () => cpp(),
  '.cpp': () => cpp(),
  '.h': () => cpp(),
  '.java': () => java(),
  '.yaml': () => yaml(),
  '.yml': () => yaml(),
}

function getLanguageExtension(filePath: string) {
  const ext = '.' + filePath.split('.').pop()?.toLowerCase()
  return LANG_MAP[ext]?.() || []
}

function createEditor(content: string) {
  if (editorView) {
    editorView.destroy()
    editorView = null
  }

  const state = EditorState.create({
    doc: content,
    extensions: [
      lineNumbers(),
      highlightActiveLine(),
      highlightSpecialChars(),
      history(),
      foldGutter(),
      bracketMatching(),
      autocompletion(),
      highlightSelectionMatches(),
      keymap.of([
        ...defaultKeymap,
        ...historyKeymap,
        ...foldKeymap,
        ...searchKeymap,
        ...completionKeymap,
        { key: 'Mod-s', run: () => { save(); return true } },
      ]),
      getLanguageExtension(props.filePath),
      oneDark,
      EditorView.theme({
        '&': { height: '100%', fontSize: '13px' },
        '.cm-scroller': { overflow: 'auto' },
      }),
    ],
  })

  editorView = new EditorView({ state, parent: editorContainer.value! })
}

async function loadFile() {
  if (!props.sessionId || !props.filePath) return
  loading.value = true
  error.value = ''
  try {
    const result = await filesApi.readForEdit(props.sessionId, props.filePath)
    filename.value = result.filename
    fileSize.value = result.size
    const content = atob(result.content)
    await nextTick()
    createEditor(content)
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function save() {
  if (!editorView || saving.value) return
  saving.value = true
  error.value = ''
  try {
    const content = editorView.state.doc.toString()
    const encoded = btoa(unescape(encodeURIComponent(content)))
    await filesApi.saveFromEdit(props.sessionId, props.filePath, encoded)
    emit('saved')
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    saving.value = false
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

watch(() => props.visible, (v) => {
  if (v) loadFile()
  else if (editorView) { editorView.destroy(); editorView = null }
})

onBeforeUnmount(() => {
  if (editorView) { editorView.destroy(); editorView = null }
})
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="editor-overlay" @click.self="emit('close')">
      <div class="editor-dialog">
        <!-- Header -->
        <div class="editor-header">
          <div class="editor-title">
            <span class="editor-filename">{{ filename || filePath }}</span>
            <span class="editor-meta">{{ formatSize(fileSize) }}</span>
          </div>
          <div class="editor-actions">
            <Button variant="primary" size="sm" :disabled="saving || loading" @click="save">
              {{ saving ? t('files.saving') : t('files.save') + ' (Ctrl+S)' }}
            </Button>
            <Button variant="ghost" size="sm" @click="emit('close')">{{ t('files.close') }}</Button>
          </div>
        </div>

        <!-- Error -->
        <div v-if="error" class="editor-error">{{ error }}</div>

        <!-- Loading -->
        <div v-if="loading" class="editor-loading">{{ t('files.loadingFile') }}</div>

        <!-- Editor -->
        <div ref="editorContainer" class="editor-content" />
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.editor-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
}

.editor-dialog {
  width: 90vw;
  height: 85vh;
  max-width: 1200px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.editor-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
  background: var(--bg-elevated);
}

.editor-title {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
}

.editor-filename {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.editor-meta {
  font-size: var(--text-xs);
  color: var(--text-secondary);
}

.editor-actions {
  display: flex;
  gap: var(--space-2);
  flex-shrink: 0;
}

.editor-error {
  padding: var(--space-2) var(--space-4);
  background: var(--danger-soft);
  color: var(--danger);
  font-size: var(--text-xs);
  border-bottom: 1px solid var(--border);
}

.editor-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.editor-content {
  flex: 1;
  overflow: hidden;
}
</style>
