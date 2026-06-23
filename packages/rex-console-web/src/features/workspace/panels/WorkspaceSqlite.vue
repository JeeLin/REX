<template>
  <div class="ws-sqlite">
    <div class="sqlite-topbar">
      <span class="sqlite-status-dot" :class="{ connected: session.connected.value }" />
      <span class="sqlite-label">SQLite</span>
      <span class="sqlite-name">{{ resourceName }}</span>
      <span v-if="dbPath" class="sqlite-dbpath">{{ dbPath }}</span>
      <div class="sqlite-spacer" />
      <button
        v-if="!session.connected.value"
        class="btn btn-ghost btn-sm"
        :disabled="connecting"
        @click="handleConnect"
      >
        {{ connecting ? t('docker.connecting') : t('docker.connect') }}
      </button>
      <button
        v-else
        class="btn btn-ghost btn-sm btn-danger"
        @click="handleDisconnect"
      >
        {{ t('docker.disconnect') }}
      </button>
    </div>

    <!-- 错误 -->
    <div v-if="session.error.value" class="sqlite-error">
      {{ session.error.value }}
    </div>

    <!-- 欢迎页 -->
    <div v-if="!session.connected.value && !session.error.value" class="sqlite-welcome">
      <p>{{ t('sqlite.welcome') }}</p>
    </div>

    <!-- SQL 控制台 -->
    <template v-else>
      <!-- 工具栏 -->
      <div class="sqlite-toolbar">
        <button class="btn btn-run btn-sm" :disabled="executing" @click="runQuery">
          ▶ {{ t('sql.execute') }}
        </button>
        <span class="toolbar-sep" />
        <button class="btn btn-ghost btn-sm" @click="clearEditor">{{ t('sql.clear') }}</button>
        <div class="toolbar-spacer" />
        <span class="toolbar-info">Ctrl+Enter</span>
      </div>

      <!-- 主区域 -->
      <div class="sqlite-main">
        <!-- 侧边栏：表列表 -->
        <div class="sqlite-sidebar">
          <div class="sidebar-header">{{ t('sqlite.tables') }}</div>
          <div class="sidebar-list">
            <div
              v-for="table in tables"
              :key="table"
              class="sidebar-item"
              @click="insertTable(table)"
            >
              {{ table }}
            </div>
            <div v-if="tables.length === 0" class="sidebar-empty">
              {{ t('sqlite.noTables') }}
            </div>
          </div>
        </div>

        <!-- 编辑器 + 结果 -->
        <div class="sqlite-content">
          <textarea
            v-model="sql"
            class="sqlite-editor"
            :placeholder="t('sql.placeholder')"
            @keydown.ctrl.enter.prevent="runQuery"
            @keydown.meta.enter.prevent="runQuery"
          />
          <div class="sqlite-result">
            <div v-if="result" class="result-table-wrap">
              <table class="result-table">
                <thead>
                  <tr>
                    <th v-for="col in result.columns" :key="col">{{ col }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, i) in result.rows" :key="i">
                    <td v-for="(cell, j) in row" :key="j" :class="{ 'cell-null': cell === null }">
                      {{ cell === null ? 'NULL' : cell }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div v-if="resultMessage" class="result-message" :class="{ error: resultIsError }">
              {{ resultMessage }}
            </div>
            <div v-if="!result && !resultMessage" class="result-empty">
              {{ t('sql.noResult') }}
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSqliteSession, type SqliteResult } from '@/features/sqlite/useSqliteSession'

const props = defineProps<{
  resourceId: string
  resourceName: string
}>()

const { t } = useI18n()

const session = useSqliteSession(() => props.resourceId)

const connecting = ref(false)
const sql = ref('')
const result = ref<SqliteResult | null>(null)
const resultMessage = ref('')
const resultIsError = ref(false)
const executing = ref(false)
const tables = ref<string[]>([])

const dbPath = computed(() => session.serverInfo.value?.db_path ?? '')

// ── 连接/断开 ────────────────────────────────────────────
async function handleConnect() {
  connecting.value = true
  try {
    await session.connect()
    await refreshTables()
  } catch {
    // error is set in session.error
  } finally {
    connecting.value = false
  }
}

