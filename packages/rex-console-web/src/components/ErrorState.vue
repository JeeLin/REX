<template>
  <div class="error-state">
    <span class="error-icon">⚠</span>
    <p class="error-message">{{ message }}</p>
    <button v-if="retry" class="btn btn-ghost btn-sm" @click="retry">
      {{ t('common.retry') }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  message: string
  retry?: () => void
}>()

const { t } = useI18n()
</script>

<style scoped>
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--sp-md);
  padding: var(--sp-3xl);
  text-align: center;
}

.error-icon {
  font-size: 28px;
  color: var(--danger);
  opacity: 0.8;
}

.error-message {
  color: var(--text-secondary);
  font-size: var(--fs-sm);
  max-width: 400px;
}
</style>
