<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ close: [] }>()

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

const groups = computed(() => [
  {
    title: t('shortcuts.groupWorkspace'),
    shortcuts: [
      { keys: 'Ctrl+K', desc: t('shortcuts.globalSearch') },
      { keys: 'Ctrl+N', desc: t('shortcuts.newConnection') },
      { keys: 'Ctrl+T', desc: t('shortcuts.newTab') },
      { keys: 'Ctrl+W', desc: t('shortcuts.closeTab') },
      { keys: 'Ctrl+Tab', desc: t('shortcuts.nextTab') },
      { keys: 'Ctrl+Shift+Tab', desc: t('shortcuts.prevTab') },
      { keys: 'Alt+1~9', desc: t('shortcuts.jumpTab') },
      { keys: 'Ctrl+\\', desc: t('shortcuts.splitH') },
      { keys: 'Ctrl+Shift+\\', desc: t('shortcuts.splitV') },
      { keys: 'Alt+1', desc: t('shortcuts.layoutSingle') },
      { keys: 'Alt+2', desc: t('shortcuts.layoutLR') },
      { keys: 'Alt+3', desc: t('shortcuts.layoutTB') },
      { keys: 'Alt+4', desc: t('shortcuts.layoutGrid') },
      { keys: 'Alt+5', desc: t('shortcuts.layoutMain') },
      { keys: 'F11', desc: t('shortcuts.fullscreen') },
      { keys: 'F1', desc: t('shortcuts.toggleShortcuts') },
    ],
  },
  {
    title: t('shortcuts.groupSSH'),
    shortcuts: [
      { keys: 'Ctrl+Shift+C', desc: t('shortcuts.copy') },
      { keys: 'Ctrl+Shift+V', desc: t('shortcuts.paste') },
      { keys: 'Ctrl+F', desc: t('shortcuts.findTerminal') },
      { keys: 'Ctrl+L', desc: t('shortcuts.clearScreen') },
    ],
  },
  {
    title: t('shortcuts.groupSQL'),
    shortcuts: [
      { keys: 'Ctrl+Enter', desc: t('shortcuts.execute') },
      { keys: 'Ctrl+Shift+F', desc: t('shortcuts.formatSQL') },
      { keys: 'Ctrl+S', desc: t('shortcuts.saveQuery') },
      { keys: 'Ctrl+F', desc: t('shortcuts.find') },
      { keys: 'Ctrl+Shift+R', desc: t('shortcuts.findReplace') },
      { keys: 'Ctrl+Shift+Q', desc: t('shortcuts.globalQuery') },
      { keys: 'Ctrl+Shift+A', desc: t('shortcuts.aiAssistant') },
    ],
  },
  {
    title: t('shortcuts.groupFile'),
    shortcuts: [
      { keys: 'F2', desc: t('shortcuts.renameFile') },
      { keys: 'F4', desc: t('shortcuts.editFile') },
      { keys: 'F5', desc: t('shortcuts.download') },
      { keys: 'F6', desc: t('shortcuts.upload') },
      { keys: 'F7', desc: t('shortcuts.newFolder') },
      { keys: 'F8 / Delete', desc: t('shortcuts.deleteFile') },
      { keys: 'Ctrl+R', desc: t('shortcuts.refreshFiles') },
      { keys: 'Tab', desc: t('shortcuts.switchPanel') },
    ],
  },
])
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
            <div v-for="s in group.shortcuts" :key="s.keys" class="sp-row">
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
