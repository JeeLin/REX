<script lang="ts">
// Static key table behind the shortcut panel. `title`/`desc` are i18n keys
// resolved at render time. `scope` disambiguates keys that do different
// things depending on focus (the three Ctrl+Shift+F meanings).
export type ShortcutScope = 'editorFocused' | 'editorBlurred' | 'terminalFocused'

export interface PanelShortcut {
  keys: string
  desc: string
  scope?: ShortcutScope
}

export interface PanelGroup {
  title: string
  shortcuts: PanelShortcut[]
}

export const SHORTCUT_PANEL_GROUPS: PanelGroup[] = [
  {
    title: 'shortcuts.groupWorkspace',
    shortcuts: [
      { keys: 'Ctrl+K', desc: 'shortcuts.globalSearch' },
      { keys: 'Ctrl+Shift+N', desc: 'shortcuts.newConnection' },
      { keys: 'Alt+T', desc: 'shortcuts.newTab' },
      { keys: 'Alt+W', desc: 'shortcuts.closeTab' },
      { keys: 'Ctrl+Shift+→', desc: 'shortcuts.nextTab' },
      { keys: 'Ctrl+Shift+←', desc: 'shortcuts.prevTab' },
      { keys: 'Alt+1~9', desc: 'shortcuts.jumpTab' },
      { keys: 'Ctrl+\\', desc: 'shortcuts.splitH' },
      { keys: 'Ctrl+Shift+\\', desc: 'shortcuts.splitV' },
      { keys: 'Ctrl+Alt+1', desc: 'shortcuts.layoutSingle' },
      { keys: 'Ctrl+Alt+2', desc: 'shortcuts.layoutLR' },
      { keys: 'Ctrl+Alt+3', desc: 'shortcuts.layoutTB' },
      { keys: 'Ctrl+Alt+4', desc: 'shortcuts.layoutGrid' },
      { keys: 'Ctrl+Alt+5', desc: 'shortcuts.layoutMain' },
      { keys: 'F11', desc: 'shortcuts.fullscreen' },
      { keys: 'F1', desc: 'shortcuts.toggleShortcuts' },
    ],
  },
  {
    title: 'shortcuts.groupSSH',
    shortcuts: [
      { keys: 'Ctrl+Shift+C', desc: 'shortcuts.copy' },
      { keys: 'Ctrl+Shift+V', desc: 'shortcuts.paste' },
      { keys: 'Ctrl+F', desc: 'shortcuts.findTerminal' },
      { keys: 'Ctrl+L', desc: 'shortcuts.clearScreen' },
      { keys: 'Ctrl+Shift+F', desc: 'terminal.openSftp', scope: 'terminalFocused' },
    ],
  },
  {
    title: 'shortcuts.groupSQL',
    shortcuts: [
      { keys: 'Ctrl+Enter', desc: 'shortcuts.execute' },
      { keys: 'Ctrl+Shift+F', desc: 'shortcuts.formatSQL', scope: 'editorFocused' },
      { keys: 'Ctrl+Shift+F', desc: 'shortcuts.globalSearch', scope: 'editorBlurred' },
      { keys: 'Ctrl+S', desc: 'shortcuts.saveQuery' },
      { keys: 'Ctrl+F', desc: 'shortcuts.find' },
      { keys: 'Ctrl+Shift+R', desc: 'shortcuts.findReplace' },
      { keys: 'Ctrl+Shift+Q', desc: 'shortcuts.globalQuery' },
      { keys: 'Ctrl+Shift+A', desc: 'shortcuts.aiAssistant' },
    ],
  },
  {
    title: 'shortcuts.groupFile',
    shortcuts: [
      { keys: 'F2', desc: 'shortcuts.renameFile' },
      { keys: 'F7', desc: 'shortcuts.newFolder' },
      { keys: 'F8 / Delete', desc: 'shortcuts.deleteFile' },
      { keys: 'Ctrl+R', desc: 'shortcuts.refreshFiles' },
    ],
  },
]
</script>

<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'

const { t, locale } = useI18n()
const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ close: [] }>()

// Focus-scope qualifiers for context-sensitive keys. Kept local to this
// component because locale JSON files are outside this change's file scope;
// move them into i18n locales when those files are next touched.
const SCOPE_QUALIFIERS: Record<ShortcutScope, { zh: string; en: string }> = {
  editorFocused: { zh: '编辑器聚焦时', en: 'editor focused' },
  editorBlurred: { zh: '编辑器未聚焦时', en: 'editor not focused' },
  terminalFocused: { zh: '终端聚焦时', en: 'terminal focused' },
}

function scopeSuffix(scope?: ShortcutScope): string {
  if (!scope) return ''
  const zh = locale.value === 'zh'
  const label = SCOPE_QUALIFIERS[scope][zh ? 'zh' : 'en']
  return zh ? `（${label}）` : ` (${label})`
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('close')
}

watch(() => props.show, (visible) => {
  if (visible) {
    document.addEventListener('keydown', handleKeydown)
  } else {
    document.removeEventListener('keydown', handleKeydown)
  }
})

const groups = computed(() =>
  SHORTCUT_PANEL_GROUPS.map((group) => ({
    title: t(group.title),
    shortcuts: group.shortcuts.map((s) => ({
      keys: s.keys,
      desc: t(s.desc) + scopeSuffix(s.scope),
    })),
  })),
)
</script>

<template>
  <Teleport to="body">
    <Transition name="overlay">
      <div v-if="show" class="shortcut-overlay" @click="emit('close')" />
    </Transition>
    <Transition name="panel">
      <div v-if="show" class="shortcut-panel">
        <header class="sp-header">
          <h3 class="sp-title mono">{{ t('shortcuts.title') }}</h3>
          <button class="sp-close" aria-label="Close" @click="emit('close')">×</button>
        </header>
        <div class="sp-body">
          <div v-for="group in groups" :key="group.title" class="sp-group">
            <h4 class="sp-group-title mono">{{ group.title }}</h4>
            <div v-for="s in group.shortcuts" :key="s.desc" class="sp-row">
              <kbd class="sp-keys mono">{{ s.keys }}</kbd>
              <span class="sp-desc">{{ s.desc }}</span>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.shortcut-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(2px);
  z-index: 80;
}
.shortcut-panel {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 420px;
  max-height: 80vh;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  z-index: 90;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.sp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}
.sp-title {
  font-size: var(--text-md);
  font-weight: 600;
}
.sp-close {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-md);
  cursor: pointer;
}
.sp-close:hover { color: var(--text-primary); }
.sp-body {
  padding: var(--space-4);
  overflow-y: auto;
}
.sp-group {
  margin-bottom: var(--space-4);
}
.sp-group:last-child { margin-bottom: 0; }
.sp-group-title {
  font-size: var(--text-xs);
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: var(--space-2);
}
.sp-row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-1) 0;
}
.sp-keys {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 120px;
  padding: 2px 8px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  text-align: center;
}
.sp-desc {
  font-size: var(--text-sm);
  color: var(--text-primary);
}
.overlay-enter-active, .overlay-leave-active { transition: opacity var(--transition); }
.overlay-enter-from, .overlay-leave-to { opacity: 0; }
.panel-enter-active, .panel-leave-active { transition: opacity var(--transition), transform var(--transition); }
.panel-enter-from, .panel-leave-to { opacity: 0; transform: translate(-50%, -50%) scale(0.96); }
</style>
