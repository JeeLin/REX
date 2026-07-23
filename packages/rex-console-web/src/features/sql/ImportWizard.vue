<script setup lang="ts">
import { ref, computed } from 'vue'
import { executeQuery } from '@/api/sql'

const props = defineProps<{
  visible: boolean
  sessionId: string
  db: string
  table: string
}>()

const emit = defineEmits<{
  close: []
  imported: []
}>()

type ImportFormat = 'csv' | 'json' | 'sql'

const step = ref<'select' | 'preview' | 'options' | 'importing' | 'done'>('select')
const file = ref<File | null>(null)
const format = ref<ImportFormat>('csv')
const delimiter = ref(',')
const hasHeader = ref(true)
const skipDuplicates = ref(true)
const previewData = ref<any[]>([])
const previewColumns = ref<string[]>([])
const importing = ref(false)
const importResult = ref<{ success: number; failed: number; errors: string[] } | null>(null)

function onFileSelect(e: Event) {
  const input = e.target as HTMLInputElement
  if (input.files && input.files.length > 0) {
    const f = input.files[0]
    if (f) processFile(f)
  }
}

function onDrop(e: DragEvent) {
  e.preventDefault()
  if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
    const f = e.dataTransfer.files[0]
    if (f) processFile(f)
  }
}

function onDragOver(e: DragEvent) {
  e.preventDefault()
}

async function processFile(f: File) {
  file.value = f

  // Auto-detect format
  if (f.name.endsWith('.csv')) {
    format.value = 'csv'
  } else if (f.name.endsWith('.json')) {
    format.value = 'json'
  } else if (f.name.endsWith('.sql')) {
    format.value = 'sql'
  }

  // Read file for preview
  const content = await f.text()
  parsePreview(content)
  step.value = 'preview'
}

function parsePreview(content: string) {
  if (format.value === 'csv') {
    const lines = content.split('\n').filter((l) => l.trim())
    if (lines.length === 0) return

    const firstLine = lines[0]
    if (!firstLine) return

    if (hasHeader.value) {
      previewColumns.value = firstLine.split(delimiter.value).map((c) => c.trim())
      previewData.value = lines.slice(1, 6).map((line) => {
        const values = line.split(delimiter.value)
        const row: any = {}
        previewColumns.value.forEach((col, i) => {
          row[col] = values[i]?.trim() || ''
        })
        return row
      })
    } else {
      const firstLineParts = firstLine.split(delimiter.value)
      previewColumns.value = firstLineParts.map((_, i) => `Column ${i + 1}`)
      previewData.value = lines.slice(0, 5).map((line) => {
        const values = line.split(delimiter.value)
        const row: any = {}
        previewColumns.value.forEach((col, i) => {
          row[col] = values[i]?.trim() || ''
        })
        return row
      })
    }
  } else if (format.value === 'json') {
    try {
      const data = JSON.parse(content)
      const rows = Array.isArray(data) ? data : [data]
      if (rows.length > 0 && rows[0]) {
        previewColumns.value = Object.keys(rows[0])
        previewData.value = rows.slice(0, 5)
      }
    } catch {
      previewColumns.value = ['Error']
      previewData.value = [{ Error: 'Invalid JSON format' }]
    }
  }
}

function goToOptions() {
  step.value = 'options'
}

function goBack() {
  if (step.value === 'preview') {
    step.value = 'select'
  } else if (step.value === 'options') {
    step.value = 'preview'
  }
}

async function startImport() {
  if (!file.value) return

  step.value = 'importing'
  importing.value = true
  importResult.value = { success: 0, failed: 0, errors: [] }

  try {
    const content = await file.value.text()
    const statements = generateInsertStatements(content)

    // Execute in batches
    const batchSize = 100
    for (let i = 0; i < statements.length; i += batchSize) {
      const batch = statements.slice(i, i + batchSize)
      const batchSql = batch.join(';\n') + ';'

      try {
        await executeQuery(props.sessionId, batchSql)
        importResult.value.success += batch.length
      } catch (e: unknown) {
        importResult.value.failed += batch.length
        importResult.value.errors.push(
          e instanceof Error ? e.message : String(e)
        )
      }
    }

    step.value = 'done'
    emit('imported')
  } catch (e: unknown) {
    importResult.value.errors.push(
      e instanceof Error ? e.message : String(e)
    )
    step.value = 'done'
  } finally {
    importing.value = false
  }
}

