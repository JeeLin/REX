<template>
  <div class="sql-layout">
    <!-- Top Bar -->
    <SqlTopbar
      v-if="resource"
      :resource-name="resource.name"
      :protocol="resource.protocol"
      :databases="databases"
      :selected-db="selectedDb"
      @update:selected-db="onDbChange"
      @refresh="loadDatabases"
    />
    <div v-else class="sql-topbar">
      <button class="btn btn-ghost btn-sm" @click="goBack">← {{ t('common.back') }}</button>
    </div>

    <!-- Tabs -->
    <SqlTabs
      :tabs="tabList"
      :active-id="activeTabId"
      @select="activeTabId = $event"
      @close="closeTab"
      @close-others="closeOthers"
      @close-all="closeAll"
      @close-saved="closeSaved"
      @save="handleTabSave"
      @save-as="handleTabSaveAs"
      @rename="handleTabRename"
      @copy-sql="handleTabCopySql"
      @execute-sql="handleTabExecuteSql"
      @add="addTab"
    />

    <!-- Toolbar (desktop) -->
    <div v-if="!isMobile" class="sql-toolbar">
      <button class="btn btn-run btn-sm" @click="execute(activeTab.sql)">
        ▶ {{ t('sql.execute') }}
      </button>
      <div class="sql-toolbar-sep"></div>
      <button class="btn btn-ghost btn-sm" @click="clearEditor">{{ t('sql.clear') }}</button>
      <div class="sql-toolbar-sep"></div>
      <div class="sql-toolbar-spacer"></div>
      <span class="sql-toolbar-info">Ctrl+Enter {{ t('sql.execute') }}</span>
      <button class="btn btn-ghost btn-sm" @click="openGlobalQuery">
        ⊞ {{ t('sql.globalQuery') }}
      </button>
    </div>

    <!-- Main Area -->
    <div class="sql-main">
      <!-- Sidebar -->
      <SqlSidebar
        v-if="selectedDb"
        ref="sidebarRef"
        :resource-id="resourceId"
        :database="selectedDb"
        :databases="databases"
        :protocol="resource?.protocol ?? ''"
        @update:database="onDbChange"
        @select-table="insertTableSql"
        @open-query="handleOpenQuery"
        @refresh="loadDatabases"
        @query-deleted="handleQueryDeleted"
        @query-renamed="handleQueryRenamed"
        @open-sql-tab="openSqlTab"
        @export-table="handleExportTable"
      />

      <!-- Right: Editor + Results -->
      <div class="sql-right">
        <div class="sql-editor-section" :style="{ height: (splitRatio * 100) + '%' }">
          <SqlEditor
            ref="editorRef"
            v-model="activeTab.sql"
            :dialect="editorDialect"
            @execute="execute(activeTab.sql)"
            @execute-selection="execute"
            @save="handleToolbarSave"
            @show-history="showHistoryPanel = true"
          />
        </div>
        <div class="sql-resize-handle" @mousedown.prevent="startEditorResize" />
        <SqlResults
          :result="activeTab.result"
          :loading="executing"
          :message="activeTab.message"
          :is-error="activeTab.isError"
          @sort="handleSort"
          @generate-sql="handleGenerateSql"
        />
        <SqlHistoryPanel
          :resource-id="resourceId"
          :visible="showHistoryPanel"
          @close="showHistoryPanel = false"
          @select="handleHistorySelect"
          @open-sql-tab="openSqlTab"
        />
      </div>
    </div>
  </div>

  <!-- Mobile Toolbar -->
  <SqlMobileToolbar
    :visible="isMobile"
    @execute="execute(activeTab.sql)"
    @format="handleFormat"
    @clear="clearEditor"
    @save="handleToolbarSave"
    @history="showHistoryPanel = true"
    @global-query="openGlobalQuery"
  />

  <!-- Global Query Modal -->
  <GlobalQueryModal
    v-model:visible="globalQueryVisible"
    :peer-resources="peerResources"
  />
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useToast } from '@/composables/useToast'
import SqlTopbar from '@/features/sql/SqlTopbar.vue'
import SqlTabs from '@/features/sql/SqlTabs.vue'
import SqlSidebar from '@/features/sql/SqlSidebar.vue'
import SqlEditor from '@/features/sql/SqlEditor.vue'
import SqlResults from '@/features/sql/SqlResults.vue'
import SqlHistoryPanel from '@/features/sql/SqlHistoryPanel.vue'
import SqlMobileToolbar from '@/features/sql/SqlMobileToolbar.vue'
import GlobalQueryModal from '@/components/GlobalQueryModal.vue'
import { listDatabases, getResourceInfo, getQuery, saveQuery, updateQuery, recordHistory, listPeerSqlResources, executeSql } from '@/api/sql'
import type { DatabaseInfo, SqlResourceInfo, QueryFileMeta, HistoryRecord, SqlResult } from '@/api/sql'
import { exportCsv } from '@/features/sql/result-export'
import { useSqlTabActions } from '@/features/sql/useSqlTabActions'

