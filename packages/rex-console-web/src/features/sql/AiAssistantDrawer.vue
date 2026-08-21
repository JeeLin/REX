<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  visible: boolean
  sessionId: string
  db: string
  table?: string
  query?: string
}>()

const emit = defineEmits<{
  close: []
  insertSql: [sql: string]
}>()

interface AiAction {
  id: string
  label: string
  icon: string
  description: string
}

const actions = computed<AiAction[]>(() => [
  { id: 'analyze', label: t('sql.analyzeSlowQuery'), icon: '🔍', description: t('sql.analyzeSlowQueryDesc') },
  { id: 'optimize', label: t('sql.optimizeSql'), icon: '⚡', description: t('sql.optimizeSqlDesc') },
  { id: 'generate', label: t('sql.generateSql'), icon: '📝', description: t('sql.generateSqlDesc') },
  { id: 'relations', label: t('sql.tableRelationships'), icon: '🔗', description: t('sql.tableRelationshipsDesc') },
])

const selectedAction = ref<string | null>(null)
const input = ref('')
const response = ref('')
const loading = ref(false)

function selectAction(action: AiAction) {
  selectedAction.value = action.id
  input.value = ''
  response.value = ''
}

async function executeAction() {
  if (!selectedAction.value) return

  loading.value = true
  response.value = ''

  // Simulate AI response (in real implementation, this would call an API)
  await new Promise((resolve) => setTimeout(resolve, 1000))

  const context = props.table ? `${props.db}.${props.table}` : props.db
  const queryContext = props.query ? `\nQuery: ${props.query}` : ''

  switch (selectedAction.value) {
    case 'analyze':
      response.value = `## Query Analysis for ${context}${queryContext}

### Performance Metrics
- Estimated rows scanned: 10,000
- Index usage: None detected
- Query complexity: Medium

### Suggestions
1. **Add index** on \`status\` column for WHERE clause
2. **Consider pagination** for large result sets
3. **Use EXPLAIN** to verify execution plan

### Risk
⚠️ This query may perform a full table scan on large datasets.`
      break
    case 'optimize':
      response.value = `## Optimized SQL for ${context}${queryContext}

\`\`\`sql
-- Original query
${props.query || 'SELECT * FROM table'}

-- Optimized version
SELECT id, name, email, status
FROM ${props.table || 'table'}
WHERE status = 'active'
LIMIT 100
\`\`\`

### Changes Made
- Selected specific columns instead of \`*\`
- Added \`LIMIT\` to prevent large result sets
- Ensured proper index usage`
      break
    case 'generate':
      response.value = `## Generated SQL for ${context}

Based on your input: "${input.value || 'No description provided'}"

\`\`\`sql
-- Generated query
SELECT *
FROM ${props.table || 'table'}
WHERE ${input.value ? 'column = \'value\'' : 'condition'}
\`\`\`

### Notes
- Adjust the column names and conditions as needed
- Add appropriate indexes for better performance`
      break
    case 'relations':
      response.value = `## Table Relationships for ${context}

### Foreign Keys
- \`user_id\` → \`users.id\`
- \`category_id\` → \`categories.id\`

### Related Tables
- \`users\` (1:N)
- \`categories\` (N:1)
- \`orders\` (1:N)

### Suggested JOINs
\`\`\`sql
SELECT t.*, u.name AS user_name, c.name AS category_name
FROM ${props.table || 'table'} t
LEFT JOIN users u ON t.user_id = u.id
LEFT JOIN categories c ON t.category_id = c.id
\`\`\``
      break
  }

  loading.value = false
}

function insertToEditor() {
  // Extract SQL from response (simplified)
  const sqlMatch = response.value.match(/```sql\n([\s\S]*?)```/)
  if (sqlMatch && sqlMatch[1]) {
    emit('insertSql', sqlMatch[1].trim())
  }
}

