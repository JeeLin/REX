<template>
  <div class="login-page">
    <h1>REX Hub</h1>
    <p>{{ t('auth.login') }}</p>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
</script>

<style scoped>
.login-page {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  background: var(--bg-deep);
  color: var(--text-primary);
}
</style>
