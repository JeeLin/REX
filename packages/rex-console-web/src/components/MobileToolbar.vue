<template>
  <div v-if="visible" class="mobile-toolbar">
    <slot />
  </div>
</template>

<script setup lang="ts">
defineProps<{
  visible: boolean
}>()
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

.toolbar-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
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