const { t } = useI18n()
const toast = useToast()
const route = useRoute()
const router = useRouter()
const resourceId = route.params.resourceId as string

const {
  tabs, activeTabId, executing, tabList, activeTab,
  addTab, closeTab, closeOthers, closeAll, closeSaved, renameTab, getTabSql,
  clearEditor, openQueryFile, openSqlTab, markSaved, getQueryId,
  execute, handleSort, handleGenerateSql,
} = useSqlTabActions(
  resourceId,
  (msg) => toast.error(msg),
  (sql: string, result: SqlResult) => {
    // 自动记录执行历史
    recordHistory(resourceId, sql, selectedDb.value, result.elapsed_ms, result.rows.length)
  },
)

// Resource info
const resource = ref<{ name: string; protocol: string } | null>(null)
const databases = ref<DatabaseInfo[]>([])
const selectedDb = ref('')
const sidebarRef = ref<InstanceType<typeof SqlSidebar>>()
const editorRef = ref<InstanceType<typeof SqlEditor>>()
const showHistoryPanel = ref(false)
const globalQueryVisible = ref(false)
const peerResources = ref<SqlResourceInfo[]>([])

// Mobile detection
const isMobile = ref(window.innerWidth < 768)
function checkMobile() {
  isMobile.value = window.innerWidth < 768
}

// Editor/Results split resize
const EDITOR_SPLIT_KEY = 'rex-sql-editor-split'
const splitRatio = ref(parseFloat(localStorage.getItem(EDITOR_SPLIT_KEY) || '0.4'))
let resizingEditor = false
let resizeStartY = 0
let resizeStartRatio = 0

function startEditorResize(e: MouseEvent) {
  resizingEditor = true
  resizeStartY = e.clientY
  resizeStartRatio = splitRatio.value
  document.addEventListener('mousemove', onEditorResize)
  document.addEventListener('mouseup', stopEditorResize)
  document.body.style.cursor = 'row-resize'
  document.body.style.userSelect = 'none'
}

function onEditorResize(e: MouseEvent) {
  if (!resizingEditor) return
  const sqlRight = document.querySelector('.sql-right') as HTMLElement
  if (!sqlRight) return
  const rect = sqlRight.getBoundingClientRect()
  const delta = e.clientY - resizeStartY
  const newRatio = resizeStartRatio + delta / rect.height
  splitRatio.value = Math.min(0.8, Math.max(0.15, newRatio))
}

