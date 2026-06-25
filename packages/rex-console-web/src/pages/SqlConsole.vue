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
      @rename="handleTabRename"
      @copy-sql="handleTabCopySql"
      @execute-sql="handleTabExecuteSql"
      @add="addTab"
    />

    <!-- Toolbar -->
    <div class="sql-toolbar">
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
        @select-table="insertTableSql"
        @open-query="handleOpenQuery"
        @refresh="loadDatabases"
        @query-deleted="handleQueryDeleted"
        @query-renamed="handleQueryRenamed"
      />

      <!-- Right: Editor + Results -->
      <div class="sql-right">
        <SqlEditor
          v-model="activeTab.sql"
          @execute="execute(activeTab.sql)"
          @execute-selection="execute"
          @save="handleToolbarSave"
          @show-history="showHistoryPanel = true"
        />
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
        />
      </div>
    </div>
  </div>

  <!-- Global Query Modal -->
  <GlobalQueryModal
    v-model:visible="globalQueryVisible"
    :peer-resources="peerResources"
  />
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import SqlTopbar from '@/features/sql/SqlTopbar.vue'
import SqlTabs from '@/features/sql/SqlTabs.vue'
import SqlSidebar from '@/features/sql/SqlSidebar.vue'
import SqlEditor from '@/features/sql/SqlEditor.vue'
import SqlResults from '@/features/sql/SqlResults.vue'
import SqlHistoryPanel from '@/features/sql/SqlHistoryPanel.vue'
import GlobalQueryModal from '@/components/GlobalQueryModal.vue'
import { listDatabases, getResourceInfo, getQuery, saveQuery, updateQuery, recordHistory, listPeerSqlResources } from '@/api/sql'
import type { DatabaseInfo, SqlResourceInfo, QueryFileMeta, HistoryRecord, SqlResult } from '@/api/sql'
import { useSqlTabActions } from '@/features/sql/useSqlTabActions'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const resourceId = route.params.resourceId as string

const {
  tabs, activeTabId, executing, tabList, activeTab,
  addTab, closeTab, closeOthers, closeAll, closeSaved, renameTab, getTabSql,
  clearEditor, openQueryFile, markSaved, getQueryId,
  execute, handleSort, handleGenerateSql,
} = useSqlTabActions(
  resourceId,
  (msg) => alert(msg),
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
const showHistoryPanel = ref(false)
const globalQueryVisible = ref(false)
const peerResources = ref<SqlResourceInfo[]>([])

function insertTableSql(tableName: string) {
  const tab = tabs.value.find((t) => t.id === activeTabId.value)
  if (tab) tab.sql = `SELECT * FROM ${tableName} LIMIT 100;`
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
      selectedDb.value = databases.value[0].name
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
})
</script>

<style scoped>
.sql-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
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
</style>
