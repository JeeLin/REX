<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { ColumnInfo } from '@/api/sql'

const props = defineProps<{
  columns: ColumnInfo[]
  rows: unknown[][]
  currentIndex: number
}>()

const emit = defineEmits<{
  'update:currentIndex': [index: number]
  save: [row: unknown[]]
  discard: []
}>()

const editValues = ref<Record<string, unknown>>({})
const isEditing = ref(false)

// Reset edit values when row changes
watch(() => props.currentIndex, () => {
  editValues.value = {}
  isEditing.value = false
})

const currentRow = computed(() => props.rows[props.currentIndex] || [])
const totalRows = computed(() => props.rows.length)
const hasChanges = computed(() => Object.keys(editValues.value).length > 0)

function getValue(colIndex: number): unknown {
  const key = `col_${colIndex}`
  if (key in editValues.value) {
    return editValues.value[key]
  }
  return currentRow.value[colIndex]
}

function setValue(colIndex: number, value: unknown) {
  const key = `col_${colIndex}`
  const original = currentRow.value[colIndex]
  if (value === original || (value === '' && original === null)) {
    delete editValues.value[key]
  } else {
    editValues.value[key] = value
  }
  isEditing.value = true
}

function formatValue(value: unknown): string {
  if (value === null || value === undefined) return 'NULL'
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}

function goPrevious() {
  if (props.currentIndex > 0) {
    emit('update:currentIndex', props.currentIndex - 1)
  }
}

function goNext() {
  if (props.currentIndex < totalRows.value - 1) {
    emit('update:currentIndex', props.currentIndex + 1)
  }
}

function saveChanges() {
  const newRow = [...currentRow.value]
  Object.entries(editValues.value).forEach(([key, value]) => {
    const colIndex = parseInt(key.replace('col_', ''), 10)
    newRow[colIndex] = value
  })
  emit('save', newRow)
  editValues.value = {}
  isEditing.value = false
}

function discardChanges() {
  editValues.value = {}
  isEditing.value = false
  emit('discard')
}

function formatFieldName(name: string): string {
  return name.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())
}
</script>

<template>
  <div class="form-view">
    <!-- Navigation -->
    <div class="form-nav">
      <button
        class="nav-btn"
        :disabled="currentIndex === 0"
        @click="goPrevious"
      >
        ◀
      </button>
      <span class="nav-info">
        {{ currentIndex + 1 }} / {{ totalRows }}
      </span>
      <button
        class="nav-btn"
        :disabled="currentIndex >= totalRows - 1"
        @click="goNext"
      >
        ▶
      </button>
    </div>

    <!-- Form fields -->
    <div class="form-fields">
      <div
        v-for="(col, ci) in columns"
        :key="col.name"
        class="form-field"
      >
        <label class="field-label">{{ formatFieldName(col.name) }}</label>
        <input
          class="field-input mono"
          :value="formatValue(getValue(ci))"
          :placeholder="col.nullable ? 'NULL' : ''"
          @input="setValue(ci, ($event.target as HTMLInputElement).value)"
        />
      </div>
    </div>

    <!-- Actions -->
    <div class="form-actions">
      <button class="btn btn-secondary" :disabled="currentIndex === 0" @click="goPrevious">
        Previous
      </button>
      <button class="btn btn-secondary" :disabled="currentIndex >= totalRows - 1" @click="goNext">
        Next
      </button>
      <span class="actions-spacer" />
      <button v-if="hasChanges" class="btn btn-primary" @click="saveChanges">
        Save
      </button>
      <button v-if="hasChanges" class="btn btn-secondary" @click="discardChanges">
        Discard
      </button>
    </div>
  </div>
</template>

<style scoped>
.form-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-deep);
}

.form-nav {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-4);
  padding: var(--space-3);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
}

.nav-btn {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  cursor: pointer;
  transition: all var(--transition);
}

.nav-btn:hover:not(:disabled) {
  background: var(--bg-hover);
  border-color: var(--accent);
}

.nav-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.nav-info {
  font-size: var(--text-sm);
  color: var(--text-primary);
  min-width: 80px;
  text-align: center;
}

.form-fields {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.field-label {
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-muted);
}

.field-input {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
  transition: border-color var(--transition);
}

.field-input:focus {
  border-color: var(--accent);
}

.form-actions {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  background: var(--bg-surface);
  border-top: 1px solid var(--border);
}

.actions-spacer {
  flex: 1;
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

.btn-secondary:hover:not(:disabled) {
  background: var(--bg-hover);
}

.btn-secondary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background: var(--accent);
  border: 1px solid var(--accent);
  color: white;
}

.btn-primary:hover {
  opacity: 0.9;
}

.mono {
  font-family: var(--font-mono);
}
</style>
