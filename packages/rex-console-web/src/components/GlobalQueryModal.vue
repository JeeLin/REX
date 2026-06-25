<template>
  <div class="modal-overlay" @click.self="close" v-show="visible">
    <div class="modal-content">
      <div class="modal-header">
        <span>⊞ 全局查询</span>
        <button @click="close">×</button>
      </div>

      <div class="modal-body">
        <div class="db-selector">
          <label>资源：</label>
          <div class="db-checkbox-group">
            <label
              v-for="res in peerResources"
              :key="res.id"
              class="db-checkbox"
              :class="{ disabled: !checkCompatibility(res.protocol) }"
            >
              <input
                type="checkbox"
                :value="res.id"
                v-model="selectedResources"
                :disabled="!checkCompatibility(res.protocol)"
                @change="onResourceSelectionChange"
              />
              <span>{{ res.name }} ({{ res.protocol }})</span>
            </label>
          </div>
          <div class="db-select-actions">
            <button
              @click="selectAllCompatible"
              :disabled="!selectedResources.length"
              class="btn btn-ghost btn-sm"
            >
              全选兼容
            </button>
          </div>
          <small class="hint">仅支持相同方言的数据库（MySQL 或 PostgreSQL）</small>
        </div>

        <div class="sql-editor">
          <textarea
            ref="textareaRef"
            v-model="sqlQuery"
            placeholder="输入 SQL 查询语句..."
            @keydown.ctrl.enter="executeGlobalQuery"
          />
          <div class="editor-toolbar">
            <button
              @click="executeGlobalQuery"
              :disabled="!selectedResources.length || !sqlQuery.trim() || isExecuting"
            >
              执行 (Ctrl+Enter)
            </button>
            <button
              @click="cancelQuery"
              :disabled="!isExecuting"
            >
              取消
            </button>
            <progress
              v-if="isExecuting"
              :value="progress"
              max="100"
            ></progress>
          </div>
        </div>

        <div v-if="results.length > 0" class="results-tabs">
          <div class="tab-bar">
            <button
              v-for="(result, index) in results"
              :key="index"
              :class="{ active: activeTab === index }"
              @click="activeTab = index"
            >
              {{ result.connectionName }}
              <span v-if="result.error !== undefined" class="error-tag">错误</span>
              <span v-else>{{ result.rowCount }} 行</span>
            </button>
          </div>
          <div class="tab-content">
            <div v-if="results[activeTab]" class="result-view">
              <div v-if="results[activeTab].error" class="error-message">
                ❌ {{ results[activeTab].error }}
              </div>
              <div v-else class="results-table">
                <table v-if="results[activeTab].data.length > 0">
                  <thead>
                    <tr>
                      <th v-for="col in results[activeTab].columns" :key="col">
                        {{ col }}
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="(row, rowIndex) in results[activeTab].data" :key="rowIndex">
                      <td v-for="(colName, colIndex) in results[activeTab].columns" :key="colIndex">
                        {{ formatValue(row[colName]) }}
                      </td>
                    </tr>
                  </tbody>
                </table>
                <div v-else class="empty-result">无结果数据</div>
              </div>
            </div>
          </div>
        </div>

        <div v-else class="empty-state">暂无结果</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useGlobalQuery, type SqlResource } from '@/composables/useGlobalQuery'

const props = defineProps<{
  visible: boolean
  peerResources: SqlResource[]
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
}>()

const textareaRef = ref<HTMLTextAreaElement>()

const {
  selectedResources,
  sqlQuery,
  isExecuting,
  progress,
  results,
  activeTab,
  executeGlobalQuery,
  cancelQuery,
  checkCompatibility,
  onResourceSelectionChange,
  selectAllCompatible,
} = useGlobalQuery(props.peerResources)

watch(
  () => props.visible,
  (newVal) => {
    if (newVal) {
      selectedResources.value = []
      sqlQuery.value = ''
      isExecuting.value = false
      progress.value = 0
      results.value = []
      activeTab.value = 0
      textareaRef.value?.focus()
    }
  },
)

function close() {
  emit('update:visible', false)
}

