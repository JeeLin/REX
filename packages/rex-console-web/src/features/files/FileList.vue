<template>
  <div class="file-list" @contextmenu.prevent="$emit('contextMenu', $event, null)">
    <div class="file-list-header">
      <div class="col col-name">名称</div>
      <div class="col col-size">大小</div>
      <div class="col col-actions"></div>
    </div>
    <div class="file-list-body" ref="listBody">
      <div
        v-if="currentPath !== '/'"
        class="file-row file-row-parent"
        @dblclick="$emit('goUp')"
      >
        <div class="col col-name">
          <span class="file-icon">📁</span>
          <span class="file-name">..</span>
        </div>
        <div class="col col-size"></div>
        <div class="col col-actions"></div>
      </div>
      <div
        v-for="entry in entries"
        :key="entry.path"
        class="file-row"
        :class="{ selected: selectedPaths.includes(entry.path) }"
        @click.stop="$emit('select', entry, $event)"
        @dblclick="entry.file_type === 'directory' ? $emit('open', entry.name) : $emit('preview', entry)"
        @contextmenu.prevent="$emit('contextMenu', $event, entry)"
      >
        <div class="col col-name">
          <span class="file-icon">{{ entry.file_type === 'directory' ? '📁' : getFileIcon(entry.name) }}</span>
          <span class="file-name">{{ entry.name }}</span>
        </div>
        <div class="col col-size">{{ entry.file_type === 'directory' ? '-' : formatSize(entry.size) }}</div>
        <div class="col col-actions">
          <button class="btn btn-ghost btn-xs" @click.stop="$emit('contextMenu', $event, entry)">⋯</button>
        </div>
      </div>
      <div v-if="entries.length === 0 && !loading" class="file-list-empty">
        此目录为空
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { FileEntry } from '@/api/files'

defineProps<{
  entries: FileEntry[]
  currentPath: string
  selectedPaths: string[]
  loading: boolean
}>()

defineEmits<{
  goUp: []
  open: [name: string]
  preview: [entry: FileEntry]
  select: [entry: FileEntry, event: MouseEvent]
  contextMenu: [event: MouseEvent, entry: FileEntry | null]
}>()

function formatSize(bytes: number | null): string {
  if (bytes == null) return '-'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
}

function getFileIcon(name: string): string {
  if (name.endsWith('.zip') || name.endsWith('.tar.gz') || name.endsWith('.gz')) return '📦'
  if (name.endsWith('.png') || name.endsWith('.jpg') || name.endsWith('.gif') || name.endsWith('.svg')) return '🖼'
  return '📄'
}
</script>

<style scoped>
.file-list {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}

.file-list-header {
  display: flex;
  align-items: center;
  padding: var(--sp-sm) var(--sp-md);
  background: var(--bg-elevated);
  border-bottom: 1px solid var(--border);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  flex-shrink: 0;
}

.file-list-body {
  flex: 1;
  overflow-y: auto;
}

.col {
  padding: 0 var(--sp-sm);
}

.col-name {
  flex: 1;
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  min-width: 0;
}

.col-size {
  width: 100px;
  text-align: right;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  flex-shrink: 0;
}

.col-actions {
  width: 40px;
  flex-shrink: 0;
}

.file-row {
  display: flex;
  align-items: center;
  padding: var(--sp-xs) var(--sp-md);
  cursor: pointer;
  transition: background var(--transition-fast);
  border-bottom: 1px solid transparent;
}

.file-row:hover {
  background: var(--bg-hover);
}

.file-row.selected {
  background: var(--accent-muted);
}

.file-row-parent {
  opacity: 0.6;
}

.file-icon {
  width: 20px;
  text-align: center;
  flex-shrink: 0;
}

.file-name {
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-list-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 200px;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.btn-xs {
  height: 24px;
  font-size: var(--fs-xs);
  padding: 0 var(--sp-sm);
}
</style>
