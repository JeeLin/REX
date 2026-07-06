<template>
  <SettingsSection>
    <template #header>{{ t('settings.security.title') }}</template>

    <!-- Session Timeout -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.security.sessionTimeout') }}</div>
        <div class="settings-row-desc">{{ t('settings.security.sessionTimeoutDesc') }}</div>
      </div>
      <select class="form-select" :value="settingsStore.securitySettings.sessionTimeout" @change="setSessionTimeout">
        <option value="15">{{ t('settings.security.timeout15') }}</option>
        <option value="30">{{ t('settings.security.timeout30') }}</option>
        <option value="60">{{ t('settings.security.timeout60') }}</option>
        <option value="never">{{ t('settings.security.timeoutNever') }}</option>
      </select>
    </div>
    <!-- Config Encryption -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.security.configEncryption') }}</div>
        <div class="settings-row-desc">{{ t('settings.security.configEncryptionDesc') }}</div>
      </div>
      <div class="settings-toggle" :class="{ active: settingsStore.securitySettings.configEncryption }" @click="toggleConfigEncryption"></div>
    </div>
    <!-- Audit Log Toggle -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.security.auditLog') }}</div>
        <div class="settings-row-desc">{{ t('settings.security.auditLogDesc') }}</div>
      </div>
      <div class="settings-toggle" :class="{ active: settingsStore.securitySettings.auditEnabled }" @click="toggleAudit"></div>
    </div>
    <!-- View Audit Log (only when enabled) -->
    <div v-if="settingsStore.securitySettings.auditEnabled" class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.security.viewAuditLog') }}</div>
        <div class="settings-row-desc">{{ t('settings.security.viewAuditLogDesc') }}</div>
      </div>
      <router-link to="/audit-log" class="view-audit-link">
        {{ t('settings.security.viewAuditLogBtn') }}
      </router-link>
    </div>
  </SettingsSection>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import SettingsSection from './SettingsSection.vue'
import { useSettingsStore } from '@/stores/settings'

const { t } = useI18n()
const settingsStore = useSettingsStore()

function setSessionTimeout(e: Event) {
  settingsStore.updateSecuritySetting('sessionTimeout', Number((e.target as HTMLSelectElement).value))
}

function toggleAudit() {
  settingsStore.updateSecuritySetting('auditEnabled', !settingsStore.securitySettings.auditEnabled)
  window.dispatchEvent(new CustomEvent('audit-toggle', { detail: { enabled: settingsStore.securitySettings.auditEnabled } }))
}

function toggleConfigEncryption() {
  settingsStore.updateSecuritySetting('configEncryption', !settingsStore.securitySettings.configEncryption)
}
</script>

<style scoped>
.view-audit-link {
  color: var(--text-secondary);
  text-decoration: none;
  font-size: var(--fs-sm);
  padding: var(--sp-xs) var(--sp-sm);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.view-audit-link:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}
@media (max-width: 767px) {
  .settings-section {
    padding: var(--sp-md);
  }

  .settings-row {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-xs);
  }
}
</style>
