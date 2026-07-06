<template>
  <div v-if="visible" class="mobile-toolbar">
    <!-- 操作按钮区 -->
    <div class="toolbar-row action-keys">
      <button class="toolbar-btn func-btn btn-run" @click="$emit('execute')">
        <span class="btn-icon">▶</span>
        <span class="btn-label">{{ t('sql.mobile.execute') }}</span>
      </button>
      <button class="toolbar-btn func-btn" @click="$emit('format')">
        <span class="btn-icon">✨</span>
        <span class="btn-label">{{ t('sql.mobile.format') }}</span>
      </button>
      <button class="toolbar-btn func-btn" @click="$emit('clear')">
        <span class="btn-icon">🗑</span>
        <span class="btn-label">{{ t('sql.mobile.clear') }}</span>
      </button>
    </div>

    <!-- 更多操作区 -->
    <div class="toolbar-row more-keys">
      <button class="toolbar-btn func-btn" @click="$emit('save')">
        <span class="btn-icon">💾</span>
        <span class="btn-label">{{ t('sql.mobile.save') }}</span>
      </button>
      <button class="toolbar-btn func-btn" @click="$emit('history')">
        <span class="btn-icon">📜</span>
        <span class="btn-label">{{ t('sql.mobile.history') }}</span>
      </button>
      <button class="toolbar-btn func-btn" @click="$emit('globalQuery')">
        <span class="btn-icon">⊞</span>
        <span class="btn-label">{{ t('sql.mobile.globalQuery') }}</span>
      </button>
      <button class="toolbar-btn func-btn" @click="showMoreMenu = !showMoreMenu">
        <span class="btn-icon">⚙</span>
        <span class="btn-label">{{ t('sql.mobile.more') }}</span>
      </button>
    </div>

    <!-- 更多选项菜单 -->
    <div v-if="showMoreMenu" class="more-menu">
      <button class="more-menu-item" @click="handleMoreAction('openQuery')">
        📂 {{ t('sql.mobile.openQuery') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

defineProps<{
  visible: boolean
}>()

defineEmits<{
  execute: []
  format: []
  clear: []
  save: []
  history: []
  globalQuery: []
  openQuery: []
}>()

const { t } = useI18n()
const showMoreMenu = ref(false)

function handleMoreAction(type: string) {
  showMoreMenu.value = false
  window.dispatchEvent(new CustomEvent('sql-toolbar-action', { detail: type }))
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

.toolbar-btn.btn-run {
  background: var(--success) !important;
  border-color: var(--success) !important;
  color: #000 !important;
  font-weight: 600;
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
</style>