function generateInsertStatements(content: string): string[] {
  const statements: string[] = []

  if (format.value === 'csv') {
    const lines = content.split('\n').filter((l) => l.trim())
    const startIdx = hasHeader.value ? 1 : 0
    const firstLine = lines[0]
    const columns = hasHeader.value && firstLine
      ? firstLine.split(delimiter.value).map((c) => c.trim())
      : previewColumns.value

    for (let i = startIdx; i < lines.length; i++) {
      const line = lines[i]
      if (!line) continue
      const values = line.split(delimiter.value).map((v) => {
        const trimmed = v.trim()
        if (trimmed === 'NULL' || trimmed === 'null') return 'NULL'
        return `'${trimmed.replace(/'/g, "''")}'`
      })

      const cols = columns.map((c) => `\`${c}\``).join(', ')
      const vals = values.join(', ')
      let sql = `INSERT INTO \`${props.table}\` (${cols}) VALUES (${vals})`

      if (skipDuplicates.value) {
        sql += ' ON DUPLICATE KEY UPDATE id = id'
      }

      statements.push(sql)
    }
  } else if (format.value === 'json') {
    const data = JSON.parse(content)
    const rows = Array.isArray(data) ? data : [data]

    for (const row of rows) {
      const columns = Object.keys(row)
      const cols = columns.map((c) => `\`${c}\``).join(', ')
      const vals = columns.map((c) => {
        const val = row[c]
        if (val === null || val === undefined) return 'NULL'
        if (typeof val === 'number') return val.toString()
        return `'${String(val).replace(/'/g, "''")}'`
      }).join(', ')

      let sql = `INSERT INTO \`${props.table}\` (${cols}) VALUES (${vals})`

      if (skipDuplicates.value) {
        sql += ' ON DUPLICATE KEY UPDATE id = id'
      }

      statements.push(sql)
    }
  } else if (format.value === 'sql') {
    // SQL file - split by semicolons and execute directly
    statements.push(...content.split(';').filter((s) => s.trim()))
  }

  return statements
}

