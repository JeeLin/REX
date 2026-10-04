<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import Button from '@/components/ui/Button.vue'

const { t } = useI18n()

const props = defineProps<{
  path: string
  syncBrowsing: boolean
}>()

const emit = defineEmits<{
  'go-up': []
  'toggle-sync': []
  'upload': []
  'refresh': []
}>()
</script>

<template>
  <div class="ptb">
    <Button variant="ghost" icon :title="t('files.up')" @click="emit('go-up')">↑</Button>
    <span class="pp mono">{{ props.path }}</span>
    <Button
      variant="ghost"
      icon
      :class="{ 'pb--active': props.syncBrowsing }"
      :title="t('files.syncBrowsing')"
      @click="emit('toggle-sync')"
    >
      🔗
    </Button>

    <Button variant="ghost" icon :title="t('files.upload')" @click="emit('upload')">⬆</Button>
    <Button variant="ghost" icon :title="t('files.refresh')" @click="emit('refresh')">↻</Button>
  </div>
</template>

<style scoped>
.ptb {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-2);
  border-bottom: 1px solid var(--border);
  background: var(--bg-surface);
}
.pb--active {
  color: var(--accent);
  background: var(--accent-soft);
}
.pp {
  flex: 1;
  font-size: var(--text-xs);
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
