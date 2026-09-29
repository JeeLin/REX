<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { SHORTCUT_PANEL_GROUPS, type ShortcutScope } from './shortcuts'

const { t, locale } = useI18n()
const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ close: [] }>()

// Focus-scope qualifiers for context-sensitive keys, resolved through i18n.
const SCOPE_QUALIFIERS: Record<ShortcutScope, string> = {
  editorFocused: 'shortcuts.scopeEditorFocused',
  editorBlurred: 'shortcuts.scopeEditorBlurred',
  terminalFocused: 'shortcuts.scopeTerminalFocused',
}

function scopeSuffix(scope?: ShortcutScope): string {
  if (!scope) return ''
  const label = t(SCOPE_QUALIFIERS[scope])
  return locale.value === 'zh' ? `（${label}）` : ` (${label})`
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
