<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { settingsApi, type Settings } from '@/api/settings'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'

const { t } = useI18n()
const settings = ref<Settings>({
  theme: 'dark',
  language: 'zh',
  terminal_font: 'JetBrains Mono',
  terminal_font_size: '14',
})
const loading = ref(true)
const saving = ref(false)
const saveMessage = ref('')

onMounted(async () => {
  try {
    settings.value = await settingsApi.get()
    // 应用主题
    document.documentElement.dataset.theme = settings.value.theme === 'light' ? 'light' : undefined
  } catch {
    // ignore
  } finally {
    loading.value = false
  }
})

async function saveSettings() {
  saving.value = true
  saveMessage.value = ''
  try {
    await settingsApi.update(settings.value)
    // 应用主题
    document.documentElement.dataset.theme = settings.value.theme === 'light' ? 'light' : undefined
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
        <select v-model="settings.language" class="form-input">
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
