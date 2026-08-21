<script setup lang="ts">
import { ref, onErrorCaptured } from 'vue'
import { useI18n } from 'vue-i18n'
import Button from './ui/Button.vue'

const { t } = useI18n()
const error = ref<Error | null>(null)

onErrorCaptured((err) => {
  error.value = err instanceof Error ? err : new Error(String(err))
  return false // 阻止错误继续传播
})

function retry() {
  error.value = null
}
</script>

<template>
  <div v-if="error" class="error-boundary">
    <div class="error-boundary-content">
      <div class="error-icon">⚠</div>
      <h2 class="error-title">{{ t('errorBoundary.title') }}</h2>
      <p class="error-message muted">{{ error.message }}</p>
      <Button variant="primary" @click="retry">{{ t('errorBoundary.retry') }}</Button>
    </div>
  </div>
  <slot v-else />
</template>

<style scoped>
.error-boundary {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  background: var(--bg-deep);
  padding: var(--space-4);
}
.error-boundary-content {
  text-align: center;
  max-width: 400px;
}
.error-icon {
  font-size: 48px;
  margin-bottom: var(--space-4);
}
.error-title {
  font-size: var(--text-lg);
  margin-bottom: var(--space-2);
}
.error-message {
  font-size: var(--text-sm);
  margin-bottom: var(--space-4);
  word-break: break-word;
}
</style>
