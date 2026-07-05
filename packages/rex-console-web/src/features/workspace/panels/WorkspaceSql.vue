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
      <button class="btn btn-ghost btn-xs" @click="handleFormat">✨ 格式化</button>
      <div class="ws-sql-sep"></div>
      <button class="btn btn-ghost btn-xs" @click="handleSave">💾 保存</button>
      <div class="ws-sql-sep"></div>
      <button class="btn btn-ghost btn-xs" @click="clearEditor">清空</button>
      <div class="ws-sql-spacer"></div>
      <span class="ws-sql-hint">Ctrl+S 保存 · Ctrl+Enter 执行</span>
    </div>

    <!-- Main Area -->
    <div class="ws-sql-main">
      <SqlSidebar
        v-if="selectedDb"
        :resource-id="resourceId"
        :database="selectedDb"
        :databases="databases"
        :protocol="protocol"
        @update:database="selectedDb = $event"
        @select-table="insertTableSql"
        @refresh="loadDatabases"
        @export-table="handleExportTable"
      />
      <div
        v-if="selectedDb" class="ws-sql-resize-handle"
        @mousedown="startResize"
      >
      </div>
      <div class="ws-sql-right">
        <SqlEditor
          ref="sqlEditorRef"
          v-model="activeTab.sql"
          @execute="execute(activeTab.sql)"
          @execute-selection="execute"
          @save="handleSave"
          @show-history="() => {}"
        />
        <SqlResults
          :result="activeTab.result"
          :loading="executing"
          :resource-id="resourceId"
          :current-sql="activeTab.sql"
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

    <!-- Save Modal -->
    <div v-if="showSaveModal" class="ws-sql-modal-overlay" @click.self="showSaveModal = false">
      <div class="ws-sql-modal">
        <div class="ws-sql-modal-header">
          <span>{{ t('sql.saveQuery') }}</span>
          <button class="btn btn-ghost btn-xs" @click="showSaveModal = false">×</button>
        </div>
        <div class="ws-sql-modal-body">
          <label class="ws-sql-modal-label">{{ t('sql.fileName') }}</label>
          <input
            v-model="saveFileName"
            class="ws-sql-modal-input"
            :placeholder="t('sql.fileNamePlaceholder')"
            @keyup.enter="confirmSave"
          />
        </div>
        <div class="ws-sql-modal-footer">
          <button class="btn btn-ghost btn-xs" @click="showSaveModal = false">{{ t('common.cancel') }}</button>
          <button class="btn btn-primary btn-xs" @click="confirmSave">{{ t('common.save') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import SqlTabs from '@/features/sql/SqlTabs.vue'
import SqlSidebar from '@/features/sql/SqlSidebar.vue'
import SqlEditor from '@/features/sql/SqlEditor.vue'
import SqlResults from '@/features/sql/SqlResults.vue'
import { listDatabases, executeSql, saveQuery } from '@/api/sql'
import type { DatabaseInfo } from '@/api/sql'
import { exportCsv } from '@/features/sql/result-export'
import { useToast } from '@/composables/useToast'
import { useSqlTabActions } from '@/features/sql/useSqlTabActions'

const toast = useToast()
const { t } = useI18n()

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
  clearEditor, execute, handleSort, handleGenerateSql, markSaved,
} = useSqlTabActions(props.resourceId, (msg) => emit('error', msg))

// Database
const databases = ref<DatabaseInfo[]>([])
const selectedDb = ref('')

// Save modal
const showSaveModal = ref(false)
const saveFileName = ref('')

// SqlEditor ref
const sqlEditorRef = ref<InstanceType<typeof SqlEditor>>()

function insertTableSql(tableName: string) {
  const tab = tabs.value.find((t) => t.id === activeTabId.value)
  if (tab) tab.sql = `SELECT * FROM ${tableName} LIMIT 100;`
}

async function handleExportTable(tableName: string) {
  try {
    const result = await executeSql(props.resourceId, `SELECT * FROM \`${tableName}\``)
    exportCsv(result.columns, result.rows)
    toast.success(t('sql.toast.exportSuccess'))
  } catch {
    toast.error(t('sql.toast.exportFailed'))
  }
}

function handleTabSave(_id: string) {
  handleSave()
}

function handleSave() {
  saveFileName.value = activeTab.value.title || 'query'
  showSaveModal.value = true
}

async function confirmSave() {
  if (!saveFileName.value.trim()) return
  try {
    const result = await saveQuery(
      props.resourceId,
      saveFileName.value.trim(),
      activeTab.value.sql,
      selectedDb.value,
    )
    markSaved(activeTabId.value, result.id)
    showSaveModal.value = false
    toast.success(t('sql.toast.saveSuccess'))
  } catch {
    toast.error(t('sql.toast.saveFailed'))
  }
}

function handleFormat() {
  sqlEditorRef.value?.formatSql()
}

function handleTabRename(id: string) {
  const newTitle = prompt(t('sql.sidebar.renamePrompt'))
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

/* Save Modal */
.ws-sql-modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.ws-sql-modal {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  width: 400px;
  max-width: 90%;
}

.ws-sql-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-sm) var(--sp-md);
  border-bottom: 1px solid var(--border);
  font-weight: 600;
}

.ws-sql-modal-body {
  padding: var(--sp-md);
}

.ws-sql-modal-label {
  display: block;
  margin-bottom: var(--sp-xs);
  font-size: var(--fs-xs);
  color: var(--text-secondary);
}

.ws-sql-modal-input {
  width: 100%;
  padding: var(--sp-xs) var(--sp-sm);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
}

.ws-sql-modal-input:focus {
  outline: none;
  border-color: var(--accent);
}

.ws-sql-modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-xs);
  padding: var(--sp-sm) var(--sp-md);
  border-top: 1px solid var(--border);
}
</style>
