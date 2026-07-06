<template>
  <div v-if="visible" class="mobile-toolbar">
    <!-- 操作按钮区 -->
    <div class="toolbar-row action-keys">
      <button class="toolbar-btn func-btn" @click="$emit('upload')">
        <span class="btn-icon">⬆</span>
        <span class="btn-label">{{ t('files.upload') }}</span>
      </button>
      <button class="toolbar-btn func-btn" @click="$emit('newFile')">
        <span class="btn-icon">📄</span>
        <span class="btn-label">{{ t('files.newFile') }}</span>
      </button>
      <button class="toolbar-btn func-btn" @click="$emit('newFolder')">
        <span class="btn-icon">📁</span>
        <span class="btn-label">{{ t('files.newFolder') }}</span>
      </button>
      <button class="toolbar-btn func-btn" @click="$emit('refresh')">
        <span class="btn-icon">↻</span>
        <span class="btn-label">{{ t('files.refresh') }}</span>
      </button>
    </div>

    <!-- 更多操作区 -->
    <div class="toolbar-row more-keys">
      <button
        class="toolbar-btn func-btn"
        :disabled="selectedCount === 0"
        @click="$emit('download')"
      >
        <span class="btn-icon">⬇</span>
        <span class="btn-label">{{ t('files.download') }}</span>
      </button>
      <button
        class="toolbar-btn func-btn"
        :disabled="selectedCount === 0"
        @click="$emit('delete')"
      >
        <span class="btn-icon">🗑</span>
        <span class="btn-label">{{ t('files.delete') }}</span>
      </button>
      <button class="toolbar-btn func-btn" @click="$emit('selectAll')">
        <span class="btn-icon">☑</span>
        <span class="btn-label">{{ t('files.selectAll') }}</span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  visible: boolean
  selectedCount: number
}>()

defineEmits<{
  upload: []
  newFile: []
  newFolder: []
  refresh: []
  download: []
  delete: []
  selectAll: []
}>()

const { t } = useI18n()
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
</style>
