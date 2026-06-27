<template>
  <Teleport to="body">
    <div v-if="visible" class="cmd-overlay" @click="$emit('close')"></div>
    <div v-if="visible" class="cmd-palette" @click.stop>
      <div class="cmd-search">
        <span class="cmd-search-icon">⌕</span>
        <input
          ref="searchInput"
          v-model="query"
          type="text"
          placeholder="搜索资源、页面、操作..."
          @keydown.esc="$emit('close')"
          @keydown.up.prevent="selectPrev"
          @keydown.down.prevent="selectNext"
          @keydown.enter="executeSelected"
        />
      </div>
      <div class="cmd-list">
        <template v-for="group in filteredGroups" :key="group.label">
          <div class="cmd-group-label">{{ group.label }}</div>
          <div
            v-for="(item, ii) in group.items"
            :key="item.id"
            class="cmd-item"
            :class="{ selected: isSelected(group, ii) }"
            @click="$emit('select', item)"
            @mouseenter="setHover(group, ii)"
          >
            <span class="cmd-item-icon" :style="item.color ? { color: item.color } : {}">{{ item.icon }}</span>
            <span class="cmd-item-label">{{ item.label }}</span>
            <span v-if="item.hint" class="cmd-item-hint">{{ item.hint }}</span>
            <span v-if="item.shortcut" class="cmd-item-shortcut">
              <kbd v-for="(k, ki) in item.shortcut.split('+')" :key="ki">{{ k }}</kbd>
            </span>
          </div>
        </template>
        <div v-if="flatItems.length === 0" class="cmd-empty">没有匹配的结果</div>
      </div>
      <div class="cmd-footer">
        <span><kbd>↑↓</kbd> 导航</span>
        <span><kbd>↵</kbd> 选择</span>
        <span><kbd>Esc</kbd> 关闭</span>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'

export interface CommandItem {
  id: string
  label: string
  category: 'resource' | 'navigation' | 'action'
  icon?: string
  hint?: string
  shortcut?: string
  color?: string
}

interface CommandGroup {
  label: string
  items: CommandItem[]
}

const props = defineProps<{ visible: boolean }>()
const emit = defineEmits<{
  close: []
  select: [item: CommandItem]
}>()

const query = ref('')
const searchInput = ref<HTMLInputElement | null>(null)
const hoverIdx = ref(0)

// External command list — parent provides this via slots or expose
const commands = ref<CommandItem[]>([])
defineExpose({ setCommands: (items: CommandItem[]) => { commands.value = items } })

const filteredItems = computed<CommandItem[]>(() => {
  const q = query.value.toLowerCase().trim()
  if (!q) return commands.value
  return commands.value.filter(item =>
    item.label.toLowerCase().includes(q) ||
    item.hint?.toLowerCase().includes(q) ||
    item.category.includes(q)
  )
})

const filteredGroups = computed<CommandGroup[]>(() => {
  const groups: Record<string, CommandItem[]> = {}
  for (const item of filteredItems.value) {
    const label = CATEGORY_LABELS[item.category] ?? item.category
    if (!groups[label]) groups[label] = []
    groups[label].push(item)
  }
  return Object.entries(groups).map(([label, items]) => ({ label, items }))
})

const flatItems = computed(() => filteredGroups.value.flatMap(g => g.items))

// Compute flat index for a (group, localIndex) pair
function flatIndex(group: CommandGroup, localIndex: number): number {
  let idx = 0
  for (const g of filteredGroups.value) {
    if (g === group) return idx + localIndex
    idx += g.items.length
  }
  return 0
}

function isSelected(group: CommandGroup, localIndex: number): boolean {
  return flatIndex(group, localIndex) === hoverIdx.value
}

function setHover(group: CommandGroup, localIndex: number) {
  hoverIdx.value = flatIndex(group, localIndex)
}

function selectNext() {
  const total = flatItems.value.length
  if (total === 0) return
  hoverIdx.value = (hoverIdx.value + 1) % total
}

function selectPrev() {
  const total = flatItems.value.length
  if (total === 0) return
  hoverIdx.value = (hoverIdx.value - 1 + total) % total
}

function executeSelected() {
  const item = flatItems.value[hoverIdx.value]
  if (item) emit('select', item)
}

// Reset state when opened
watch(() => props.visible, (val) => {
  if (val) {
    query.value = ''
    hoverIdx.value = 0
    nextTick(() => searchInput.value?.focus())
  }
})

watch(query, () => { hoverIdx.value = 0 })

const CATEGORY_LABELS: Record<string, string> = {
  resource: '资源',
  navigation: '页面导航',
  action: '操作',
}
</script>

<style>
.cmd-overlay {
  position: fixed;
  inset: 0;
  z-index: 400;
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: blur(4px);
}

.cmd-palette {
  position: fixed;
  top: 100px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 410;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  width: 480px;
  max-width: 90vw;
  max-height: 440px;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-lg), 0 0 24px rgba(232, 145, 45, 0.08);
  animation: cmdIn 0.12s ease;
  overflow: hidden;
}

@keyframes cmdIn {
  from { opacity: 0; transform: translateX(-50%) translateY(-6px); }
  to { opacity: 1; transform: translateX(-50%) translateY(0); }
}

.cmd-search {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-md) var(--sp-lg);
  border-bottom: 1px solid var(--border);
}

.cmd-search input {
  flex: 1;
  border: none;
  background: none;
  outline: none;
  color: var(--text-primary);
  font-size: var(--fs-base);
  font-family: var(--font-body);
}

.cmd-search input::placeholder {
  color: var(--text-muted);
}

.cmd-search-icon {
  color: var(--text-muted);
  font-size: var(--fs-md);
}

.cmd-list {
  flex: 1;
  overflow-y: auto;
  padding: var(--sp-sm) 0;
}

.cmd-group-label {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  padding: var(--sp-sm) var(--sp-lg) var(--sp-xs);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
}

.cmd-item {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
  padding: var(--sp-sm) var(--sp-lg);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.cmd-item:hover,
.cmd-item.selected {
  background: var(--bg-hover);
}

.cmd-item-icon {
  width: 20px;
  text-align: center;
  flex-shrink: 0;
  font-size: var(--fs-sm);
  font-family: var(--font-mono);
}

.cmd-item-label {
  flex: 1;
  font-size: var(--fs-sm);
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cmd-item-hint {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
}

.cmd-item-shortcut {
  display: flex;
  gap: 3px;
  flex-shrink: 0;
}

.cmd-item-shortcut kbd {
  padding: 1px 5px;
  border: 1px solid var(--border);
  border-radius: 3px;
  font-family: var(--font-mono);
  font-size: 10px;
  background: var(--bg-surface);
  color: var(--text-muted);
}

.cmd-empty {
  padding: var(--sp-2xl);
  text-align: center;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.cmd-footer {
  padding: var(--sp-sm) var(--sp-lg);
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: var(--sp-lg);
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.cmd-footer kbd {
  padding: 1px 5px;
  border: 1px solid var(--border);
  border-radius: 3px;
  font-family: var(--font-mono);
  font-size: 10px;
  background: var(--bg-surface);
}
</style>
