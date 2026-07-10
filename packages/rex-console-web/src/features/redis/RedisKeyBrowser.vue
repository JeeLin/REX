<template>
  <div class="redis-key-browser">
    <SearchFilter
      ref="searchFilterRef"
      @search="handleSearch"
      @filter="handleFilter"
    />

    <!-- Batch Action Toolbar -->
    <div v-if="selectedKeys.size > 0" class="batch-toolbar">
      <span class="batch-count">{{ selectedKeys.size }} {{ t('redis.keys.batch.selected') }}</span>
      <button class="batch-btn batch-btn-delete" @click="$emit('batchDelete', [...selectedKeys])">
        {{ t('redis.keys.batch.delete') }}
      </button>
      <button class="batch-btn batch-btn-ttl" @click="$emit('batchSetTtl', [...selectedKeys])">
        {{ t('redis.keys.batch.setTtl') }}
      </button>
      <button class="batch-btn batch-btn-export" @click="$emit('batchExport', [...selectedKeys])">
        {{ t('redis.keys.batch.export') }}
      </button>
      <button class="batch-btn batch-btn-clear" @click="clearSelection">
        {{ t('redis.keys.batch.clearSelection') }}
      </button>
    </div>

    <div v-if="treeNodes.length > 0" class="key-list">
      <!-- Select all in header -->
      <div class="key-list-select-all" v-if="allVisibleKeys.length > 0">
        <label class="key-checkbox-label">
          <input
            type="checkbox"
            class="key-checkbox"
            :checked="allSelected"
            :indeterminate="someSelected"
            @change="toggleSelectAll"
          />
          <span class="key-select-all-text">{{ t('redis.keys.batch.selectAll') }}</span>
        </label>
      </div>
      <template v-for="node in treeNodes" :key="node.key">
        <!-- Folder node -->
        <div
          v-if="node.isFolder"
          class="key-folder"
          @click="toggleFolder(node.key)"
        >
          <span class="key-folder-arrow" :class="{ collapsed: collapsedFolders.has(node.key) }">▶</span>
          <span class="key-folder-icon">📁</span>
          <span class="key-folder-name">{{ node.label }}</span>
          <span class="key-folder-count">{{ node.children!.length }}</span>
        </div>
        <!-- Children of folder (when expanded) -->
        <div
          v-if="node.isFolder && !collapsedFolders.has(node.key)"
          class="key-folder-children"
        >
          <div
            v-for="child in node.children"
            :key="child.key"
            class="key-item"
            :class="{ selected: selectedKey === child.key }"
            @click.stop="$emit('selectKey', child.key)"
            @contextmenu.prevent="openContextMenu($event, child.key)"
          >
            <label class="key-checkbox-label" @click.stop>
              <input
                type="checkbox"
                class="key-checkbox"
                :checked="selectedKeys.has(child.key)"
                @change="toggleSelectKey(child.key)"
              />
            </label>
            <span class="key-type-icon" :class="child.keyType">{{ getTypeIcon(child.keyType) }}</span>
            <span class="key-name">{{ child.label }}</span>
          </div>
        </div>
        <!-- Leaf node (no separator) -->
        <div
          v-else-if="!node.isFolder"
          class="key-item"
          :class="{ selected: selectedKey === node.key }"
          @click="$emit('selectKey', node.key)"
          @contextmenu.prevent="openContextMenu($event, node.key)"
        >
          <label class="key-checkbox-label" @click.stop>
            <input
              type="checkbox"
              class="key-checkbox"
              :checked="selectedKeys.has(node.key)"
              @change="toggleSelectKey(node.key)"
            />
          </label>
          <span class="key-type-icon" :class="node.keyType">{{ getTypeIcon(node.keyType) }}</span>
          <span class="key-name">{{ node.label }}</span>
        </div>
      </template>
    </div>

    <div v-else-if="!loading" class="key-list-empty">
      {{ t('redis.keys.empty') }}
    </div>

    <div v-if="loading" class="key-list-loading">
      {{ t('redis.keys.loading') }}
    </div>

    <!-- Context Menu -->
    <div
      v-if="contextMenu.visible"
      class="context-menu-overlay"
      @click="closeContextMenu"
      @contextmenu.prevent="closeContextMenu"
    >
      <div
        class="context-menu"
        :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }"
        @click.stop
      >
        <div class="context-menu-item" @click="handleCopyKey">
          {{ t('redis.keys.context.copyKey') }}
        </div>
        <div class="context-menu-item" @click="handleViewValue">
          {{ t('redis.keys.context.viewValue') }}
        </div>
        <div class="context-menu-separator" />
        <div class="context-menu-item danger" @click="handleDeleteKey">
          {{ t('redis.keys.context.deleteKey') }}
        </div>
        <div class="context-menu-item" @click="handleSetTtl">
          {{ t('redis.keys.context.setTtl') }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import type { KeyWithType } from './types'
import SearchFilter from './SearchFilter.vue'
import type { FilterCriteria } from './SearchFilter.vue'

const { t } = useI18n()

const props = defineProps<{
  connected: boolean
  keys: KeyWithType[]
}>()

const emit = defineEmits<{
  (e: 'selectKey', key: string): void
  (e: 'search', pattern: string): void
  (e: 'deleteKey', key: string): void
  (e: 'setTtl', key: string, seconds: number): void
  (e: 'batchDelete', keys: string[]): void
  (e: 'batchSetTtl', keys: string[]): void
  (e: 'batchExport', keys: string[]): void
}>()

const searchFilterRef = ref<InstanceType<typeof SearchFilter> | null>(null)
const selectedKey = ref<string | null>(null)
const loading = ref(false)
const collapsedFolders = ref<Set<string>>(new Set())
const selectedKeys = ref<Set<string>>(new Set())
const activeFilter = ref<FilterCriteria>({ type: '', ttlMin: null, ttlMax: null })

const contextMenu = reactive({
  visible: false,
  x: 0,
  y: 0,
  key: '',
})

/** Keys filtered by type and TTL criteria (client-side) */
const filteredKeys = computed<KeyWithType[]>(() => {
  const criteria = activeFilter.value
  return props.keys.filter((k) => {
    if (criteria.type && k.type !== criteria.type) return false
    if (criteria.ttlMin !== null && (k.ttl ?? -2) < criteria.ttlMin) return false
    if (criteria.ttlMax !== null && (k.ttl ?? -2) > criteria.ttlMax) return false
    return true
  })
})

interface TreeNode {
  key: string
  label: string
  isFolder: boolean
  keyType?: string
  children?: TreeNode[]
}

const treeNodes = computed<TreeNode[]>(() => {
  if (filteredKeys.value.length === 0) return []

  const typeMap = new Map<string, string>()
  for (const kt of filteredKeys.value) {
    typeMap.set(kt.key, kt.type)
  }

  const folders = new Map<string, KeyWithType[]>()
  const leaves: KeyWithType[] = []

  for (const kt of filteredKeys.value) {
    const sepIndex = kt.key.indexOf(':')
    if (sepIndex > 0) {
      const folder = kt.key.substring(0, sepIndex)
      if (!folders.has(folder)) folders.set(folder, [])
      folders.get(folder)!.push(kt)
    } else {
      leaves.push(kt)
    }
  }

  const nodes: TreeNode[] = []

  for (const [folder, children] of [...folders.entries()].sort((a, b) => a[0].localeCompare(b[0]))) {
    nodes.push({
      key: folder,
      label: folder,
      isFolder: true,
      children: children.map(c => ({
        key: c.key,
        label: c.key.substring(folder.length + 1),
        isFolder: false,
        keyType: typeMap.get(c.key) ?? 'unknown',
      })),
    })
  }

  for (const leaf of leaves.sort((a, b) => a.key.localeCompare(b.key))) {
    nodes.push({
      key: leaf.key,
      label: leaf.key,
      isFolder: false,
      keyType: leaf.type,
    })
  }

  return nodes
})

/** All visible leaf keys across folders and top-level */
const allVisibleKeys = computed(() => {
  const keys: string[] = []
  for (const node of treeNodes.value) {
    if (node.isFolder) {
      for (const child of node.children ?? []) {
        keys.push(child.key)
      }
    } else {
      keys.push(node.key)
    }
  }
  return keys
})

const allSelected = computed(() =>
  allVisibleKeys.value.length > 0 && allVisibleKeys.value.every(k => selectedKeys.value.has(k)),
)

const someSelected = computed(() =>
  !allSelected.value && allVisibleKeys.value.some(k => selectedKeys.value.has(k)),
)

function toggleSelectKey(key: string) {
  const s = new Set(selectedKeys.value)
  if (s.has(key)) s.delete(key)
  else s.add(key)
  selectedKeys.value = s
}

function toggleSelectAll() {
  if (allSelected.value) {
    selectedKeys.value = new Set()
  } else {
    selectedKeys.value = new Set(allVisibleKeys.value)
  }
}

function clearSelection() {
  selectedKeys.value = new Set()
}

function getTypeIcon(type: string | undefined): string {
  switch (type) {
    case 'string': return 'Aa'
    case 'hash': return '{}'
    case 'list': return '[]'
    case 'set': return '(~)'
    case 'zset': return '< >'
    default: return '❓'
  }
}

function toggleFolder(folder: string) {
  if (collapsedFolders.value.has(folder)) {
    collapsedFolders.value.delete(folder)
  } else {
    collapsedFolders.value.add(folder)
  }
}

function handleSearch(pattern: string) {
  loading.value = true
  collapsedFolders.value.clear()
  emit('search', pattern || '*')
  setTimeout(() => { loading.value = false }, 100)
}

function handleFilter(criteria: FilterCriteria) {
  activeFilter.value = { ...criteria }
}

function openContextMenu(event: MouseEvent, key: string) {
  contextMenu.x = event.clientX
  contextMenu.y = event.clientY
  contextMenu.key = key
  contextMenu.visible = true
}

function closeContextMenu() {
  contextMenu.visible = false
}

function handleCopyKey() {
  navigator.clipboard.writeText(contextMenu.key)
  closeContextMenu()
}

function handleViewValue() {
  emit('selectKey', contextMenu.key)
  closeContextMenu()
}

function handleDeleteKey() {
  if (window.confirm(t('redis.keys.context.deleteConfirm', { key: contextMenu.key }))) {
    emit('deleteKey', contextMenu.key)
  }
  closeContextMenu()
}

function handleSetTtl() {
  const input = window.prompt(t('redis.keys.context.ttlPrompt'), '3600')
  if (input !== null) {
    const seconds = parseInt(input, 10)
    if (!isNaN(seconds) && seconds > 0) {
      emit('setTtl', contextMenu.key, seconds)
    }
  }
  closeContextMenu()
}

function handleGlobalClick() {
  closeContextMenu()
}

onMounted(() => {
  if (props.connected) {
    handleSearch('*')
  }
  document.addEventListener('click', handleGlobalClick)
})

onUnmounted(() => {
  document.removeEventListener('click', handleGlobalClick)
})
</script>

<style scoped>
.redis-key-browser {
  width: 240px;
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  background: var(--bg-surface);
  position: relative;
}

/* Batch Toolbar */
.batch-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 8px;
  background: var(--accent-muted);
  border-bottom: 1px solid var(--border);
}

