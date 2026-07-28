<script setup lang="ts">
import type { Terminal } from '@xterm/xterm'

defineProps<{
  terminal: Terminal | null
}>()
</script>
<template>
  <div class="mobile-terminal-bar">

    <!-- 方向键区 -->
    <div class="mtb-dpad">
      <button class="mtb-btn mtb-dpad-up" @click="terminal?.input('\x1b[A')">↑</button>
      <div class="mtb-dpad-row">
        <button class="mtb-btn mtb-dpad-left" @click="terminal?.input('\x1b[D')">←</button>
        <button class="mtb-btn mtb-dpad-right" @click="terminal?.input('\x1b[C')">→</button>
      </div>
      <button class="mtb-btn mtb-dpad-down" @click="terminal?.input('\x1b[B')">↓</button>
    </div>

    <!-- 快捷键按钮 -->
    <div class="mtb-actions">
      <button class="mtb-btn mtb-action" @click="terminal?.paste('\t')">Tab</button>
      <button class="mtb-btn mtb-action" @click="terminal?.paste('\r')">⏎</button>
      <button class="mtb-btn mtb-action mtb-ctrl" @click="terminal?.paste('\x03')">^C</button>
      <button class="mtb-btn mtb-action mtb-ctrl" @click="terminal?.paste('\x0c')">^L</button>
    </div>
  </div>
</template>

<style scoped>
.mobile-terminal-bar {
  display: none;
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 50;
  padding: var(--space-2);
  background: rgba(13, 17, 23, 0.95);
  backdrop-filter: blur(8px);
  border-top: 1px solid var(--border);
  justify-content: center;
  gap: var(--space-4);
}

@media (max-width: 768px) {
  .mobile-terminal-bar {
    display: flex;
  }
}

.mtb-dpad {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.mtb-dpad-row {
  display: flex;
  gap: 2px;
}

.mtb-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 40px;
  height: 36px;
  padding: 0 var(--space-2);
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  cursor: pointer;
  -webkit-tap-highlight-color: transparent;
  touch-action: manipulation;
}

.mtb-btn:active {
  background: rgba(255, 255, 255, 0.2);
}

.mtb-actions {
  display: flex;
  gap: var(--space-1);
  align-items: center;
}

.mtb-action {
  min-width: 48px;
}

.mtb-ctrl {
  background: rgba(232, 145, 45, 0.2);
  border-color: rgba(232, 145, 45, 0.3);
}
</style>
