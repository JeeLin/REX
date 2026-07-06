<template>
  <MobileToolbar :visible="visible">
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
  </MobileToolbar>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import MobileToolbar from '@/components/MobileToolbar.vue'

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
