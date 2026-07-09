<template>
  <div class="code-block" :data-block-id="blockId">
    <pre class="code-content"><code>{{ content || t('notebooks.editor.placeholder.code') }}</code></pre>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  blockId: string
  content: string
}>()

const { t } = useI18n()
</script>

<style scoped>
.code-block {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--sp-md);
  overflow-x: auto;
}

.code-content {
  margin: 0;
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  line-height: var(--lh-relaxed);
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
