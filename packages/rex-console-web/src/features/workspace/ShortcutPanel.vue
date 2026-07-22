<script setup lang="ts">
defineProps<{ show: boolean }>()
const emit = defineEmits<{ close: [] }>()

const groups = [
  {
    title: 'Workspace',
    shortcuts: [
      { keys: 'Ctrl+K', desc: 'Global search / Command palette' },
      { keys: 'Ctrl+N', desc: 'New connection' },
      { keys: 'Ctrl+T', desc: 'New tab (same protocol)' },
      { keys: 'Ctrl+W', desc: 'Close current tab' },
      { keys: 'Ctrl+Tab', desc: 'Switch next tab' },
      { keys: 'Ctrl+Shift+Tab', desc: 'Switch previous tab' },
      { keys: 'Alt+1~9', desc: 'Jump to tab N' },
      { keys: 'Ctrl+\\', desc: 'Split horizontal' },
      { keys: 'Ctrl+Shift+\\', desc: 'Split vertical' },
      { keys: 'Alt+1', desc: 'Layout: single pane' },
      { keys: 'Alt+2', desc: 'Layout: left-right split' },
      { keys: 'Alt+3', desc: 'Layout: top-bottom split' },
      { keys: 'Alt+4', desc: 'Layout: four grid' },
      { keys: 'Alt+5', desc: 'Layout: main + side' },
      { keys: 'F11', desc: 'Toggle fullscreen' },
      { keys: 'F1', desc: 'Toggle shortcuts panel' },
    ],
  },
  {
    title: 'SSH Terminal',
    shortcuts: [
      { keys: 'Ctrl+Shift+C', desc: 'Copy selection' },
      { keys: 'Ctrl+Shift+V', desc: 'Paste (bracketed)' },
      { keys: 'Ctrl+F', desc: 'Find in terminal' },
      { keys: 'Ctrl+L', desc: 'Clear screen' },
    ],
  },
  {
    title: 'SQL Console',
    shortcuts: [
      { keys: 'Ctrl+Enter', desc: 'Execute (Run)' },
      { keys: 'Ctrl+Shift+F', desc: 'Format SQL' },
      { keys: 'Ctrl+S', desc: 'Save query' },
      { keys: 'Ctrl+F', desc: 'Find' },
      { keys: 'Ctrl+Shift+R', desc: 'Find & Replace' },
      { keys: 'Ctrl+Shift+Q', desc: 'Global query' },
      { keys: 'Ctrl+Shift+A', desc: 'AI assistant' },
    ],
  },
  {
    title: 'File Manager',
    shortcuts: [
      { keys: 'F2', desc: 'Rename' },
      { keys: 'F4', desc: 'Edit (temporary download)' },
      { keys: 'F5', desc: 'Download (active → opposite)' },
      { keys: 'F6', desc: 'Upload (active → opposite)' },
      { keys: 'F7', desc: 'New folder' },
      { keys: 'F8 / Delete', desc: 'Delete' },
      { keys: 'Ctrl+R', desc: 'Refresh' },
      { keys: 'Tab', desc: 'Switch active panel' },
    ],
  },
]
</script>

<template>
  <Teleport to="body">
    <Transition name="overlay">
      <div v-if="show" class="shortcut-overlay" @click="emit('close')" />
    </Transition>
    <Transition name="panel">
      <div v-if="show" class="shortcut-panel">
        <header class="sp-header">
          <h3 class="sp-title mono">Keyboard Shortcuts</h3>
          <button class="sp-close" @click="emit('close')">×</button>
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