.batch-count {
  font-size: 11px;
  color: var(--accent);
  font-weight: 600;
  margin-right: auto;
}

.batch-btn {
  padding: 2px 6px;
  font-size: 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-primary);
  cursor: pointer;
  white-space: nowrap;
}

.batch-btn:hover {
  background: var(--bg-elevated);
}

.batch-btn-delete {
  color: #f85149;
  border-color: #f8514933;
}

.batch-btn-delete:hover {
  background: #f8514922;
}

.batch-btn-clear {
  color: var(--text-muted);
}

.key-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
}

/* Select All */
.key-list-select-all {
  padding: 4px 8px;
  border-bottom: 1px solid var(--border);
  margin-bottom: 2px;
}

/* Checkbox */
.key-checkbox-label {
  display: flex;
  align-items: center;
  cursor: pointer;
}

.key-checkbox {
  width: 14px;
  height: 14px;
  accent-color: var(--accent);
  cursor: pointer;
  margin: 0;
}

.key-select-all-text {
  font-size: 10px;
  color: var(--text-muted);
  margin-left: 4px;
}

.key-folder {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--fs-xs);
  color: var(--text-primary);
  font-weight: 600;
}

.key-folder:hover {
  background: var(--bg-elevated);
}

.key-folder-arrow {
  font-size: 8px;
  transition: transform 0.15s ease;
  color: var(--text-muted);
}

