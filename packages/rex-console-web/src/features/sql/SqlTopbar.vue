<template>
  <div class="sql-topbar">
    <div class="sql-topbar-breadcrumb">
      <span>{{ resourceName }}</span>
      <span class="sep">›</span>
      <span class="current">{{ protocol.toUpperCase() }}</span>
    </div>
    <div class="sql-topbar-spacer"></div>
    <div class="db-selector">
      <label>{{ t('sql.database') }}:</label>
      <select :value="selectedDb" @change="$emit('update:selectedDb', ($event.target as HTMLSelectElement).value)">
        <option v-for="db in databases" :key="db.name" :value="db.name">{{ db.name }}</option>
      </select>
    </div>
    <button class="btn btn-ghost btn-sm" :title="t('common.refresh')" @click="$emit('refresh')">↻</button>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { DatabaseInfo } from '@/api/sql'

const { t } = useI18n()

defineProps<{
  resourceName: string
  protocol: string
  databases: DatabaseInfo[]
  selectedDb: string
}>()

defineEmits<{
  'update:selectedDb': [value: string]
  'refresh': []
}>()
</script>

<style scoped>
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

.sql-topbar-breadcrumb {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  font-size: var(--fs-sm);
  font-family: var(--font-mono);
  color: var(--text-secondary);
}

.sql-topbar-breadcrumb .sep { color: var(--text-muted); }
.sql-topbar-breadcrumb .current { color: var(--text-primary); font-weight: 500; }

.sql-topbar-spacer { flex: 1; }

.db-selector {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.db-selector label {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
}

.db-selector select {
  padding: 2px var(--sp-sm);
  padding-right: 24px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-deep);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  outline: none;
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='10' viewBox='0 0 10 10'%3E%3Cpath fill='%238B949E' d='M5 7L0 2h10z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 6px center;
}
</style>
