<template>
  <div class="settings-page">
    <h1>{{ t('nav.settings') }}</h1>
    <UpdateSection />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import UpdateSection from '@/features/settings/UpdateSection.vue'

const { t } = useI18n()
</script>

<style scoped>
.settings-page {
  padding: var(--sp-lg);
  max-width: 800px;
}

.settings-page h1 {
  margin: 0 0 var(--sp-lg);
  font-size: var(--fs-xl);
  color: var(--text-primary);
}
</style>
