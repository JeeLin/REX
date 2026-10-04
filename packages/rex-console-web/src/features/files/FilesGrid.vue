<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import FilesList from '@/features/files/FilesList.vue'
import type { FileEntry, Side, FilesPanel } from '@/features/files/types'

const { t } = useI18n()

const props = withDefaults(
  defineProps<{
    side: Side
    panel: FilesPanel
    renamingId: string | null
    renameValue: string
    /** Show S3-only columns (storage class + ACL). */
    showStorageClass: boolean
  }>(),
  {
    renamingId: null,
    renameValue: '',
    showStorageClass: false,
  },
)

const emit = defineEmits<{
  select: [side: Side, name: string, ev: MouseEvent]
  activate: [side: Side, entry: FileEntry]
  context: [ev: MouseEvent, entry: FileEntry, side: Side]
  'update:renameValue': [value: string]
  'rename-submit': [side: Side]
  'rename-cancel': []
  'drag-start': [ev: DragEvent, side: Side, name: string]
  'drag-end': []
}>()

const isRenamingRow = (entry: FileEntry) =>
  props.renamingId === `${props.side}:${entry.name}`
</script>

<template>
  <div class="pf">
    <div class="fr fh">
      <span class="cn">{{ t('files.name') }}</span>
      <span class="cs">{{ t('files.size') }}</span>
      <span class="cm">{{ t('files.modified') }}</span>
      <span v-if="showStorageClass" class="csc">{{ t('files.storageClass') }}</span>
      <span v-if="showStorageClass" class="csc">{{ t('files.acl') }}</span>
    </div>
    <FilesList
      v-for="e in props.panel.entries"
      :key="e.name"
      :entry="e"
      :selected="props.panel.selected.has(e.name)"
      :is-renaming="isRenamingRow(e)"
      :rename-value="renameValue"
      :show-storage-class="showStorageClass"
      @select="(name, ev) => emit('select', props.side, name, ev)"
      @activate="emit('activate', props.side, $event)"
      @context="(ev, entry) => emit('context', ev, entry, props.side)"
      @update:rename-value="emit('update:renameValue', $event)"
      @rename-submit="emit('rename-submit', props.side)"
      @rename-cancel="emit('rename-cancel')"
      @drag-start="(ev, name) => emit('drag-start', ev, props.side, name)"
      @drag-end="emit('drag-end')"
    />
    <div v-if="!props.panel.loading && !props.panel.entries.length" class="pe">
      {{ t('files.empty') }}
    </div>
  </div>
</template>

<style scoped>
.pf {
  flex: 1;
  overflow-y: auto;
}
.pe {
  padding: var(--space-4);
  text-align: center;
  color: var(--text-muted);
  font-size: var(--text-sm);
}
</style>
