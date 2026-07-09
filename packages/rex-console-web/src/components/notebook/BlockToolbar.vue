<template>
  <div class="block-toolbar" :class="{ visible: isVisible }">
    <button
      class="toolbar-btn drag-handle"
      :title="t('common.copy')"
      @click.stop
    >
      <svg viewBox="0 0 16 16" fill="currentColor">
        <circle cx="5" cy="3" r="1.2" />
        <circle cx="11" cy="3" r="1.2" />
        <circle cx="5" cy="8" r="1.2" />
        <circle cx="11" cy="8" r="1.2" />
        <circle cx="5" cy="13" r="1.2" />
        <circle cx="11" cy="13" r="1.2" />
      </svg>
    </button>
    <button
      class="toolbar-btn add-btn"
      :title="t('common.create')"
      @click.stop="$emit('add')"
    >
      <svg viewBox="0 0 16 16" fill="currentColor">
        <path d="M8 3a.5.5 0 0 1 .5.5v4h4a.5.5 0 0 1 0 1h-4v4a.5.5 0 0 1-1 0v-4h-4a.5.5 0 0 1 0-1h4v-4A.5.5 0 0 1 8 3z" />
      </svg>
    </button>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  isVisible: boolean
}>()

defineEmits<{
  add: []
}>()

const { t } = useI18n()
</script>

<style scoped>
.block-toolbar {
  position: absolute;
  left: -40px;
  top: 0;
  display: flex;
  gap: 2px;
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--transition-fast);
}

.block-toolbar.visible {
  opacity: 1;
  pointer-events: auto;
}

.toolbar-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.toolbar-btn:hover {
  background: var(--bg-hover);
  color: var(--text-secondary);
}

.toolbar-btn svg {
  width: 14px;
  height: 14px;
}
</style>
