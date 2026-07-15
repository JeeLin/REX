<script setup lang="ts">
import { ref, onMounted } from 'vue'
import SqlNavTree from './SqlNavTree.vue'
import { useSqlNav } from './useSqlNav'

const sessionId = ref<string | null>(null)
const { databases, loading, searchQuery, loadDatabases, toggleDatabase, filteredDatabases } =
  useSqlNav(sessionId)

const navWidth = ref(260)
const isResizing = ref(false)

function startResize(e: MouseEvent) {
  isResizing.value = true
  const startX = e.clientX
  const startWidth = navWidth.value

  function onMove(ev: MouseEvent) {
    const delta = ev.clientX - startX
    navWidth.value = Math.min(400, Math.max(200, startWidth + delta))
  }

  function onUp() {
    isResizing.value = false
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('mouseup', onUp)
  }

  document.addEventListener('mousemove', onMove)
  document.addEventListener('mouseup', onUp)
}

function onSelectTable(_db: string, _table: string) {
  // TODO: subtask 3 — open query tab
}

onMounted(() => {
  // TODO: get sessionId from connection flow
  // For now, use a mock for UI development
})
</script>

<template>
  <div class="sql-page" :class="{ 'sql-page--resizing': isResizing }">
    <!-- Left: Navigation tree -->
    <div class="sql-nav-panel" :style="{ width: navWidth + 'px' }">
      <SqlNavTree
        :databases="filteredDatabases()"
        :loading="loading"
        :search-query="searchQuery"
        @update:search-query="searchQuery = $event"
        @select-table="onSelectTable"
        @refresh="loadDatabases"
      />
    </div>

    <!-- Resize handle -->
    <div class="sql-resize-handle" @mousedown="startResize" />

    <!-- Right: Content area -->
    <div class="sql-content">
      <div class="sql-content-placeholder">
        <div class="placeholder-icon">📋</div>
        <div class="placeholder-title">SQL Console</div>
        <div class="placeholder-desc">Select a table from the navigation tree to get started</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sql-page {
  display: flex;
  height: 100%;
  overflow: hidden;
  background: var(--bg-deep);
}

.sql-page--resizing {
  cursor: col-resize;
  user-select: none;
}

.sql-nav-panel {
  flex-shrink: 0;
  height: 100%;
  overflow: hidden;
}

.sql-resize-handle {
  width: 4px;
  cursor: col-resize;
  background: transparent;
  transition: background var(--transition);
  flex-shrink: 0;
}

.sql-resize-handle:hover {
  background: var(--accent);
}

.sql-content {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.sql-content-placeholder {
  text-align: center;
  color: var(--text-muted);
}

.placeholder-icon {
  font-size: 48px;
  margin-bottom: var(--space-4);
  opacity: 0.5;
}

.placeholder-title {
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: var(--space-2);
}

.placeholder-desc {
  font-size: var(--text-sm);
  max-width: 300px;
}
</style>
