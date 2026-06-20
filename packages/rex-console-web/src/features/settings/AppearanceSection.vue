<template>
  <SettingsSection>
    <template #header>{{ t('settings.appearance.title') }}</template>

    <!-- Theme -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.appearance.theme') }}</div>
        <div class="settings-row-desc">{{ t('settings.appearance.themeDesc') }}</div>
      </div>
      <div class="theme-options">
        <div
          class="theme-option"
          :class="{ active: userStore.theme === 'dark' }"
          @click="userStore.setTheme('dark')"
        >
          <div class="theme-preview" style="background:#0D1117;color:#E6EDF3">☾</div>
          <span class="theme-label">{{ t('settings.appearance.dark') }}</span>
        </div>
        <div
          class="theme-option"
          :class="{ active: userStore.theme === 'light' }"
          @click="userStore.setTheme('light')"
        >
          <div class="theme-preview" style="background:#F6F8FA;color:#24292F;border-color:#D0D7DE">☀</div>
          <span class="theme-label">{{ t('settings.appearance.light') }}</span>
        </div>
        <div
          class="theme-option"
          :class="{ active: userStore.theme === 'system' }"
          @click="userStore.setTheme('system')"
        >
          <div class="theme-preview" style="background:linear-gradient(135deg,#0D1117 50%,#F6F8FA 50%);color:#E6EDF3">◐</div>
          <span class="theme-label">{{ t('settings.appearance.system') }}</span>
        </div>
      </div>
    </div>
    <!-- Language -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.appearance.language') }}</div>
        <div class="settings-row-desc">{{ t('settings.appearance.languageDesc') }}</div>
      </div>
      <div class="lang-options">
        <div
          class="lang-option"
          :class="{ active: userStore.lang === 'zh' }"
          @click="switchLang('zh')"
        >
          简体中文
        </div>
        <div
          class="lang-option"
          :class="{ active: userStore.lang === 'en' }"
          @click="switchLang('en')"
        >
          English
        </div>
      </div>
    </div>
  </SettingsSection>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useUserStore, type Lang } from '@/stores/user'
import SettingsSection from './SettingsSection.vue'

const { t } = useI18n()
const userStore = useUserStore()

function switchLang(lang: Lang) {
  userStore.setLang(lang)
  location.reload()
}
</script>

<style scoped>
.theme-options {
  display: flex;
  gap: var(--sp-md);
}

.theme-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--sp-sm);
  cursor: pointer;
}

.theme-preview {
  width: 64px;
  height: 48px;
  border-radius: var(--radius-md);
  border: 2px solid var(--border);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--fs-xl);
  transition: border-color var(--transition-fast);
}

.theme-option.active .theme-preview {
  border-color: var(--accent);
}

.theme-option:hover .theme-preview {
  border-color: var(--text-muted);
}

.theme-option.active:hover .theme-preview {
  border-color: var(--accent);
}

.theme-label {
  font-size: var(--fs-xs);
  color: var(--text-secondary);
}

.lang-options {
  display: flex;
  gap: var(--sp-sm);
}

.lang-option {
  padding: var(--sp-sm) var(--sp-lg);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  font-size: var(--fs-sm);
  cursor: pointer;
  background: var(--bg-deep);
  color: var(--text-secondary);
  transition: all var(--transition-fast);
}

.lang-option.active {
  border-color: var(--accent);
  color: var(--text-primary);
  background: rgba(232,145,45,0.08);
}

.lang-option:hover:not(.active) {
  border-color: var(--text-muted);
  color: var(--text-primary);
}
</style>