function close() {
  selectedAction.value = null
  input.value = ''
  response.value = ''
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <Transition name="drawer">
      <div v-if="visible" class="drawer-overlay" @click.self="close">
        <div class="drawer-content">
          <div class="drawer-header">
            <span class="drawer-title">{{ t('sql.aiAssistant') }}</span>
            <button class="drawer-close" @click="close">×</button>
          </div>

          <div class="drawer-body">
            <!-- Context -->
            <div class="context">
              <span class="context-label">{{ t('sql.context') }}:</span>
              <span class="context-value mono">{{ db }}{{ table ? `.${table}` : '' }}</span>
            </div>

            <!-- Quick Actions -->
            <div class="section">
              <div class="section-label">{{ t('sql.quickActions') }}:</div>
              <div class="actions-grid">
                <button
                  v-for="action in actions"
                  :key="action.id"
                  class="action-btn"
                  :class="{ 'action-btn--selected': selectedAction === action.id }"
                  @click="selectAction(action)"
                >
                  <span class="action-icon">{{ action.icon }}</span>
                  <span class="action-label">{{ action.label }}</span>
                </button>
              </div>
            </div>

            <!-- Response Area -->
            <div class="section">
              <div class="response-area">
                <div v-if="loading" class="loading">
                  <div class="spinner" />
                  <span>{{ t('sql.analyzing') }}</span>
                </div>
                <div v-else-if="response" class="response-content">
                  <pre class="response-text">{{ response }}</pre>
                  <button
                    v-if="response.includes('```sql')"
                    class="insert-btn"
                    @click="insertToEditor"
                  >
                    {{ t('sql.insertToEditor') }}
                  </button>
                </div>
                <div v-else class="response-placeholder">
                  {{ t('sql.selectActionHint') }}
                </div>
              </div>
            </div>

            <!-- Input -->
            <div class="input-area">
              <input
                v-model="input"
                class="input-field"
                type="text"
                :placeholder="t('sql.askQuestion')"
                @keyup.enter="executeAction"
              />
              <button
                class="input-btn"
                :disabled="!selectedAction || loading"
                @click="executeAction"
              >
                →
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.drawer-overlay {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  left: 0;
  background: rgba(0, 0, 0, 0.3);
  z-index: 999;
}

.drawer-content {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 400px;
  background: var(--bg-surface);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  box-shadow: -10px 0 30px rgba(0, 0, 0, 0.3);
}

.drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-4);
  border-bottom: 1px solid var(--border);
}

.drawer-title {
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.drawer-close {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-xl);
  padding: var(--space-1);
  border-radius: var(--radius-sm);
  transition: color var(--transition);
}

.drawer-close:hover {
  color: var(--danger);
}

.drawer-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.context {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3);
  background: var(--bg-deep);
  border-radius: var(--radius-sm);
}

.context-label {
  font-size: var(--text-sm);
  color: var(--text-muted);
}

.context-value {
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.section-label {
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-primary);
}

.actions-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-2);
}

.action-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition);
}

.action-btn:hover {
  border-color: var(--accent);
  background: var(--bg-hover);
}

.action-btn--selected {
  border-color: var(--accent);
  background: rgba(232, 145, 45, 0.1);
}

.action-icon {
  font-size: var(--text-xl);
}

.action-label {
  font-size: var(--text-sm);
  color: var(--text-primary);
  text-align: center;
}

.response-area {
  flex: 1;
  min-height: 200px;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-3);
  overflow-y: auto;
}

.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  height: 100%;
  color: var(--text-muted);
}

.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.response-content {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.response-text {
  margin: 0;
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}

.insert-btn {
  align-self: flex-start;
  padding: var(--space-2) var(--space-3);
  background: var(--accent);
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-on-accent);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: opacity var(--transition);
}

.insert-btn:hover {
  opacity: 0.9;
}

.response-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-muted);
  font-size: var(--text-sm);
}

.input-area {
  display: flex;
  gap: var(--space-2);
}

.input-field {
  flex: 1;
  padding: var(--space-3);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}

.input-field:focus {
  border-color: var(--accent);
}

.input-btn {
  padding: var(--space-3) var(--space-4);
  background: var(--accent);
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-on-accent);
  font-size: var(--text-lg);
  cursor: pointer;
  transition: opacity var(--transition);
}

.input-btn:hover {
  opacity: 0.9;
}

.input-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.mono {
  font-family: var(--font-mono);
}

/* Drawer transition */
.drawer-enter-active,
.drawer-leave-active {
  transition: all 0.3s ease;
}

.drawer-enter-active .drawer-content,
.drawer-leave-active .drawer-content {
  transition: transform 0.3s ease;
}

.drawer-enter-from,
.drawer-leave-to {
  background: rgba(0, 0, 0, 0);
}

.drawer-enter-from .drawer-content,
.drawer-leave-to .drawer-content {
  transform: translateX(100%);
}
</style>
