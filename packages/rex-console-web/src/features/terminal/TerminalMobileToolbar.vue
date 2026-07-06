<template>
  <MobileToolbar :visible="visible">
    <!-- 方向键区 -->
    <div class="toolbar-row direction-keys">
      <button class="toolbar-btn" @click="sendKey('\x1b[A')">↑</button>
      <button class="toolbar-btn" @click="sendKey('\x1b[B')">↓</button>
      <button class="toolbar-btn" @click="sendKey('\x1b[D')">←</button>
      <button class="toolbar-btn" @click="sendKey('\x1b[C')">→</button>
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
        <span class="btn-label">{{ t('ws.terminal.mobile.history') }}</span>
      </button>
      <button class="toolbar-btn func-btn" @click="$emit('openPaste')">
        <span class="btn-icon">📋</span>
        <span class="btn-label">{{ t('ws.terminal.mobile.paste') }}</span>
      </button>
      <div class="toolbar-sep"></div>
      <button class="toolbar-btn func-btn" @click="$emit('fontSizeChange', -1)">A-</button>
      <button class="toolbar-btn func-btn" @click="$emit('fontSizeChange', 1)">A+</button>
      <div class="toolbar-sep"></div>
      <button class="toolbar-btn func-btn" @click="showMoreMenu = !showMoreMenu">
        <span class="btn-icon">⚙</span>
        <span class="btn-label">{{ t('ws.terminal.mobile.more') }}</span>
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
  </MobileToolbar>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Terminal } from '@xterm/xterm'
import MobileToolbar from '@/components/MobileToolbar.vue'

const props = defineProps<{
  terminal: Terminal | null
  visible: boolean
}>()

defineEmits<{
  openHistory: []
  openPaste: []
  fontSizeChange: [delta: number]
}>()

const { t } = useI18n()
const showMoreMenu = ref(false)

function sendKey(data: string) {
  const textarea = props.terminal?.textarea
  if (!textarea) return
  textarea.focus()
  const setter = Object.getOwnPropertyDescriptor(
    window.HTMLTextAreaElement.prototype, 'value'
  )?.set
  if (setter) {
    setter.call(textarea, data)
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
  }
}

function handleMoreAction(type: string) {
  showMoreMenu.value = false
  window.dispatchEvent(new CustomEvent('toolbar-action', { detail: type }))
}
</script>

<style scoped>
.toolbar-btn.ctrl-key {
  font-weight: 600;
  color: var(--accent);
}
</style>
