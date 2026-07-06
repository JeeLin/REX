<template>
  <MobileToolbar :visible="visible">
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
  </MobileToolbar>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import MobileToolbar from '@/components/MobileToolbar.vue'

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
.btn-run {
  background: var(--success) !important;
  border-color: var(--success) !important;
  color: #000 !important;
  font-weight: 600;
}
</style>
