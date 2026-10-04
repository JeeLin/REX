<script setup lang="ts">
import { computed } from 'vue'
import { fmtSize } from '@/features/files/format'
import type { FileEntry } from '@/api/files'

const props = withDefaults(
  defineProps<{
    entry: FileEntry
    selected: boolean
    isRenaming: boolean
    renameValue: string
    /** Show S3-only columns (storage class + ACL). */
    showStorageClass: boolean
  }>(),
  {
    selected: false,
    isRenaming: false,
    renameValue: '',
    showStorageClass: false,
  },
)

const emit = defineEmits<{
  select: [name: string, ev: MouseEvent]
  activate: [entry: FileEntry]
  context: [ev: MouseEvent, entry: FileEntry]
  'update:renameValue': [value: string]
  'rename-submit': []
  'rename-cancel': []
  'drag-start': [ev: DragEvent, name: string]
  'drag-end': []
}>()

const icon = computed(() => (props.entry.is_dir ? '📁' : '📄'))
const sizeLabel = computed(() =>
  props.entry.is_dir ? '-' : fmtSize(props.entry.size),
)
</script>

<template>
  <div
    class="fr"
    :class="{ 'fr--sel': props.selected }"
    draggable="true"
    @dragstart="emit('drag-start', $event, props.entry.name)"
    @dragend="emit('drag-end')"
    @click="emit('select', props.entry.name, $event)"
    @dblclick="!props.isRenaming && emit('activate', props.entry)"
    @contextmenu="emit('context', $event, props.entry)"
  >
    <template v-if="!isRenaming">
      <span class="cn">
        <span class="fi">{{ icon }}</span> {{ props.entry.name }}
      </span>
    </template>
    <input
      v-else
      :value="renameValue"
      class="fp-rename-input"
      autofocus
      @input="$emit('update:renameValue', ($event.target as HTMLInputElement).value)"
      @blur="emit('rename-cancel')"
      @keydown.enter="emit('rename-submit')"
      @keydown.escape="emit('rename-cancel')"
      @click.stop
      @keydown.stop
    />
    <span class="cs mu">{{ sizeLabel }}</span>
    <span class="cm mu">{{ props.entry.modified || '-' }}</span>
    <span v-if="showStorageClass" class="csc mu">{{ props.entry.storage_class || '-' }}</span>
    <span v-if="showStorageClass" class="csc mu">{{ props.entry.acl || '-' }}</span>
  </div>
</template>

<style scoped>
.fr {
  display: flex;
  padding: var(--space-1) var(--space-3);
  font-size: var(--text-sm);
  cursor: pointer;
}
.fr:hover {
  background: var(--bg-hover);
}
.fr--sel {
  background: var(--bg-hover);
  border-left: 2px solid var(--accent);
}
.fh {
  font-weight: 600;
  color: var(--text-muted);
  font-size: var(--text-xs);
  text-transform: uppercase;
  cursor: default;
}
.fh:hover {
  background: none;
}
.cn {
  flex: 1;
  display: flex;
  align-items: center;
  gap: var(--space-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cs {
  width: 80px;
  text-align: right;
}
.cm {
  width: 140px;
  text-align: right;
}
.csc {
  width: 100px;
  text-align: right;
}
.fi {
  font-size: 14px;
}
.mu {
  color: var(--text-muted);
}
.fp-rename-input {
  flex: 1;
  background: var(--bg-deep);
  border: 1px solid var(--accent);
  border-radius: 2px;
  color: var(--text-primary);
  font-size: var(--text-sm);
  padding: 0 4px;
  outline: none;
  min-width: 0;
}
</style>
