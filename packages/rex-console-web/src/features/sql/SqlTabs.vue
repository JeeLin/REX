<template>
  <div class="sql-tabs">
    <div
      v-for="tab in tabs"
      :key="tab.id"
      class="sql-tab"
      :class="{ active: tab.id === activeId }"
      @click="$emit('select', tab.id)"
    >
      <span class="tab-icon">📄</span> {{ tab.title }}
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
defineProps<{
  tabs: Array<{ id: string; title: string }>
  activeId: string
}>()

defineEmits<{
  select: [id: string]
  close: [id: string]
  add: []
}>()
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
