<template>
  <div class="ws-sql">
    <!-- Top Bar -->
    <div class="ws-sql-topbar">
      <div class="ws-sql-db-select">
        <select v-model="selectedDb" class="ws-sql-db-dropdown">
          <option value="" disabled>选择数据库</option>
          <option v-for="db in databases" :key="db.name" :value="db.name">{{ db.name }}</option>
        </select>
        <button class="btn btn-ghost btn-xs" @click="loadDatabases">↻</button>
      </div>
      <div class="ws-sql-topbar-spacer"></div>
      <span class="ws-sql-topbar-label">{{ resourceName }}</span>
    </div>

    <!-- Tabs -->
    <SqlTabs
      :tabs="tabList"
      :active-id="activeTabId"
      @select="activeTabId = $event"
      @close="closeTab"
      @close-others="closeOthers"
      @save="handleTabSave"
      @rename="handleTabRename"
      @copy-sql="handleTabCopySql"
      @execute-sql="handleTabExecuteSql"
      @add="addTab"
    />

    <!-- Toolbar -->
    <div class="ws-sql-toolbar">
      <button class="btn btn-run btn-xs" @click="execute(activeTab.sql)">▶ 执行</button>
      <div class="ws-sql-sep"></div>
      <button class="btn btn-ghost btn-xs" @click="clearEditor">清空</button>
      <div class="ws-sql-sep"></div>
      <div class="ws-sql-spacer"></div>
      <span class="ws-sql-hint">Ctrl+Enter 执行</span>
    </div>

    <!-- Main Area -->
    <div class="ws-sql-main">
      <SqlSidebar
        v-if="selectedDb"
        :resource-id="resourceId"
        :database="selectedDb"
        @select-table="insertTableSql"
        @refresh="loadDatabases"
      />
      <div
        v-if="selectedDb" class="ws-sql-resize-handle"
        @mousedown="startResize"
      >
      </div>
      <div class="ws-sql-right">
        <SqlEditor
          v-model="activeTab.sql"
          @execute="execute(activeTab.sql)"
          @execute-selection="execute"
          @save="() => {}"
          @show-history="() => {}"
        />
        <SqlResults
          :result="activeTab.result"
          :loading="executing"
          @sort="handleSort"
          @generate-sql="handleGenerateSql"
        />
      </div>
    </div>

    <!-- Status Bar -->
    <div class="ws-sql-statusbar">
      <span>{{ resourceName }}</span>
      <span class="spacer"></span>
      <span v-if="executing" style="color: #000">执行中...</span>
      <span v-else-if="activeTab.result" style="color: #000">
        {{ activeTab.result.rows?.length ?? 0 }} 行 · {{ activeTab.result.elapsed_ms }}ms
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import SqlTabs from '@/features/sql/SqlTabs.vue'
import SqlSidebar from '@/features/sql/SqlSidebar.vue'
import SqlEditor from '@/features/sql/SqlEditor.vue'
import SqlResults from '@/features/sql/SqlResults.vue'
import { listDatabases } from '@/api/sql'
import type { DatabaseInfo } from '@/api/sql'
import { useSqlTabActions } from '@/features/sql/useSqlTabActions'

const props = defineProps<{
  resourceId: string
  resourceName: string
  protocol: string
}>()

const emit = defineEmits<{
  (e: 'disconnect'): void
  (e: 'error', msg: string): void
}>()

const {
  tabs, activeTabId, executing, tabList, activeTab,
  addTab, closeTab, closeOthers, renameTab, getTabSql,
  clearEditor, execute, handleSort, handleGenerateSql,
} = useSqlTabActions(props.resourceId, (msg) => emit('error', msg))

// Database
const databases = ref<DatabaseInfo[]>([])
const selectedDb = ref('')

function insertTableSql(tableName: string) {
  const tab = tabs.value.find((t) => t.id === activeTabId.value)
  if (tab) tab.sql = `SELECT * FROM ${tableName} LIMIT 100;`
}

function handleTabSave(id: string) {
  // TODO: implement save query file flow
  console.log('save tab', id)
}

function handleTabRename(id: string) {
  const newTitle = prompt('输入新名称:')
  if (newTitle) renameTab(id, newTitle)
}

function handleTabCopySql(id: string) {
  const sql = getTabSql(id)
  if (sql) navigator.clipboard.writeText(sql)
}

function handleTabExecuteSql(id: string) {
  const tab = tabs.value.find((t) => t.id === id)
  if (tab) execute(tab.sql)
}

async function loadDatabases() {
  try {
    databases.value = await listDatabases(props.resourceId)
    if (databases.value.length > 0 && !selectedDb.value) {
      selectedDb.value = databases.value[0].name
    }
  } catch {
    databases.value = []
  }
}

// Sidebar resize
let startX = 0
let startWidth = 0

function startResize(e: MouseEvent) {
  startX = e.clientX
  const sidebar = (e.target as HTMLElement).previousElementSibling as HTMLElement
  if (sidebar) startWidth = sidebar.getBoundingClientRect().width
  document.addEventListener('mousemove', onResize)
  document.addEventListener('mouseup', stopResize)
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
}

function onResize(e: MouseEvent) {
  const delta = e.clientX - startX
  const newWidth = Math.max(120, Math.min(400, startWidth + delta))
  const sidebar = document.querySelector('.ws-sql-main .ws-sql-resize-handle')?.previousElementSibling as HTMLElement
  if (sidebar) sidebar.style.width = `${newWidth}px`
}

function stopResize() {
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}

onMounted(async () => {
  addTab()
  await loadDatabases()
})

onBeforeUnmount(() => {
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
})
</script>

<style scoped>
.ws-sql {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-deep);
}

.ws-sql-topbar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-sm);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 32px;
  flex-shrink: 0;
  gap: var(--sp-sm);
}

.ws-sql-db-select {
  display: flex;
  align-items: center;
  gap: 4px;
}

.ws-sql-db-dropdown {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  padding: 2px 6px;
  outline: none;
}

.ws-sql-topbar-spacer { flex: 1; }

.ws-sql-topbar-label {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.ws-sql-toolbar {
  display: flex;
  align-items: center;
  padding: 2px var(--sp-sm);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 28px;
  flex-shrink: 0;
  gap: var(--sp-xs);
}

.btn-run {
  background: var(--success) !important;
  border-color: var(--success) !important;
  color: #000 !important;
  font-weight: 600;
}

.ws-sql-sep {
  width: 1px;
  height: 16px;
  background: var(--border);
}

.ws-sql-spacer { flex: 1; }

.ws-sql-hint {
  font-size: 11px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.ws-sql-main {
  flex: 1;
  display: flex;
  overflow: hidden;
  min-height: 0;
}

.ws-sql-main :deep(.sql-sidebar) {
  width: 200px;
  flex-shrink: 0;
  overflow-y: auto;
  border-right: 1px solid var(--border);
}

.ws-sql-resize-handle {
  width: 4px;
  cursor: col-resize;
  background: transparent;
  flex-shrink: 0;
  transition: background 0.15s;
}

.ws-sql-resize-handle:hover {
  background: var(--accent);
}

.ws-sql-right {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.ws-sql-statusbar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-sm);
  background: var(--accent);
  color: #000;
  height: 22px;
  flex-shrink: 0;
  font-size: 11px;
  font-family: var(--font-mono);
  font-weight: 500;
  gap: var(--sp-sm);
}

.ws-sql-statusbar .spacer { flex: 1; }
</style>
