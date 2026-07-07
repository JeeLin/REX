<template>
  <div v-if="error" class="error-boundary">
    <div class="error-boundary-icon">⚠</div>
    <h2 class="error-boundary-title">{{ t('errorBoundary.title') }}</h2>
    <p class="error-boundary-message">{{ error.message }}</p>
    <button class="btn btn-primary" @click="retry">{{ t('errorBoundary.retry') }}</button>
  </div>
  <slot v-else />
</template>

<script setup lang="ts">
import { ref, onErrorCaptured } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const error = ref<Error | null>(null)

onErrorCaptured((err: Error) => {
  error.value = err
  return false // 阻止错误继续向上传播
})

function retry() {
  error.value = null
  window.location.reload()
}
</script>

<style scoped>
.error-boundary {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  padding: 2rem;
  text-align: center;
}

.error-boundary-icon {
  font-size: 4rem;
  margin-bottom: 1rem;
}

.error-boundary-title {
  font-size: 1.5rem;
  font-weight: 600;
  margin-bottom: 0.5rem;
  color: var(--text-primary);
}

.error-boundary-message {
  font-size: 0.875rem;
  color: var(--text-secondary);
  margin-bottom: 1.5rem;
  max-width: 400px;
  word-break: break-word;
}
</style>
