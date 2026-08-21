<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import Modal from '@/components/ui/Modal.vue'
import Input from '@/components/ui/Input.vue'
import Button from '@/components/ui/Button.vue'
import {
  listSavedQueries,
  upsertSavedQuery,
  deleteSavedQuery,
  type SavedQuery,
} from '@/api/sql'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  open: [query: SavedQuery]
}>()

const { t } = useI18n()

const queries = ref<SavedQuery[]>([])
const loading = ref(false)
const errorMsg = ref('')

// rename state
const renaming = ref<SavedQuery | null>(null)
const renameValue = ref('')
const savingRename = ref(false)

async function load() {
  loading.value = true
  errorMsg.value = ''
  try {
    queries.value = await listSavedQueries()
  } catch (e) {
    errorMsg.value = (e as Error).message
  } finally {
    loading.value = false
  }
}

watch(
  () => props.open,
  (open) => {
    if (open) load()
  },
)

function close() {
  emit('update:open', false)
}

function onOpen(q: SavedQuery) {
  emit('open', q)
  close()
}

function startRename(q: SavedQuery) {
  renaming.value = q
  renameValue.value = q.name
}

async function confirmRename() {
  if (!renaming.value) return
  const name = renameValue.value.trim()
  if (!name) return
  savingRename.value = true
  try {
    await upsertSavedQuery({ id: renaming.value.id, name, sql: renaming.value.sql })
    renaming.value = null
    await load()
  } catch (e) {
    errorMsg.value = (e as Error).message
  } finally {
    savingRename.value = false
  }
}

async function onDelete(q: SavedQuery) {
  if (!confirm(`${t('sql.deleteSavedQuery')} 「${q.name}」?`)) return
  try {
    await deleteSavedQuery(q.id)
    await load()
  } catch (e) {
    errorMsg.value = (e as Error).message
  }
}
</script>

<template>
  <Modal :model-value="open" :title="t('sql.savedQueries')" width="440px" @update:model-value="close">
    <div class="saved-query-list">
      <p v-if="errorMsg" class="saved-query-error">{{ errorMsg }}</p>
      <p v-else-if="loading" class="saved-query-hint">{{ t('sql.loading') }}</p>
      <p v-else-if="!queries.length" class="saved-query-hint">{{ t('sql.noSavedQueries') }}</p>
      <ul v-else class="saved-query-items">
        <li v-for="q in queries" :key="q.id" class="saved-query-item">
          <div class="saved-query-meta">
            <span class="saved-query-name">{{ q.name }}</span>
            <span v-if="q.db_type" class="saved-query-type">{{ q.db_type }}</span>
          </div>
          <div class="saved-query-actions">
            <Button size="sm" variant="ghost" :aria-label="t('sql.openSavedQuery')" @click="onOpen(q)">
              {{ t('sql.openSavedQuery') }}
            </Button>
            <Button size="sm" variant="ghost" :aria-label="t('sql.renameQuery')" @click="startRename(q)">
              {{ t('sql.renameQuery') }}
            </Button>
            <Button size="sm" variant="danger" :aria-label="t('sql.deleteSavedQuery')" @click="onDelete(q)">
              {{ t('sql.deleteSavedQuery') }}
            </Button>
          </div>
        </li>
      </ul>
    </div>

    <template #footer>
      <Button variant="secondary" @click="close">{{ t('sql.close') }}</Button>
    </template>

    <Modal
      v-if="renaming"
      :model-value="!!renaming"
      :title="t('sql.renameQuery')"
      width="380px"
      @update:model-value="renaming = null"
    >
      <Input
        v-model="renameValue"
        :placeholder="t('sql.savedQueryNamePlaceholder')"
        @keydown.enter="confirmRename"
      />
      <template #footer>
        <Button variant="ghost" :disabled="savingRename" @click="renaming = null">{{ t('cancel') }}</Button>
        <Button variant="primary" :loading="savingRename" :disabled="!renameValue.trim()" @click="confirmRename">
          {{ t('sql.saveQuery') }}
        </Button>
      </template>
    </Modal>
  </Modal>
</template>

<style scoped>
.saved-query-list {
  max-height: 60vh;
  overflow-y: auto;
}
.saved-query-hint {
  color: var(--text-muted);
  padding: var(--space-2) 0;
}
.saved-query-error {
  color: var(--danger);
}
.saved-query-items {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.saved-query-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
}
.saved-query-meta {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}
.saved-query-name {
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.saved-query-type {
  font-size: 0.75rem;
  color: var(--text-muted);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 0 var(--space-1);
}
.saved-query-actions {
  display: flex;
  gap: var(--space-1);
  flex-shrink: 0;
}
</style>
