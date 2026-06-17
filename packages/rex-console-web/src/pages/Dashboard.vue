<template>
  <div class="dashboard-page">
    <h1>{{ t('nav.dashboard') }}</h1>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
</script>

<style scoped>
.dashboard-page {
  padding: var(--sp-xl);
  background: var(--bg-deep);
  color: var(--text-primary);
  min-height: 100vh;
}
</style>
