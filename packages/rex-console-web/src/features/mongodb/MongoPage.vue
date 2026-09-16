<script setup lang="ts">
/**
 * MongoPage — MongoDB 控制台
 * 支持连接、数据库/集合浏览、查询执行。
 */
import { ref, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import Button from '@/components/ui/Button.vue'
import * as mongoApi from '@/api/mongodb'

const { t } = useI18n()

const props = defineProps<{
  resourceId?: string
}>()

const emit = defineEmits<{
  'update:status': [status: string]
}>()

// ── State ──────────────────────────────────────
const sessionId = ref<string | null>(null)
const databases = ref<string[]>([])
const collections = ref<string[]>([])
const selectedDb = ref('')
const selectedColl = ref('')
const queryText = ref('{ "filter": {} }')
const operation = ref('find')
const loading = ref(false)
const error = ref('')
const resultDocs = ref<Record<string, unknown>[]>([])
const resultCount = ref(0)
const elapsedMs = ref(0)
const connected = ref(false)

// ── Connect ────────────────────────────────────
async function onConnect() {
  loading.value = true
  error.value = ''
  try {
    if (!props.resourceId) throw new Error('No resource ID')
    sessionId.value = await mongoApi.connect(props.resourceId)
    connected.value = true
    emit('update:status', 'connected')
    await loadDatabases()
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function onDisconnect() {
  if (!sessionId.value) return
  try {
    await mongoApi.disconnect(sessionId.value)
  } catch { /* ignore */ }
  sessionId.value = null
  connected.value = false
  databases.value = []
  collections.value = []
  resultDocs.value = []
  emit('update:status', 'disconnected')
}

// ── Metadata ───────────────────────────────────
async function loadDatabases() {
  if (!sessionId.value) return
  try {
    databases.value = await mongoApi.getDatabases(sessionId.value)
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  }
}

async function loadCollections(db: string) {
  if (!sessionId.value) return
  selectedDb.value = db
  collections.value = []
  selectedColl.value = ''
  try {
    collections.value = await mongoApi.getCollections(sessionId.value, db)
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  }
}

// ── Query ──────────────────────────────────────
async function onExecute() {
  if (!sessionId.value || !selectedDb.value || !selectedColl.value) return
  loading.value = true
  error.value = ''
  resultDocs.value = []

  try {
    let filter: Record<string, unknown> = {}
    try {
      filter = JSON.parse(queryText.value)
    } catch {
      throw new Error('Invalid JSON filter')
    }

    const data = await mongoApi.query(
      sessionId.value,
      selectedDb.value,
      selectedColl.value,
      operation.value,
      filter.filter as Record<string, unknown> || filter,
    )
    if (data.error) throw new Error(data.error)
    resultDocs.value = data.documents || []
    resultCount.value = data.count ?? data.documents?.length ?? 0
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

// ── Cleanup ────────────────────────────────────
onBeforeUnmount(() => {
  if (sessionId.value) {
    mongoApi.disconnect(sessionId.value).catch(() => {})
  }
})
</script>

<template>
  <div class="mongo-page">
    <!-- Header -->
    <div class="mp-header">
      <div class="mp-header-left">
        <span v-if="connected" class="mp-status mp-status-ok">● {{ t('mongo.connected', 'Connected') }}</span>
        <span v-else class="mp-status mp-status-off">○ {{ t('mongo.disconnected', 'Disconnected') }}</span>
      </div>
      <div class="mp-header-right">
        <Button v-if="!connected" :disabled="loading" @click="onConnect">
          {{ loading ? t('mongo.connecting', 'Connecting...') : t('mongo.connect', 'Connect') }}
        </Button>
        <Button v-else @click="onDisconnect" variant="danger">
          {{ t('mongo.disconnect', 'Disconnect') }}
        </Button>
      </div>
    </div>

    <!-- Error -->
    <div v-if="error" class="mp-error">❌ {{ error }}</div>

    <!-- Main content -->
    <div v-if="connected" class="mp-body">
      <!-- Left: tree -->
      <div class="mp-sidebar">
        <div class="mp-sidebar-title">{{ t('mongo.databases', 'Databases') }}</div>
        <div class="mp-tree">
          <div
            v-for="db in databases"
            :key="db"
            class="mp-tree-db"
          >
            <div
              class="mp-tree-item"
              :class="{ 'mp-tree-active': selectedDb === db }"
              @click="loadCollections(db)"
            >📁 {{ db }}</div>
            <div v-if="selectedDb === db" class="mp-tree-children">
              <div
                v-for="coll in collections"
                :key="coll"
                class="mp-tree-item mp-tree-coll"
                :class="{ 'mp-tree-active': selectedColl === coll }"
                @click="selectedColl = coll"
              >📄 {{ coll }}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- Right: query + results -->
      <div class="mp-main">
        <!-- Query bar -->
        <div class="mp-query-bar">
          <select v-model="operation" class="mp-select">
            <option value="find">find</option>
            <option value="count">count</option>
            <option value="aggregate">aggregate</option>
          </select>
          <textarea
            v-model="queryText"
            class="mp-query-input"
            :placeholder="t('mongo.queryPlaceholder', 'Filter JSON')"
            rows="3"
          />
          <Button :disabled="loading || !selectedColl" @click="onExecute">
            {{ loading ? t('mongo.executing', 'Running...') : '▶ ' + t('mongo.execute', 'Execute') }}
          </Button>
        </div>

        <!-- Results -->
        <div class="mp-results-header">
          <span>{{ resultCount }} {{ t('mongo.docs', 'docs') }} ({{ elapsedMs }}ms)</span>
        </div>
        <div class="mp-results">
          <div v-if="resultDocs.length === 0" class="mp-empty">
            {{ t('mongo.noResults', 'No results. Select a database and collection, then execute a query.') }}
          </div>
          <div v-for="(doc, i) in resultDocs" :key="i" class="mp-doc">
            <pre class="mp-doc-json">{{ JSON.stringify(doc, null, 2) }}</pre>
          </div>
        </div>
      </div>
    </div>

    <!-- Disconnected state -->
    <div v-else class="mp-empty-state">
      <div class="mp-empty-icon">🍃</div>
      <div>{{ t('mongo.prompt', 'Connect to a MongoDB resource to get started.') }}</div>
    </div>
  </div>
</template>

<style scoped>
.mongo-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.mp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
}

.mp-status { font-size: 12px; }
.mp-status-ok { color: #22c55e; }
.mp-status-off { color: var(--text-secondary); }

.mp-error {
  padding: var(--space-2) var(--space-3);
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
  font-size: 12px;
}

.mp-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.mp-sidebar {
  width: 220px;
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.mp-sidebar-title {
  padding: var(--space-2);
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-secondary);
  letter-spacing: 0.5px;
}

.mp-tree { flex: 1; }

.mp-tree-item {
  padding: 4px 8px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}

.mp-tree-item:hover { background: var(--bg-hover); }
.mp-tree-active { background: var(--bg-active, rgba(59, 130, 246, 0.1)); }
.mp-tree-children { padding-left: 12px; }
.mp-tree-coll { font-family: monospace; }

.mp-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.mp-query-bar {
  display: flex;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
  align-items: flex-start;
}

.mp-select {
  padding: 4px 8px;
  font-size: 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-primary);
}

.mp-query-input {
  flex: 1;
  padding: 6px 8px;
  font-size: 12px;
  font-family: monospace;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-primary);
  resize: vertical;
  min-height: 60px;
}

.mp-results-header {
  padding: var(--space-1) var(--space-3);
  font-size: 11px;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border);
}

.mp-results {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-2);
}

.mp-doc {
  margin-bottom: var(--space-2);
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: auto;
}

.mp-doc-json {
  margin: 0;
  padding: var(--space-2);
  font-size: 11px;
  font-family: monospace;
  white-space: pre-wrap;
  word-break: break-all;
}

.mp-empty, .mp-empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  font-size: 13px;
  flex-direction: column;
  gap: var(--space-2);
}

.mp-empty-icon { font-size: 32px; }
</style>
