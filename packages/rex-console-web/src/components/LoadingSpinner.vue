<template>
  <div class="loading-spinner-wrap" :class="`size-${size}`">
    <span class="spinner"></span>
    <span v-if="text" class="loading-text">{{ text }}</span>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  size?: 'sm' | 'md' | 'lg'
  text?: string
}>(), {
  size: 'md',
})
</script>

<style scoped>
.loading-spinner-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-md);
  padding: var(--sp-3xl);
  color: var(--text-secondary);
}

.spinner {
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  flex-shrink: 0;
}

.size-sm .spinner { width: 16px; height: 16px; }
.size-md .spinner { width: 28px; height: 28px; }
.size-lg .spinner { width: 40px; height: 40px; }

.loading-text {
  font-size: var(--fs-sm);
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
