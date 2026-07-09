<template>
  <div class="notebook-editor-page">
    <div class="editor-header">
      <button class="btn btn-ghost btn-sm" @click="goBack">← {{ t('common.back') }}</button>
      <h1 class="editor-title">{{ notebook?.title || '...' }}</h1>
      <span v-if="isSaving" class="save-indicator">{{ t('common.saving') }}</span>
    </div>
    <div v-if="loading" class="editor-loading">
      <LoadingSpinner />
    </div>
    <div v-else-if="error" class="editor-error">
      <ErrorState :message="error" @retry="loadNotebook" />
    </div>
    <NotebookEditor
      v-else-if="notebook"
      :notebook="notebook"
      @saved="isSaving = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { getNotebook } from '../api/notebook'
import type { NotebookWithBlocks } from '../api/notebook'
import NotebookEditor from '../components/notebook/NotebookEditor.vue'
import LoadingSpinner from '../components/LoadingSpinner.vue'
import ErrorState from '../components/ErrorState.vue'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()

const notebook = ref<NotebookWithBlocks | null>(null)
const loading = ref(true)
const error = ref<string | null>(null)
const isSaving = ref(false)

function goBack() {
  router.push('/notebooks')
}

async function loadNotebook() {
  const id = route.params.id as string
  loading.value = true
  error.value = null
  try {
    notebook.value = await getNotebook(id)
  } catch (e) {
    console.error('Failed to load notebook:', e)
    error.value = t('common.requestFailed')
  } finally {
    loading.value = false
  }
}

onMounted(loadNotebook)
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

.save-indicator {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.editor-loading {
  display: flex;
  justify-content: center;
  padding: 48px;
}

.editor-error {
  padding: 48px;
}
</style>