function handleDisconnect() {
  session.disconnect()
  tables.value = []
  result.value = null
  resultMessage.value = ''
}

// ── 表列表 ──────────────────────────────────────────────
async function refreshTables() {
  if (!session.connected.value) return
  try {
    tables.value = await session.listTables()
  } catch {
    tables.value = []
  }
}

function insertTable(table: string) {
  sql.value = `SELECT * FROM ${table} LIMIT 100;`
}

// ── 执行查询 ────────────────────────────────────────────
async function runQuery() {
  if (!sql.value.trim() || executing.value) return
  executing.value = true
  result.value = null
  resultMessage.value = ''
  resultIsError.value = false
  try {
    const r = await session.executeSql(sql.value)
    result.value = r
    resultMessage.value = `Query OK, ${r.affected_rows ?? r.rows.length} rows (${(r.elapsed_ms / 1000).toFixed(3)}s)`
  } catch (err) {
    result.value = null
    resultMessage.value = `ERROR: ${err instanceof Error ? err.message : String(err)}`
    resultIsError.value = true
  } finally {
    executing.value = false
  }
}

function clearEditor() {
  sql.value = ''
  result.value = null
  resultMessage.value = ''
}
</script>

<style scoped>
.ws-sqlite {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.sqlite-topbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  font-size: 13px;
  flex-shrink: 0;
}

.sqlite-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #f85149;
}
.sqlite-status-dot.connected { background: #3fb950; }
.sqlite-label { font-weight: 600; color: #D29922; }
.sqlite-name { color: var(--text-secondary); }
.sqlite-dbpath { color: var(--text-secondary); font-size: 11px; font-family: 'JetBrains Mono', monospace; }
.sqlite-spacer { flex: 1; }

.btn-danger { border-color: #f85149; color: #f85149; }

.sqlite-error {
  color: #f85149;
  padding: 8px 12px;
  border-bottom: 1px solid #f8514933;
  background: #f8514911;
  font-size: 12px;
  flex-shrink: 0;
}

.sqlite-welcome {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  font-size: 14px;
}

.sqlite-toolbar {
  display: flex;
  align-items: center;
  padding: 4px 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
  flex-shrink: 0;
  gap: 8px;
}

.btn-run {
  background: #3fb950 !important;
  border-color: #3fb950 !important;
  color: #000 !important;
  font-weight: 600;
}

.toolbar-sep { width: 1px; height: 16px; background: var(--border-primary); }
.toolbar-spacer { flex: 1; }
.toolbar-info { font-size: 11px; color: var(--text-secondary); font-family: 'JetBrains Mono', monospace; }

.sqlite-main {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.sqlite-sidebar {
  width: 180px;
  border-right: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.sidebar-header {
  padding: 8px 12px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border-primary);
}

.sidebar-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.sidebar-item {
  padding: 4px 12px;
  font-size: 12px;
  font-family: 'JetBrains Mono', monospace;
  cursor: pointer;
  color: var(--text-primary);
}
.sidebar-item:hover { background: var(--bg-hover); }

.sidebar-empty {
  padding: 12px;
  font-size: 12px;
  color: var(--text-secondary);
}

.sqlite-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.sqlite-editor {
  min-height: 120px;
  max-height: 40%;
  padding: 12px;
  background: var(--bg-primary);
  color: var(--text-primary);
  border: none;
  border-bottom: 1px solid var(--border-primary);
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
  line-height: 1.5;
  resize: vertical;
  outline: none;
  width: 100%;
  box-sizing: border-box;
}

.sqlite-result {
  flex: 1;
  overflow: auto;
  padding: 8px;
}

.result-table-wrap { overflow: auto; }

.result-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
  font-family: 'JetBrains Mono', monospace;
}

.result-table th {
  text-align: left;
  padding: 6px 10px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
  font-weight: 600;
  color: var(--text-secondary);
  position: sticky;
  top: 0;
}

.result-table td {
  padding: 4px 10px;
  border-bottom: 1px solid var(--border-primary);
}

.cell-null { color: var(--text-secondary); font-style: italic; }

.result-message {
  padding: 8px 12px;
  font-size: 12px;
  color: var(--text-secondary);
}
.result-message.error { color: #f85149; }

.result-empty {
  padding: 20px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 13px;
}
</style>
