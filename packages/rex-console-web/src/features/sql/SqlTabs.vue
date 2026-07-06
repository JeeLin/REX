<template>
  <div class="sql-tabs">
    <div
      v-for="tab in tabs"
      :key="tab.id"
      class="sql-tab"
      :class="{ active: tab.id === activeId }"
      :title="tab.subtitle || tab.title"
      @click="$emit('select', tab.id)"
      @contextmenu.prevent="handleContextMenu($event, tab)"
    >
      <span v-if="!tab.queryId" class="tab-unsaved">·</span>
      <span class="tab-icon">{{ tab.queryId ? '💾' : '📄' }}</span>
      <span class="tab-label">
        <span class="tab-title">{{ tab.title }}</span>
        <span v-if="tab.subtitle" class="tab-subtitle">{{ tab.subtitle }}</span>
      </span>
      <span
        v-if="tabs.length > 1"
        class="tab-close"
        @click.stop="$emit('close', tab.id)"
      >×</span>
    </div>
    <button class="sql-tab-add" @click="$emit('add')">+</button>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useContextMenu } from '@/composables/useContextMenu'

defineProps<{
  tabs: Array<{ id: string; title: string; queryId: string | null; subtitle?: string }>
  activeId: string
}>()

const emit = defineEmits<{
  select: [id: string]
  close: [id: string]
  closeOthers: [id: string]
  closeAll: []
  closeSaved: []
  save: [id: string]
  saveAs: [id: string]
  rename: [id: string]
  copySql: [id: string]
  executeSql: [id: string]
  add: []
}>()

const { t } = useI18n()
const ctxMenu = useContextMenu()

function handleContextMenu(event: MouseEvent, tab: { id: string; title: string; queryId: string | null }) {
  const items = [
    { label: t('sql.tab.ctx.close'), action: () => emit('close', tab.id) },
    { label: t('sql.tab.ctx.closeOthers'), action: () => emit('closeOthers', tab.id) },
    { label: t('sql.tab.ctx.closeAll'), action: () => emit('closeAll') },
    { label: t('sql.tab.ctx.closeSaved'), action: () => emit('closeSaved') },
    { separator: true },
    { label: t('sql.tab.ctx.save'), action: () => emit('save', tab.id) },
    ...(tab.queryId ? [{ label: t('sql.tab.ctx.saveAs'), action: () => emit('saveAs', tab.id) }] : []),
    { label: t('sql.tab.ctx.rename'), action: () => emit('rename', tab.id) },
    { separator: true },
    { label: t('sql.tab.ctx.copySql'), action: () => emit('copySql', tab.id) },
    { label: t('sql.tab.ctx.executeSql'), action: () => emit('executeSql', tab.id) },
  ]
  ctxMenu.show(event, items)
}
</script>

<style scoped>
.sql-tabs {
  display: flex;
  align-items: center;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  padding: 0 var(--sp-md);
  height: 32px;
  flex-shrink: 0;
  gap: 2px;
  overflow-x: auto;
}

.sql-tabs::-webkit-scrollbar { display: none; }

.sql-tab {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-xs) var(--sp-sm);
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  white-space: nowrap;
  background: none;
  border-top: none;
  border-left: none;
  border-right: none;
  font-family: var(--font-body);
  border-radius: var(--radius-sm) var(--radius-sm) 0 0;
  transition: all var(--transition-fast);
}

.sql-tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.sql-tab.active {
  color: var(--text-primary);
  border-bottom-color: var(--accent);
  background: var(--bg-deep);
}

.tab-icon {
  font-size: 10px;
}

.tab-unsaved {
  color: var(--accent);
  font-size: 16px;
  line-height: 1;
  margin-right: -2px;
}

.tab-label {
  display: flex;
  flex-direction: column;
  gap: 0;
  min-width: 0;
}

.tab-title {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-subtitle {
  font-size: 10px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 120px;
  line-height: 1.2;
}

.tab-close {
  font-size: 10px;
  color: var(--text-muted);
  opacity: 0;
  cursor: pointer;
  width: 14px;
  height: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 3px;
}

.sql-tab:hover .tab-close {
  opacity: 1;
}

.tab-close:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.sql-tab-add {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  color: var(--text-muted);
  cursor: pointer;
  background: none;
  border: none;
  border-radius: var(--radius-sm);
}

.sql-tab-add:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
</style>
