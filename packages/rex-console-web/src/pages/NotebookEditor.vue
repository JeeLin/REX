<template>
  <div class="notebook-editor-page">
    <div class="editor-header">
      <button class="btn btn-ghost btn-sm" @click="goBack">← {{ t('common.back') }}</button>
      <h1 class="editor-title">{{ notebook?.title || '...' }}</h1>
    </div>
    <div class="editor-placeholder">
      <p>Block-based editor — coming in subtask 3</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { getNotebook } from '../api/notebook'
import type { NotebookWithBlocks } from '../api/notebook'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()

const notebook = ref<NotebookWithBlocks | null>(null)

function goBack() {
  router.push('/notebooks')
}

onMounted(async () => {
  const id = route.params.id as string
  try {
    notebook.value = await getNotebook(id)
  } catch (e) {
    console.error('Failed to load notebook:', e)
  }
})
</script>

<style scoped>
.notebook-editor-page {
  padding: 24px;
  max-width: 960px;
  margin: 0 auto;
}

.editor-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
}

.editor-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.editor-placeholder {
  padding: 48px;
  text-align: center;
  color: var(--text-secondary);
  border: 1px dashed var(--border-color);
  border-radius: 8px;
}
</style>
