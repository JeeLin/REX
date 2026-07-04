<template>
  <div class="redis-key-browser">
    <div class="key-browser-header">
      <input
        v-model="searchPattern"
        class="key-search-input"
        :placeholder="t('redis.keys.searchPlaceholder')"
        @keydown.enter="handleSearch"
      />
      <button class="redis-btn redis-btn-sm" @click="handleSearch">
        {{ t('redis.keys.search') }}
      </button>
    </div>

    <div class="key-list" v-if="treeNodes.length > 0">
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
}>()

const searchPattern = ref('*')
const selectedKey = ref<string | null>(null)
const loading = ref(false)
const collapsedFolders = ref<Set<string>>(new Set())

const contextMenu = reactive({
  visible: false,
  x: 0,
  y: 0,
  key: '',
})

interface TreeNode {
  key: string
  label: string
  isFolder: boolean
  keyType?: string
  children?: TreeNode[]
}

const treeNodes = computed<TreeNode[]>(() => {
  if (props.keys.length === 0) return []

  const typeMap = new Map<string, string>()
  for (const kt of props.keys) {
    typeMap.set(kt.key, kt.type)
  }

  const folders = new Map<string, KeyWithType[]>()
  const leaves: KeyWithType[] = []

  for (const kt of props.keys) {
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

function handleSearch() {
  loading.value = true
  collapsedFolders.value.clear()
  emit('search', searchPattern.value)
  setTimeout(() => { loading.value = false }, 100)
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
    handleSearch()
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

.key-browser-header {
  padding: 8px;
  display: flex;
  gap: 4px;
  border-bottom: 1px solid var(--border);
}

.key-search-input {
  flex: 1;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 4px 8px;
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
}

.key-search-input:focus {
  outline: none;
  border-color: var(--accent);
}

.key-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
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
