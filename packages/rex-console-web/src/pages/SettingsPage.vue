<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { settingsApi, type Settings } from '@/api/settings'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'

const { t, locale } = useI18n()
const settings = ref<Settings>({
  theme: 'dark',
  language: 'zh',
  terminal_font: 'JetBrains Mono',
  terminal_font_size: '14',
  terminal_theme: 'default',
  terminal_opacity: 100,
  terminal_bg_image: 'none',
  session_timeout: parseInt(localStorage.getItem('rex-session-timeout') || '30', 10),
})
const loading = ref(true)
const saving = ref(false)
const saveMessage = ref('')

onMounted(async () => {
  try {
    const remote = await settingsApi.get()
    // Merge remote settings, preserving frontend-only fields
    settings.value = {
      ...remote,
      session_timeout: parseInt(localStorage.getItem('rex-session-timeout') || '30', 10),
    }
    document.documentElement.dataset.theme = settings.value.theme === 'light' ? 'light' : undefined
    localStorage.setItem('rex-theme', settings.value.theme)
    // Sync i18n locale from saved settings
    if (settings.value.language) {
      locale.value = settings.value.language as 'zh' | 'en'
      localStorage.setItem('rex-lang', settings.value.language)
    }
  } catch {
    // ignore
  } finally {
    loading.value = false
  }
})

function onLanguageChange() {
  locale.value = settings.value.language as 'zh' | 'en'
  localStorage.setItem('rex-lang', settings.value.language)
}

async function saveSettings() {
  saving.value = true
  saveMessage.value = ''
  try {
    await settingsApi.update(settings.value)
    document.documentElement.dataset.theme = settings.value.theme === 'light' ? 'light' : undefined
    localStorage.setItem('rex-theme', settings.value.theme)
    // Persist language setting
    localStorage.setItem('rex-lang', settings.value.language)
    // Cache terminal settings for TerminalView to read on mount
    localStorage.setItem('rex-terminal-settings', JSON.stringify({
      theme: settings.value.terminal_theme,
      opacity: settings.value.terminal_opacity,
      backgroundImage: settings.value.terminal_bg_image,
    }))
    // Session timeout (localStorage only, frontend concern)
    localStorage.setItem('rex-session-timeout', String(settings.value.session_timeout))
    saveMessage.value = t('settings.saved')
    setTimeout(() => saveMessage.value = '', 2000)
  } catch (e: unknown) {
    saveMessage.value = e instanceof Error ? e.message : t('settings.saveFailed')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="settings-page">
    <h1 class="page-title">{{ t('settings.title') }}</h1>

    <Card class="settings-section">
      <h2 class="section-title">{{ t('settings.appearance') }}</h2>
      <div class="form-group">
        <label class="form-label">{{ t('settings.theme') }}</label>
        <select v-model="settings.theme" class="form-input">
          <option value="dark">{{ t('settings.dark') }}</option>
          <option value="light">{{ t('settings.light') }}</option>
        </select>
      </div>
      <div class="form-group">
        <label class="form-label">{{ t('settings.language') }}</label>
        <select v-model="settings.language" class="form-input" @change="onLanguageChange">
          <option value="zh">中文</option>
          <option value="en">English</option>
        </select>
      </div>
    </Card>

    <Card class="settings-section">
      <h2 class="section-title">{{ t('settings.terminal') }}</h2>
      <div class="form-group">
        <label class="form-label">{{ t('settings.fontFamily') }}</label>
        <input v-model="settings.terminal_font" type="text" class="form-input" />
      </div>
      <div class="form-group">
        <label class="form-label">{{ t('settings.fontSize') }}</label>
        <input v-model="settings.terminal_font_size" type="number" class="form-input" min="10" max="24" />
      </div>
      <div class="form-group">
        <label class="form-label">{{ t('settings.terminalTheme') }}</label>
        <select v-model="settings.terminal_theme" class="form-input">
          <option value="default">{{ t('settings.terminalThemeDefault') }}</option>
          <option value="ubuntu">{{ t('settings.terminalThemeUbuntu') }}</option>
          <option value="solarized-dark">{{ t('settings.terminalThemeSolarized') }}</option>
        </select>
      </div>
      <div class="form-group">
        <label class="form-label">{{ t('settings.bgOpacity') }}</label>
        <input v-model.number="settings.terminal_opacity" type="number" class="form-input" min="0" max="100" />
      </div>
      <div class="form-group">
        <label class="form-label">{{ t('settings.bgImage') }}</label>
        <select v-model="settings.terminal_bg_image" class="form-input">
          <option value="none">{{ t('settings.bgImageNone') }}</option>
          <option value="grid">{{ t('settings.bgImageGrid') }}</option>
          <option value="dots">{{ t('settings.bgImageDots') }}</option>
          <option value="gradient">{{ t('settings.bgImageGradient') }}</option>
        </select>
      </div>
    </Card>

    <Card class="settings-section">
      <h2 class="section-title">{{ t('settings.security') }}</h2>
      <div class="form-group">
        <label class="form-label">{{ t('settings.sessionTimeout') }}</label>
        <select v-model.number="settings.session_timeout" class="form-input">
          <option :value="15">15 {{ t('settings.minutes') }}</option>
          <option :value="30">30 {{ t('settings.minutes') }}</option>
          <option :value="60">60 {{ t('settings.minutes') }}</option>
          <option :value="120">120 {{ t('settings.minutes') }}</option>
        </select>
      </div>
    </Card>

    <div class="save-bar">
      <span v-if="saveMessage" class="save-message" :class="{ error: saveMessage.includes('failed') }">{{ saveMessage }}</span>
      <Button variant="primary" :loading="saving" @click="saveSettings">{{ t('settings.saveSettings') }}</Button>
    </div>
  </div>
</template>

<style scoped>
.settings-page { max-width: 600px; }
.page-title { font-size: var(--text-xl); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-6); }
.settings-section { margin-bottom: var(--space-4); }
.section-title { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-4); }
.form-group { margin-bottom: var(--space-3); }
.form-label { display: block; font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-1); }
.form-input {
  width: 100%; background: var(--bg-deep); border: 1px solid var(--border); border-radius: 6px;
  padding: 8px 12px; color: var(--text-primary); font-size: var(--text-sm); outline: none;
}
.form-input:focus { border-color: var(--accent); }
.save-bar { display: flex; align-items: center; justify-content: flex-end; gap: var(--space-3); margin-top: var(--space-4); }
.save-message { font-size: var(--text-sm); color: var(--success); }
.save-message.error { color: var(--danger); }
</style>
