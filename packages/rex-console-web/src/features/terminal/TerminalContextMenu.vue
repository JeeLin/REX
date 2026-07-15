<script setup lang="ts">
import { ref, onBeforeUnmount, watch } from 'vue'
import type { Terminal } from '@xterm/xterm'

const props = defineProps<{
  visible: boolean
  x: number
  y: number
  terminal: Terminal | null
}>()

const emit = defineEmits<{
  close: []
  find: []
  reconnect: []
  disconnect: []
}>()

const showEncoding = ref(false)

function copySelection() {
  if (!props.terminal) return
  const selection = props.terminal.getSelection()
  if (selection) {
    navigator.clipboard.writeText(selection)
  }
  emit('close')
}

async function pasteClipboard() {
  if (!props.terminal) return
  try {
    const text = await navigator.clipboard.readText()
    // Bracketed paste: wrapping in ESC[200~ ... ESC[201~
    const bracketed = `\x1b[200~${text}\x1b[201~`
    props.terminal.paste(bracketed)
  } catch {
    // 降级：直接 paste
    props.terminal.paste('')
  }
  emit('close')
}

function selectAll() {
  props.terminal?.selectAll()
  emit('close')
}

function clearScreen() {
  props.terminal?.clear()
  emit('close')
}

function openFind() {
  emit('find')
  emit('close')
}

function setEncoding(_encoding: string) {
  // 编码切换逻辑（后续里程碑实现转换层）
  emit('close')
}

function handleReconnect() {
  emit('reconnect')
  emit('close')
}

function handleDisconnect() {
  emit('disconnect')
  emit('close')
}

function onClickOutside(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (!target.closest('.tcm-menu')) {
    emit('close')
  }
}

watch(() => props.visible, (v) => {
  if (v) {
    showEncoding.value = false
    document.addEventListener('click', onClickOutside)
    document.addEventListener('contextmenu', onClickOutside)
  } else {
    document.removeEventListener('click', onClickOutside)
    document.removeEventListener('contextmenu', onClickOutside)
  }
})

onBeforeUnmount(() => {
  document.removeEventListener('click', onClickOutside)
  document.removeEventListener('contextmenu', onClickOutside)
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="tcm-menu"
      :style="{ top: y + 'px', left: x + 'px' }"
      @click.stop
      @contextmenu.prevent.stop
    >
      <div class="tcm-item" @click="copySelection">
        <span class="tcm-icon">📋</span>
        <span>Copy</span>
        <span class="tcm-shortcut muted">Ctrl+Shift+C</span>
      </div>
      <div class="tcm-item" @click="pasteClipboard">
        <span class="tcm-icon">📄</span>
        <span>Paste</span>
        <span class="tcm-shortcut muted">Ctrl+Shift+V</span>
      </div>
      <div class="tcm-item" @click="selectAll">
        <span class="tcm-icon">Select All</span>
      </div>
      <div class="tcm-item" @click="clearScreen">
        <span class="tcm-icon">Clear</span>
        <span class="tcm-shortcut muted">Ctrl+L</span>
      </div>
      <div class="tcm-item" @click="openFind">
        <span class="tcm-icon">🔍</span>
        <span>Find</span>
        <span class="tcm-shortcut muted">Ctrl+F</span>
      </div>
      <div class="tcm-separator" />
      <div
        class="tcm-item tcm-item--has-sub"
        @mouseenter="showEncoding = true"
        @mouseleave="showEncoding = false"
      >
        <span class="tcm-icon">Encoding</span>
        <span class="tcm-arrow">▸</span>
        <!-- 编码子菜单 -->
        <div v-if="showEncoding" class="tcm-submenu">
          <div class="tcm-item" @click="setEncoding('UTF-8')">
            <span>UTF-8</span>
            <span class="tcm-check">✓</span>
          </div>
          <div class="tcm-item" @click="setEncoding('GBK')">
            <span>GBK</span>
          </div>
          <div class="tcm-item" @click="setEncoding('ISO-8859-1')">
            <span>ISO-8859-1</span>
          </div>
        </div>
      </div>
      <div class="tcm-separator" />
      <div class="tcm-item" @click="handleReconnect">
        <span class="tcm-icon">🔄</span>
        <span>Reconnect</span>
      </div>
      <div class="tcm-item tcm-item--danger" @click="handleDisconnect">
        <span class="tcm-icon">Disconnect</span>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.tcm-menu {
  position: fixed;
  z-index: 200;
  min-width: 180px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  padding: var(--space-1) 0;
}

.tcm-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-primary);
  cursor: pointer;
  transition: background var(--transition);
  white-space: nowrap;
}

.tcm-item:hover {
  background: var(--bg-hover);
}

.tcm-item--danger {
  color: var(--danger);
}

.tcm-icon {
  width: 20px;
  text-align: center;
  font-size: var(--text-xs);
}

.tcm-shortcut {
  margin-left: auto;
  font-size: var(--text-xs);
  font-family: var(--font-mono);
}

.tcm-separator {
  height: 1px;
  background: var(--border);
  margin: var(--space-1) 0;
}

.tcm-item--has-sub {
  position: relative;
}

.tcm-arrow {
  margin-left: auto;
  font-size: var(--text-xs);
  color: var(--text-muted);
}

.tcm-submenu {
  position: absolute;
  left: 100%;
  top: -1px;
  min-width: 140px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  padding: var(--space-1) 0;
}

.tcm-check {
  margin-left: auto;
  color: var(--accent);
  font-size: var(--text-xs);
}
</style>
