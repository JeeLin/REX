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
    <div class="sql-topbar" v-else>
      <button class="btn btn-ghost btn-sm" @click="goBack">← {{ t('common.back') }}</button>
    </div>

    <!-- Tabs -->
    <SqlTabs
      :tabs="tabList"
      :active-id="activeTabId"
      @select="activeTabId = $event"
      @close="closeTab"
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
    </div>

    <!-- Main Area -->
    <div class="sql-main">
      <!-- Sidebar -->
      <SqlSidebar
        v-if="selectedDb"
        :resource-id="resourceId"
        :database="selectedDb"
        @select-table="insertTableSql"
        @refresh="loadDatabases"
      />

      <!-- Right: Editor + Results -->
      <div class="sql-right">
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
  </div>
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
import { listDatabases, getResourceInfo } from '@/api/sql'
import type { DatabaseInfo } from '@/api/sql'
import { useSqlTabActions } from '@/features/sql/useSqlTabActions'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const resourceId = route.params.resourceId as string

const {
  tabs, activeTabId, executing, tabList, activeTab,
  addTab, closeTab, clearEditor, execute, handleSort, handleGenerateSql,
} = useSqlTabActions(resourceId, (msg) => alert(msg))

// Resource info
const resource = ref<{ name: string; protocol: string } | null>(null)
const databases = ref<DatabaseInfo[]>([])
const selectedDb = ref('')

function insertTableSql(tableName: string) {
  const tab = tabs.value.find((t) => t.id === activeTabId.value)
  if (tab) tab.sql = `SELECT * FROM ${tableName} LIMIT 100;`
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