function formatValue(value: any): string {
  if (value === null || value === undefined) {
    return '(null)'
  }
  if (typeof value === 'object') {
    try {
      return JSON.stringify(value)
    } catch {
      return String(value)
    }
  }
  return String(value)
}
</script>

<style scoped>
.modal-overlay {
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

.modal-content {
  background: var(--bg-panel);
  border-radius: 8px;
  width: 90%;
  max-width: 800px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
  overflow: hidden;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-header);
}

.modal-header span {
  font-weight: 600;
  font-size: 18px;
  color: var(--text-primary);
}

.modal-header button {
  background: transparent;
  border: none;
  font-size: 24px;
  cursor: pointer;
  color: var(--text-secondary);
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-header button:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
  border-radius: 4px;
}

.modal-body {
  flex: 1;
  overflow: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.db-selector {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.db-selector > label {
  font-weight: 500;
  color: var(--text-primary);
}

.db-checkbox-group {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

.db-checkbox {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  padding: 8px 12px;
  border-radius: 4px;
  transition: background 0.2s;
}

.db-checkbox:hover:not(.disabled) {
  background: var(--bg-hover);
}

.db-checkbox.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.db-checkbox input {
  width: 16px;
  height: 16px;
  cursor: pointer;
}

.db-checkbox span {
  font-size: 14px;
}

.hint {
  font-size: 12px;
  color: var(--text-muted);
}

.sql-editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.sql-editor textarea {
  flex: 1;
  min-height: 100px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--bg-input);
  color: var(--text-primary);
  font-family: 'Fira Code', 'Courier New', monospace;
  font-size: 14px;
  resize: vertical;
}

.sql-editor textarea:focus {
  outline: none;
  border-color: var(--primary);
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
}

.editor-toolbar {
  display: flex;
  gap: 12px;
  align-items: center;
}

.editor-toolbar button {
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.editor-toolbar button:first-child {
  background: var(--primary);
  color: white;
}

.editor-toolbar button:first-child:hover:not(:disabled) {
  background: var(--primary-dark);
}

.editor-toolbar button:first-child:disabled {
  background: var(--bg-muted);
  cursor: not-allowed;
}

.editor-toolbar button:nth-child(2) {
  background: var(--bg-muted);
  color: var(--text-primary);
}

.editor-toolbar button:nth-child(2):hover:not(:disabled) {
  background: var(--bg-hover);
}

.editor-toolbar button:nth-child(2):disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.editor-toolbar progress {
  width: 150px;
  height: 6px;
}

.editor-toolbar progress::-webkit-progress-bar {
  background: var(--bg-muted);
  border-radius: 3px;
}

.editor-toolbar progress::-webkit-progress-value {
  background: var(--primary);
  border-radius: 3px;
}

.results-tabs {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.tab-bar {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
}

.tab-bar button {
  padding: 6px 12px;
  border: 1px solid var(--border);
  background: var(--bg-muted);
  color: var(--text-secondary);
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}

.tab-bar button.active {
  background: var(--primary);
  color: white;
  border-color: var(--primary);
}

.tab-bar button:hover:not(.active) {
  background: var(--bg-hover);
  border-color: var(--border-hover);
}

.error-tag {
  background: var(--error);
  color: white;
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 3px;
  margin-left: 8px;
}

.tab-content {
  flex: 1;
  overflow: auto;
}

.result-view {
  padding: 16px;
}

.error-message {
  padding: 16px;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid var(--error);
  border-radius: 4px;
  color: var(--error);
  font-size: 14px;
}

.results-table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0;
}

.results-table th,
.results-table td {
  padding: 12px 8px;
  text-align: left;
  border-bottom: 1px solid var(--border);
}

.results-table th {
  background: var(--bg-muted);
  font-weight: 600;
  font-size: 13px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.results-table tr:hover {
  background: var(--bg-hover);
}

.results-table td {
  font-size: 14px;
}

.empty-result {
  padding: 24px;
  text-align: center;
  color: var(--text-muted);
  font-style: italic;
}

.empty-state {
  padding: 40px;
  text-align: center;
  color: var(--text-muted);
  font-style: italic;
}
</style>
