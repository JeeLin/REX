<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { onClickOutside } from '@vueuse/core'

const { t } = useI18n()

defineProps<{
  selectedCount: number
}>()

const emit = defineEmits<{
  upload: []
  download: []
  newFolder: []
  refresh: []
  rename: []
  delete: []
  permissions: []
  copyPath: []
}>()

const showMore = ref(false)
const moreRef = ref<HTMLElement | null>(null)

onClickOutside(moreRef, () => { showMore.value = false })

function onMore(action: string) {
  showMore.value = false
  emit(action as 'rename')
}
</script>

<template>
  <div class="mobile-files-bar">
    <button class="mfb-btn" @click="emit('upload')">📤<span>{{ t('files.upload') }}</span></button>
    <button class="mfb-btn" :disabled="selectedCount === 0" @click="emit('download')">📥<span>{{ t('files.download') }}</span></button>
    <button class="mfb-btn" @click="emit('newFolder')">📁<span>{{ t('files.new') }}</span></button>
    <button class="mfb-btn" @click="emit('refresh')">🔄<span>{{ t('files.refresh') }}</span></button>
    <div ref="moreRef" class="mfb-more-wrap">
      <button class="mfb-btn mfb-more" @click="showMore = !showMore">⋯</button>
      <div v-if="showMore" class="mfb-menu">
        <div class="mfb-menu-item" :class="{ 'mfb-menu-item--disabled': selectedCount === 0 }" @click="onMore('rename')">{{ t('files.rename') }}</div>
        <div class="mfb-menu-item mfb-menu-item--danger" :class="{ 'mfb-menu-item--disabled': selectedCount === 0 }" @click="onMore('delete')">{{ t('files.delete') }}</div>
        <div class="mfb-menu-item" :class="{ 'mfb-menu-item--disabled': selectedCount !== 1 }" @click="onMore('permissions')">{{ t('files.permissions') }}</div>
        <div class="mfb-menu-item" :class="{ 'mfb-menu-item--disabled': selectedCount !== 1 }" @click="onMore('copyPath')">{{ t('files.copyPath') }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mobile-files-bar {
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
  gap: var(--space-2);
}

@media (max-width: 768px) {
  .mobile-files-bar {
    display: flex;
  }
}

.mfb-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-width: 48px;
  height: 44px;
  padding: 0 var(--space-2);
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 11px;
  cursor: pointer;
  -webkit-tap-highlight-color: transparent;
  touch-action: manipulation;
  gap: 2px;
}

.mfb-btn:active {
  background: rgba(255, 255, 255, 0.2);
}

.mfb-btn:disabled {
  opacity: 0.4;
}

.mfb-btn span {
  font-size: 10px;
  color: var(--text-muted);
}

.mfb-more {
  font-size: var(--text-lg);
  font-weight: 700;
  min-width: 40px;
}

.mfb-more-wrap {
  position: relative;
}

.mfb-menu {
  position: absolute;
  bottom: calc(100% + 4px);
  right: 0;
  min-width: 140px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  padding: var(--space-1) 0;
  z-index: 60;
}

.mfb-menu-item {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  cursor: pointer;
  color: var(--text-primary);
}

.mfb-menu-item:hover {
  background: var(--bg-hover);
}

.mfb-menu-item--danger {
  color: var(--danger);
}

.mfb-menu-item--disabled {
  opacity: 0.4;
  pointer-events: none;
}
</style>