function stopEditorResize() {
  resizingEditor = false
  document.removeEventListener('mousemove', onEditorResize)
  document.removeEventListener('mouseup', stopEditorResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  localStorage.setItem(EDITOR_SPLIT_KEY, String(splitRatio.value))
}

onBeforeUnmount(() => {
  document.removeEventListener('mousemove', onEditorResize)
  document.removeEventListener('mouseup', stopEditorResize)
})

// SQL dialect from protocol
const editorDialect = computed(() => {
  const protocol = resource.value?.protocol?.toLowerCase() ?? ''
  if (protocol.includes('mysql')) return 'mysql' as const
  if (protocol.includes('postgres')) return 'postgresql' as const
  if (protocol.includes('sqlite')) return 'sqlite' as const
  return 'sql' as const
})

function insertTableSql(tableName: string) {
  const tab = tabs.value.find((t) => t.id === activeTabId.value)
  if (tab) tab.sql = `SELECT * FROM ${tableName} LIMIT 100;`
}

async function handleExportTable(tableName: string) {
  try {
    const result = await executeSql(resourceId, `SELECT * FROM \`${tableName}\``)
    exportCsv(result.columns, result.rows)
    toast.success(t('sql.toast.exportSuccess'))
  } catch {
    toast.error(t('sql.toast.exportFailed'))
  }
}

function handleOpenQuery(query: QueryFileMeta) {
  // 加载查询文件内容并打开
  getQuery(resourceId, query.id).then((detail) => {
    openQueryFile(query.id, query.name, detail.sql)
  })
}

async function handleTabSave(id: string) {
  const tab = tabs.value.find((t) => t.id === id)
  if (!tab) return

  const existingQueryId = getQueryId(id)
  const database = selectedDb.value

  if (existingQueryId) {
    // 已保存的查询文件，直接更新
    await updateQuery(resourceId, existingQueryId, {
      sql: tab.sql,
      database,
    })
  } else {
    // 新查询文件，弹出命名对话框
    const name = prompt(t('sql.savePrompt'))
    if (!name || !name.trim()) return
    const saved = await saveQuery(resourceId, name.trim(), tab.sql, database)
    markSaved(id, saved.id)
    tab.title = saved.name
  }

  // 刷新侧边栏查询文件列表
  sidebarRef.value?.loadQueries()
}

function handleToolbarSave() {
  handleTabSave(activeTabId.value)
}

function handleFormat() {
  editorRef.value?.formatSql()
}

function handleToolbarAction(e: Event) {
  const type = (e as CustomEvent).detail
  switch (type) {
    case 'openQuery': openGlobalQuery(); break
  }
}

function handleTabRename(id: string) {
  const newTitle = prompt(t('sql.sidebar.renamePrompt'))
  if (newTitle) renameTab(id, newTitle)
}

async function handleTabSaveAs(id: string) {
  const tab = tabs.value.find((t) => t.id === id)
  if (!tab) return
  const name = prompt(t('sql.savePrompt'))
  if (!name || !name.trim()) return
  const saved = await saveQuery(resourceId, name.trim(), tab.sql, selectedDb.value)
  markSaved(id, saved.id)
  tab.title = saved.name
  sidebarRef.value?.loadQueries()
}

function handleTabCopySql(id: string) {
  const sql = getTabSql(id)
  if (sql) navigator.clipboard.writeText(sql)
}

function handleTabExecuteSql(id: string) {
  const tab = tabs.value.find((t) => t.id === id)
  if (tab) execute(tab.sql)
}

function handleQueryDeleted() {
  const tab = tabs.value.find((t) => t.id === activeTabId.value)
  if (tab) tab.queryId = null
}

function handleQueryRenamed() {
  // 侧边栏自动刷新，无需额外处理
}

function handleHistorySelect(record: HistoryRecord) {
  const tab = tabs.value.find((t) => t.id === activeTabId.value)
  if (tab) {
    tab.sql = record.sql
  }
  showHistoryPanel.value = false
}

async function loadDatabases() {
  try {
    databases.value = await listDatabases(resourceId)
    if (databases.value.length > 0 && !selectedDb.value) {
      selectedDb.value = databases.value[0]!.name
    }
  } catch {
    databases.value = []
  }
}

function onDbChange(db: string) {
  selectedDb.value = db
}

function goBack() {
  router.back()
}

function openGlobalQuery() {
  loadPeerResources()
  globalQueryVisible.value = true
}

async function loadPeerResources() {
  try {
    peerResources.value = await listPeerSqlResources(resourceId)
  } catch {
    peerResources.value = []
  }
}

onMounted(async () => {
  try {
    resource.value = await getResourceInfo(resourceId)
  } catch {
    router.push('/')
    return
  }
  addTab()
  await loadDatabases()
  window.addEventListener('resize', checkMobile)
  window.addEventListener('sql-toolbar-action', handleToolbarAction)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', checkMobile)
  window.removeEventListener('sql-toolbar-action', handleToolbarAction)
})
</script>

<style scoped>
.sql-layout {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-deep);
}

.sql-topbar {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-md);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 40px;
  flex-shrink: 0;
  gap: var(--sp-md);
}

.sql-toolbar {
  display: flex;
  align-items: center;
  padding: var(--sp-xs) var(--sp-md);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 36px;
  flex-shrink: 0;
  gap: var(--sp-sm);
}

.sql-toolbar .btn { height: 28px; font-size: var(--fs-sm); }

.btn-run {
  background: var(--success) !important;
  border-color: var(--success) !important;
  color: #000 !important;
  font-weight: 600;
}

.btn-run:hover {
  opacity: 0.9;
}

.sql-toolbar-sep {
  width: 1px;
  height: 20px;
  background: var(--border);
  margin: 0 var(--sp-xs);
}

.sql-toolbar-spacer { flex: 1; }

.sql-toolbar-info {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.sql-main {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.sql-right {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.sql-resize-handle {
  height: 4px;
  cursor: row-resize;
  background: var(--border);
  flex-shrink: 0;
  transition: background 0.15s;
}

.sql-resize-handle:hover {
  background: var(--accent);
}

.sql-right > .sql-editor-section {
  flex: none;
  overflow: hidden;
}

.sql-right > .sql-results {
  flex: 1;
  min-height: 100px;
}
</style>
