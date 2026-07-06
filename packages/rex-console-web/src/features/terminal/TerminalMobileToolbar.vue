<template>
  <div v-if="visible" class="mobile-toolbar">
    <!-- 方向键区 -->
    <div class="toolbar-row direction-keys">
      <button
        class="toolbar-btn"
        @pointerdown="sendKey('\x1b[A')"
        @pointerup="stopRepeat"
      >↑</button>
      <button
        class="toolbar-btn"
        @pointerdown="sendKey('\x1b[B')"
        @pointerup="stopRepeat"
      >↓</button>
      <button
        class="toolbar-btn"
        @pointerdown="sendKey('\x1b[D')"
        @pointerup="stopRepeat"
      >←</button>
      <button
        class="toolbar-btn"
        @pointerdown="sendKey('\x1b[C')"
        @pointerup="stopRepeat"
      >→</button>
      <div class="toolbar-sep"></div>
      <button class="toolbar-btn" @click="sendKey('\t')">Tab</button>
      <button class="toolbar-btn" @click="sendKey('\r')">⏎</button>
      <div class="toolbar-sep"></div>
      <button class="toolbar-btn ctrl-key" @click="sendKey('\x03')">^C</button>
      <button class="toolbar-btn ctrl-key" @click="sendKey('\x0c')">^L</button>
    </div>

    <!-- 功能按钮区 -->
    <div class="toolbar-row function-keys">
      <button class="toolbar-btn func-btn" @click="$emit('openHistory')">
        <span class="btn-icon">📜</span>
        <span class="btn-label">{{ t('terminal.mobile.history') }}</span>
      </button>
      <button class="toolbar-btn func-btn" @click="$emit('openPaste')">
        <span class="btn-icon">📋</span>
        <span class="btn-label">{{ t('terminal.mobile.paste') }}</span>
      </button>
      <div class="toolbar-sep"></div>
      <button class="toolbar-btn func-btn" @click="$emit('fontSizeChange', -1)">A-</button>
      <button class="toolbar-btn func-btn" @click="$emit('fontSizeChange', 1)">A+</button>
      <div class="toolbar-sep"></div>
      <button class="toolbar-btn func-btn" @click="showMoreMenu = !showMoreMenu">
        <span class="btn-icon">⚙</span>
        <span class="btn-label">{{ t('terminal.mobile.more') }}</span>
      </button>
    </div>

    <!-- 更多选项菜单 -->
    <div v-if="showMoreMenu" class="more-menu">
      <button class="more-menu-item" @click="handleMoreAction('clear')">
        {{ t('ws.terminal.toolbar.clear') }}
      </button>
      <button class="more-menu-item" @click="handleMoreAction('sftp')">
        📁 SFTP
      </button>
      <button class="more-menu-item" @click="handleMoreAction('fullscreen')">
        ⛶ {{ t('ws.terminal.statusbar.fullscreen') }}
      </button>
      <button class="more-menu-item danger" @click="handleMoreAction('disconnect')">
        {{ t('ws.terminal.toolbar.disconnect') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Terminal } from '@xterm/xterm'

const props = defineProps<{
  terminal: Terminal | null
  visible: boolean
}>()

defineEmits<{
  openHistory: []
  openPaste: []
  fontSizeChange: [delta: number]
  action: [type: string]
}>()

const { t } = useI18n()
const showMoreMenu = ref(false)
let repeatTimer: ReturnType<typeof setInterval> | null = null

function sendKey(data: string) {
  props.terminal?.focus()
  props.terminal?.focus()
  // xterm.js textarea focus then write
  props.terminal?.textarea?.dispatchEvent(
    new KeyboardEvent('keydown', { key: data, bubbles: true })
  )
  // Direct write approach for reliable input
  const textarea = props.terminal?.textarea
  if (textarea) {
    // Use native input setter to bypass xterm.js key handling
    const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
      window.HTMLTextAreaElement.prototype, 'value'
    )?.set
    if (nativeInputValueSetter) {
      nativeInputValueSetter.call(textarea, data)
      textarea.dispatchEvent(new Event('input', { bubbles: true }))
    }
  }
}

function startRepeat(data: string) {
  stopRepeat()
  repeatTimer = setInterval(() => sendKey(data), 100)
}

function stopRepeat() {
  if (repeatTimer) {
    clearInterval(repeatTimer)
    repeatTimer = null
  }
}

function handleMoreAction(type: string) {
  showMoreMenu.value = false
  props.terminal?.focus()
  // Emit action to parent Terminal.vue to handle
  // Use a custom event for actions
  const event = new CustomEvent('toolbar-action', { detail: type })
  window.dispatchEvent(event)
}
</script>

<style scoped>
.mobile-toolbar {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  background: rgba(22, 27, 34, 0.95);
  backdrop-filter: blur(8px);
  border-top: 1px solid var(--border);
  padding: 6px 8px;
  z-index: 100;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.toolbar-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}

.toolbar-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 36px;
  height: 36px;
  padding: 0 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: var(--fs-sm);
  font-family: var(--font-mono);
  cursor: pointer;
  touch-action: manipulation;
  user-select: none;
  -webkit-user-select: none;
  transition: background var(--transition-fast);
}

.toolbar-btn:active {
  background: var(--accent);
  color: #000;
  border-color: var(--accent);
}

.toolbar-btn.ctrl-key {
  font-weight: 600;
  color: var(--accent);
}

.toolbar-sep {
  width: 1px;
  height: 24px;
  background: var(--border);
  margin: 0 2px;
}

.func-btn {
  gap: 4px;
  padding: 0 10px;
}

.btn-icon {
  font-size: 14px;
}

.btn-label {
  font-size: var(--fs-xs);
}

.more-menu {
  position: absolute;
  bottom: 100%;
  right: 8px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  padding: 4px 0;
  min-width: 140px;
}

.more-menu-item {
  display: block;
  width: 100%;
  padding: 8px 12px;
  font-size: var(--fs-sm);
  color: var(--text-primary);
  background: none;
  border: none;
  text-align: left;
  cursor: pointer;
  white-space: nowrap;
}

.more-menu-item:hover {
  background: var(--bg-hover);
}

.more-menu-item.danger {
  color: var(--danger);
}
</style>
