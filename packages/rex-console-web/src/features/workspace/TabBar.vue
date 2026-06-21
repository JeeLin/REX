<template>
  <div class="ws-tabbar" @contextmenu.prevent>
    <button class="ws-tab-add" @click="$emit('newConnection')" title="新建连接 (Ctrl+N)">+</button>
    <div
      v-for="tab in tabs"
      :key="tab.id"
      class="ws-tab"
      :class="{ active: tab.id === activeTabId, dragging: dragId === tab.id }"
      :data-tab-id="tab.id"
      draggable="true"
      @click="activateTab(tab.id)"
      @dblclick="onTabDblclick"
      @contextmenu.prevent="onTabCtx($event, tab)"
      @dragstart="onDragStart($event, tab.id)"
      @dragend="onDragEnd"
      @dragover.prevent="onDragOver($event, tab.id)"
      @dragleave="onDragLeave"
      @drop.prevent="onDrop($event, tab.id)"
    >
      <span class="ws-tab-dot" :class="tab.status"></span>
      <span class="ws-tab-icon" :style="{ color: getProtocolIcon(tab.proto).color }">{{ getProtocolIcon(tab.proto).icon }}</span>
      <span class="ws-tab-name">{{ tab.name }}</span>
      <span class="ws-tab-close" @click.stop="closeTab(tab.id)">&times;</span>
    </div>
    <div class="ws-tabbar-right">
      <slot name="right" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { getProtocolIcon } from '@/composables/useProtocol'
import { useContextMenu } from '@/composables/useContextMenu'
import { useI18n } from 'vue-i18n'
import type { Tab } from './useTabs'
import { useTabs } from './useTabs'

const props = defineProps<{
  panelCount?: number
}>()

const { t } = useI18n()
const { show: showMenu } = useContextMenu()
const { tabs, activeTabId, activePanelIndex, activateTab, closeTab, closeOtherTabs, closeTabsRight, closeTabsLeft, closeAllTabs, duplicateTab, moveTabToPanel, disconnectAll, reorderTab } = useTabs()

defineEmits<{
  newConnection: []
}>()

// ── Tab drag-and-drop ──
const dragId = ref<string | null>(null)

function onDragStart(e: DragEvent, id: string) {
  dragId.value = id
  e.dataTransfer!.effectAllowed = 'move'
}

function onDragEnd() {
  dragId.value = null
}

function onDragOver(e: DragEvent, targetId: string) {
  if (!dragId.value || dragId.value === targetId) return
  const el = e.currentTarget as HTMLElement
  const rect = el.getBoundingClientRect()
  const midX = rect.left + rect.width / 2
  el.classList.remove('drag-over-left', 'drag-over-right')
  if (e.clientX < midX) {
    el.classList.add('drag-over-left')
  } else {
    el.classList.add('drag-over-right')
  }
}

function onDragLeave(e: DragEvent) {
  const el = e.currentTarget as HTMLElement
  el.classList.remove('drag-over-left', 'drag-over-right')
}

function onDrop(e: DragEvent, targetId: string) {
  const el = e.currentTarget as HTMLElement
  el.classList.remove('drag-over-left', 'drag-over-right')
  if (!dragId.value || dragId.value === targetId) return
  reorderTab(dragId.value, targetId)
}

function onTabDblclick() {
  // Double-click to enter left-right split — parent handles this
  // For now, just emit if we need it
}

// ── Tab context menu ──
function onTabCtx(e: MouseEvent, tab: Tab) {
  const idx = tabs.value.indexOf(tab)
  const hasMultiple = tabs.value.length > 1
  const hasLeft = idx > 0
  const hasRight = idx < tabs.value.length - 1
  const panelCount = props.panelCount ?? 1
  const isSplit = panelCount > 1

  const panelChildren = isSplit
    ? Array.from({ length: panelCount }, (_, i) => ({
        label: t('ws.layout.panelN', { n: i + 1 }),
        disabled: i === activePanelIndex.value,
        action: () => moveTabToPanel(tab.id, i),
      }))
    : []

  const items = [
    { label: t('ws.tab.close'), icon: '✕', action: () => closeTab(tab.id) },
    ...(hasMultiple ? [{ label: t('ws.tab.closeOthers'), action: () => closeOtherTabs(tab.id) }] : []),
    ...(hasRight ? [{ label: t('ws.tab.closeRight'), action: () => closeTabsRight(tab.id) }] : []),
    ...(hasLeft ? [{ label: t('ws.tab.closeLeft'), action: () => closeTabsLeft(tab.id) }] : []),
    ...(hasMultiple ? [{ label: t('ws.tab.closeAll'), action: () => closeAllTabs() }] : []),
    { separator: true as const },
    { label: t('ws.tab.duplicate'), icon: '⧉', action: () => duplicateTab(tab.id) },
    ...(isSplit ? [{
      label: t('ws.tab.moveToPanel'),
      children: panelChildren,
    }] : []),
    { separator: true as const },
    { label: t('ws.tab.newConnection'), icon: '+', action: () => {} },
    { label: t('ws.tab.disconnectAll'), icon: '⚡', danger: true, action: () => disconnectAll() },
  ]

  showMenu(e, items)
}
</script>

<style scoped>
.ws-tabbar {
  display: flex;
  align-items: center;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 36px;
  padding: 0 var(--sp-sm);
  gap: 2px;
  overflow-x: auto;
  flex-shrink: 0;
}

.ws-tabbar::-webkit-scrollbar {
  display: none;
}

.ws-tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px var(--sp-md);
  border-radius: var(--radius-sm) var(--radius-sm) 0 0;
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  cursor: pointer;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  font-family: var(--font-body);
  white-space: nowrap;
  transition: background var(--transition-fast);
  height: 100%;
  user-select: none;
  position: relative;
}

.ws-tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.ws-tab.active {
  color: var(--text-primary);
  background: var(--bg-deep);
  border-bottom-color: var(--accent);
}

.ws-tab.active::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 20%;
  right: 20%;
  height: 2px;
  background: var(--accent);
  filter: blur(3px);
}

.ws-tab.dragging {
  opacity: 0.5;
  background: var(--bg-hover);
}

.ws-tab.drag-over-left {
  border-left: 2px solid var(--accent);
}

.ws-tab.drag-over-right {
  border-right: 2px solid var(--accent);
}

.ws-tab-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--success);
  box-shadow: 0 0 4px var(--success-glow);
}

.ws-tab-dot.offline {
  background: var(--text-muted);
  box-shadow: none;
}

.ws-tab-dot.connecting {
  background: var(--accent);
  box-shadow: 0 0 4px var(--accent-glow);
  animation: pulse 1.5s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.ws-tab-icon {
  font-size: 12px;
}

.ws-tab-name {
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ws-tab-close {
  width: 16px;
  height: 16px;
  border-radius: 3px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  color: var(--text-muted);
  opacity: 0;
  transition: opacity var(--transition-fast);
  margin-left: 2px;
}

.ws-tab:hover .ws-tab-close {
  opacity: 1;
}

.ws-tab-close:hover {
  background: rgba(248, 81, 73, 0.2);
  color: var(--danger);
}

.ws-tab-add {
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  color: var(--text-muted);
  cursor: pointer;
  background: none;
  border: none;
  flex-shrink: 0;
  margin-left: 2px;
}

.ws-tab-add:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.ws-tabbar-right {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-left: auto;
  flex-shrink: 0;
  padding-left: var(--sp-sm);
}
</style>
