<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

interface Column {
  name: string
  type: string
  nullable: boolean
  is_primary_key: boolean
  is_new?: boolean
}

const model = defineModel<Column[]>({ required: true })

const emit = defineEmits<{
  remove: [index: number]
}>()

const commonTypes = [
  'INT', 'BIGINT', 'SMALLINT', 'TINYINT',
  'VARCHAR', 'TEXT', 'MEDIUMTEXT', 'LONGTEXT',
  'DECIMAL', 'FLOAT', 'DOUBLE',
  'DATE', 'DATETIME', 'TIMESTAMP', 'TIME',
  'BOOLEAN', 'BIT',
  'BLOB', 'JSON',
  'UUID', 'SERIAL', 'BIGSERIAL',
]
</script>

<template>
  <div class="column-editor">
    <table class="column-table">
      <thead>
        <tr>
          <th class="col-num">#</th>
          <th>{{ t('sql.columnName') }}</th>
          <th>{{ t('sql.columnType') }}</th>
          <th class="col-pk">{{ t('sql.pk') }}</th>
          <th class="col-nn">{{ t('sql.nn') }}</th>
          <th class="col-action"></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(col, index) in model" :key="index" :class="{ 'row-new': col.is_new }">
          <td class="col-num">{{ index + 1 }}</td>
          <td>
            <input
              class="cell-input mono"
              type="text"
              :value="col.name"
              :placeholder="t('sql.columnPlaceholder')"
              @input="col.name = ($event.target as HTMLInputElement).value"
            />
          </td>
          <td>
            <select
              class="cell-select mono"
              :value="col.type"
              @change="col.type = ($event.target as HTMLSelectElement).value"
            >
              <option v-for="ct in commonTypes" :key="ct" :value="ct">{{ ct }}</option>
              <option v-if="!commonTypes.includes(col.type)" :value="col.type">{{ col.type }}</option>
            </select>
          </td>
          <td class="col-pk">
            <input
              type="checkbox"
              :checked="col.is_primary_key"
              @change="col.is_primary_key = ($event.target as HTMLInputElement).checked"
            />
          </td>
          <td class="col-nn">
            <input
              type="checkbox"
              :checked="!col.nullable"
              @change="col.nullable = !($event.target as HTMLInputElement).checked"
            />
          </td>
          <td class="col-action">
            <button class="remove-btn" :title="t('sql.removeColumn')" @click="emit('remove', index)">×</button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.column-editor {
  overflow: auto;
}

.column-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-sm);
}

.column-table th,
.column-table td {
  padding: var(--space-1) var(--space-2);
  text-align: left;
  border-bottom: 1px solid var(--border);
}

.column-table th {
  background: var(--bg-surface);
  font-weight: 600;
  color: var(--text-muted);
  font-size: var(--text-xs);
  text-transform: uppercase;
  position: sticky;
  top: 0;
  z-index: 1;
}

.column-table tr:hover td {
  background: var(--bg-hover);
}

.row-new {
  background: rgba(232, 145, 45, 0.05);
}

.col-num {
  width: 40px;
  text-align: right;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.col-pk,
.col-nn {
  width: 50px;
  text-align: center;
}

.col-action {
  width: 40px;
  text-align: center;
}

.cell-input {
  width: 100%;
  padding: var(--space-1);
  background: transparent;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
  transition: border-color var(--transition);
}

.cell-input:focus {
  border-color: var(--accent);
  background: var(--bg-surface);
}

.cell-select {
  padding: var(--space-1);
  background: transparent;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
  cursor: pointer;
  transition: border-color var(--transition);
}

.cell-select:focus {
  border-color: var(--accent);
  background: var(--bg-surface);
}

input[type="checkbox"] {
  accent-color: var(--accent);
  cursor: pointer;
}

.remove-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-md);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  transition: color var(--transition), background var(--transition);
}

.remove-btn:hover {
  color: var(--danger);
  background: rgba(248, 81, 73, 0.15);
}

.mono {
  font-family: var(--font-mono);
}
</style>