.key-folder-arrow.collapsed {
  transform: rotate(0deg);
}

.key-folder-arrow:not(.collapsed) {
  transform: rotate(90deg);
}

.key-folder-icon {
  font-size: 12px;
  flex-shrink: 0;
}

.key-folder-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
}

.key-folder-count {
  margin-left: auto;
  color: var(--text-muted);
  font-size: 10px;
  font-weight: 400;
}

.key-folder-children {
  padding-left: 12px;
}

.key-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--fs-xs);
  color: var(--text-primary);
}

.key-item:hover {
  background: var(--bg-elevated);
}

.key-item.selected {
  background: var(--accent-muted);
  color: var(--accent);
}

.key-type-icon {
  font-size: 10px;
  font-weight: 700;
  flex-shrink: 0;
  font-family: var(--font-mono);
  width: 22px;
  text-align: center;
}

.key-type-icon.string { color: #3fb950; }
.key-type-icon.hash { color: #f0883e; }
.key-type-icon.list { color: #58a6ff; }
.key-type-icon.set { color: #bc8cff; }
.key-type-icon.zset { color: #f778ba; }

.key-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
}

.key-list-empty,
.key-list-loading {
  padding: 16px;
  text-align: center;
  color: var(--text-muted);
  font-size: var(--fs-xs);
}

/* Context Menu */
.context-menu-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
}

.context-menu {
  position: fixed;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px 0;
  min-width: 160px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  z-index: 1001;
}

.context-menu-item {
  padding: 6px 12px;
  font-size: 12px;
  color: var(--text-primary);
  cursor: pointer;
}

.context-menu-item:hover {
  background: var(--bg-hover);
}

.context-menu-item.danger {
  color: #f85149;
}

.context-menu-separator {
  height: 1px;
  background: var(--border);
  margin: 4px 0;
}
</style>
