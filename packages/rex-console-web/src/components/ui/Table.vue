<script setup lang="ts" generic="T extends Record<string, any>">
withDefaults(defineProps<{
  columns: { key: string; label: string; width?: string; align?: 'left' | 'right' | 'center' }[]
  rows: T[]
  rowKey?: (row: T, index: number) => string | number
  striped?: boolean
  compact?: boolean
}>(), { striped: false, compact: false })

defineEmits<{ rowClick: [row: T, index: number] }>()
</script>

<template>
  <div class="table-wrap">
    <table class="table" :class="{ 'table--striped': striped, 'table--compact': compact }">
      <thead>
        <tr>
          <th
            v-for="col in columns"
            :key="col.key"
            :style="{ width: col.width, textAlign: col.align }"
            class="th"
          >
            {{ col.label }}
          </th>
        </tr>
      </thead>
      <tbody>
        <template v-if="rows.length">
          <tr
            v-for="(row, i) in rows"
            :key="rowKey ? rowKey(row, i) : i"
            class="tr"
            @click="$emit('rowClick', row, i)"
          >
            <td
              v-for="col in columns"
              :key="col.key"
              :style="{ textAlign: col.align }"
              class="td"
            >
              <slot :name="`cell-${col.key}`" :row="row" :value="row[col.key]">
                {{ row[col.key] }}
              </slot>
            </td>
          </tr>
        </template>
        <tr v-else>
          <td :colspan="columns.length" class="td td--empty">
            <slot name="empty">
              <span class="muted">No data</span>
            </slot>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.table-wrap {
  overflow: auto;
}
.table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-base);
}
.th {
  position: sticky;
  top: 0;
  background: var(--bg-surface);
  text-align: left;
  font-weight: 600;
  color: var(--text-secondary);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
  white-space: nowrap;
}
.tr {
  cursor: pointer;
  transition: background var(--transition);
}
.tr:hover {
  background: var(--bg-hover);
}
.td {
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border-subtle);
  color: var(--text-primary);
}
.td--empty {
  text-align: center;
  padding: var(--space-8) var(--space-4);
}
.table--striped .tr:nth-child(even) {
  background: var(--bg-elevated);
}
.table--compact .th,
.table--compact .td {
  padding: var(--space-1) var(--space-2);
  font-size: var(--text-sm);
}
</style>