function close() {
  step.value = 'select'
  file.value = null
  previewData.value = []
  previewColumns.value = []
  importResult.value = null
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-overlay" @click.self="close">
      <div class="modal-content modal-lg">
        <div class="modal-header">
          <span class="modal-title">Import Data to {{ table }}</span>
          <button class="modal-close" @click="close">×</button>
        </div>

        <div class="modal-body">
          <!-- Step 1: Select File -->
          <div v-if="step === 'select'" class="step">
            <div class="step-label">Step 1: Select File</div>
            <div
              class="drop-zone"
              @drop="onDrop"
              @dragover="onDragOver"
            >
              <div class="drop-icon">📁</div>
              <div class="drop-text">Choose File or drag & drop here</div>
              <input
                type="file"
                accept=".csv,.json,.sql"
                class="file-input"
                @change="onFileSelect"
              />
            </div>
          </div>

          <!-- Step 2: Preview -->
          <div v-if="step === 'preview'" class="step">
            <div class="step-label">Step 2: Preview</div>
            <div class="preview-table-wrap">
              <table class="preview-table">
                <thead>
                  <tr>
                    <th v-for="col in previewColumns" :key="col" class="mono">
                      {{ col }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, i) in previewData" :key="i">
                    <td v-for="col in previewColumns" :key="col" class="mono">
                      {{ row[col] }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div class="preview-info">
              {{ previewData.length }} rows preview (showing first 5)
            </div>
          </div>

          <!-- Step 3: Options -->
          <div v-if="step === 'options'" class="step">
            <div class="step-label">Step 3: Options</div>
            <div class="options-list">
              <label class="option-item">
                <input v-model="skipDuplicates" type="checkbox" />
                <span>Skip duplicates (ON DUPLICATE KEY IGNORE)</span>
              </label>
            </div>
          </div>

          <!-- Step: Importing -->
          <div v-if="step === 'importing'" class="step">
            <div class="importing-status">
              <div class="spinner" />
              <span>Importing data...</span>
            </div>
          </div>

          <!-- Step: Done -->
          <div v-if="step === 'done'" class="step">
            <div class="import-result">
              <div v-if="importResult?.errors.length" class="result-error">
                <div class="result-icon">❌</div>
                <div class="result-text">
                  Import completed with errors
                </div>
                <div class="result-details">
                  {{ importResult.success }} succeeded, {{ importResult.failed }} failed
                </div>
                <div class="result-errors">
                  <div v-for="(err, i) in importResult.errors" :key="i" class="error-item">
                    {{ err }}
                  </div>
                </div>
              </div>
              <div v-else class="result-success">
                <div class="result-icon">✅</div>
                <div class="result-text">
                  Import completed successfully
                </div>
                <div class="result-details">
                  {{ importResult?.success }} rows imported
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-footer">
          <button
            v-if="step === 'preview' || step === 'options'"
            class="btn btn-secondary"
            @click="goBack"
          >
            Back
          </button>
          <button class="btn btn-secondary" @click="close">
            {{ step === 'done' ? 'Close' : 'Cancel' }}
          </button>
          <button
            v-if="step === 'preview'"
            class="btn btn-primary"
            @click="goToOptions"
          >
            Next
          </button>
          <button
            v-if="step === 'options'"
            class="btn btn-primary"
            :disabled="importing"
            @click="startImport"
          >
            Import
          </button>
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

.modal-lg {
  width: 800px;
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

.step {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.step-label {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
}

.drop-zone {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-8);
  border: 2px dashed var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition);
}

.drop-zone:hover {
  border-color: var(--accent);
  background: rgba(232, 145, 45, 0.05);
}

.drop-icon {
  font-size: 48px;
  margin-bottom: var(--space-3);
}

.drop-text {
  font-size: var(--text-md);
  color: var(--text-muted);
}

.file-input {
  position: absolute;
  width: 100%;
  height: 100%;
  opacity: 0;
  cursor: pointer;
}

.preview-table-wrap {
  overflow-x: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}

.preview-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-sm);
}

.preview-table th,
.preview-table td {
  padding: var(--space-2) var(--space-3);
  text-align: left;
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
}

.preview-table th {
  background: var(--bg-surface);
  font-weight: 600;
  color: var(--text-muted);
  font-size: var(--text-xs);
  text-transform: uppercase;
}

.preview-table tr:hover td {
  background: var(--bg-hover);
}

.preview-info {
  font-size: var(--text-sm);
  color: var(--text-muted);
}

.options-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.option-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-sm);
  color: var(--text-primary);
  cursor: pointer;
}

.option-item input[type="checkbox"] {
  accent-color: var(--accent);
}

.importing-status {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-3);
  padding: var(--space-8);
  color: var(--text-muted);
}

.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.import-result {
  display: flex;
  justify-content: center;
  padding: var(--space-8);
}

.result-success,
.result-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
  text-align: center;
}

.result-icon {
  font-size: 48px;
  margin-bottom: var(--space-2);
}

.result-text {
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.result-details {
  font-size: var(--text-sm);
  color: var(--text-muted);
}

.result-errors {
  margin-top: var(--space-3);
  padding: var(--space-3);
  background: var(--bg-deep);
  border-radius: var(--radius-sm);
  max-height: 100px;
  overflow-y: auto;
}

.error-item {
  font-size: var(--text-xs);
  color: var(--danger);
  font-family: var(--font-mono);
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-3);
  padding: var(--space-4);
  border-top: 1px solid var(--border);
}

.btn {
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition);
}

.btn-secondary {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  color: var(--text-primary);
}

.btn-secondary:hover {
  background: var(--bg-hover);
}

.btn-primary {
  background: var(--accent);
  border: 1px solid var(--accent);
  color: white;
}

.btn-primary:hover {
  opacity: 0.9;
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.mono {
  font-family: var(--font-mono);
}
</style>
