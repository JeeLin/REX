<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { executeQuery, type QueryResult } from '@/api/sql'
import Button from '@/components/ui/Button.vue'

const { t } = useI18n()

const props = defineProps<{
  visible: boolean
  sessionId: string
  databases: string[]
}>()

const emit = defineEmits<{
  close: []
  execute: [results: GlobalQueryResult[]]
}>()

interface GlobalQueryResult {
  db: string
  result: QueryResult | null
  error: string | null
}

const selectedDbs = ref<string[]>([])
const query = ref('')
const loading = ref(false)
const results = ref<GlobalQueryResult[]>([])

function toggleDb(db: string) {
  const idx = selectedDbs.value.indexOf(db)
  if (idx >= 0) {
    selectedDbs.value.splice(idx, 1)
  } else {
    selectedDbs.value.push(db)
  }
}

function selectAll() {
  selectedDbs.value = [...props.databases]
}

function clearAll() {
  selectedDbs.value = []
}

async function execute() {
  if (!query.value.trim() || selectedDbs.value.length === 0) return

  loading.value = true
  results.value = []

  for (const db of selectedDbs.value) {
    try {
      // Execute query in the context of each selected database
      const useDbQuery = `USE \`${db}\`; ${query.value}`
      const result = await executeQuery(props.sessionId, useDbQuery)
      results.value.push({ db, result, error: null })
    } catch (e: unknown) {
      results.value.push({
        db,
        result: null,
        error: e instanceof Error ? e.message : String(e),
      })
    }
  }

  loading.value = false
  emit('execute', results.value)
}

function close() {
  selectedDbs.value = []
  query.value = ''
  results.value = []
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-overlay" @click.self="close">
      <div class="modal-content">
        <div class="modal-header">
          <span class="modal-title">{{ t('sql.globalQuery') }}</span>
          <button class="modal-close" @click="close">×</button>
        </div>

        <div class="modal-body">
          <!-- Database selection -->
          <div class="section">
            <div class="section-header">
              <span class="section-label">{{ t('sql.selectDatabases') }}:</span>
              <div class="section-actions">
                <button class="text-btn" @click="selectAll">{{ t('sql.selectAll') }}</button>
                <button class="text-btn" @click="clearAll">{{ t('sql.clear') }}</button>
              </div>
            </div>
            <div class="db-list">
              <label
                v-for="db in databases"
                :key="db"
                class="db-item"
                :class="{ 'db-item--selected': selectedDbs.includes(db) }"
              >
                <input
                  type="checkbox"
                  :checked="selectedDbs.includes(db)"
                  @change="toggleDb(db)"
                />
                <span class="db-name mono">{{ db }}</span>
              </label>
            </div>
          </div>

          <!-- Warning -->
          <div class="warning">
            ⚠️ {{ t('sql.sameDialectOnly') }}
          </div>

          <!-- Query editor -->
          <div class="section">
            <div class="section-label">{{ t('sql.query') }}:</div>
            <textarea
              v-model="query"
              class="query-input mono"
              rows="6"
              :placeholder="t('sql.queryPlaceholder')"
              @keydown.enter.exact.prevent="execute"
            />
          </div>

          <!-- Results preview -->
          <div v-if="results.length > 0" class="section">
            <div class="section-label">{{ t('sql.results') }}:</div>
            <div class="results-list">
              <div
                v-for="r in results"
                :key="r.db"
                class="result-item"
                :class="{ 'result-item--error': r.error }"
              >
                <span class="result-db mono">{{ r.db }}</span>
                <span v-if="r.error" class="result-error">{{ r.error }}</span>
                <span v-else class="result-success">
                  {{ r.result?.rows?.length || 0 }} {{ t('sql.rows') }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-footer">
          <Button variant="ghost" @click="close">{{ t('common.cancel') }}</Button>
          <Button
            variant="primary"
            :disabled="loading || !query.trim() || selectedDbs.length === 0"
            @click="execute"
          >
            {{ loading ? t('sql.executing') : t('sql.execute') }}
          </Button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  width: 600px;
  max-width: 90vw;
  max-height: 80vh;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-4);
  border-bottom: 1px solid var(--border);
}

.modal-title {
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.modal-close {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-xl);
  padding: var(--space-1);
  border-radius: var(--radius-sm);
  transition: color var(--transition);
}

.modal-close:hover {
  color: var(--danger);
}

.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4);
}

.section {
  margin-bottom: var(--space-4);
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-2);
}

.section-label {
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-primary);
  margin-bottom: var(--space-2);
}

.section-actions {
  display: flex;
  gap: var(--space-2);
}

.text-btn {
  background: none;
  border: none;
  color: var(--accent);
  cursor: pointer;
  font-size: var(--text-sm);
  padding: 0;
}

.text-btn:hover {
  text-decoration: underline;
}

.db-list {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  max-height: 120px;
  overflow-y: auto;
}

.db-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition);
}

.db-item:hover {
  border-color: var(--accent);
}

.db-item--selected {
  background: rgba(232, 145, 45, 0.1);
  border-color: var(--accent);
}

.db-item input[type="checkbox"] {
  accent-color: var(--accent);
}

.db-name {
  font-size: var(--text-sm);
}

.warning {
  padding: var(--space-3);
  background: rgba(210, 153, 34, 0.1);
  border: 1px solid rgba(210, 153, 34, 0.3);
  border-radius: var(--radius-sm);
  color: var(--warning);
  font-size: var(--text-sm);
  margin-bottom: var(--space-4);
}

.query-input {
  width: 100%;
  padding: var(--space-3);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  resize: vertical;
  outline: none;
}

.query-input:focus {
  border-color: var(--accent);
}

.results-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  max-height: 150px;
  overflow-y: auto;
}

.result-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  background: var(--bg-deep);
  border-radius: var(--radius-sm);
}

.result-item--error {
  background: rgba(248, 81, 73, 0.1);
  border: 1px solid rgba(248, 81, 73, 0.3);
}

.result-db {
  font-size: var(--text-sm);
  color: var(--text-primary);
  min-width: 100px;
}

.result-error {
  font-size: var(--text-sm);
  color: var(--danger);
}

.result-success {
  font-size: var(--text-sm);
  color: var(--success);
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-3);
  padding: var(--space-4);
  border-top: 1px solid var(--border);
}

.mono {
  font-family: var(--font-mono);
}
</style>
