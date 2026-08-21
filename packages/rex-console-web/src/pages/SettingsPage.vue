<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { settingsApi, type Settings } from '@/api/settings'
import { useUpdateStore } from '@/stores/update'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'
import Select from '@/components/ui/Select.vue'
import Switch from '@/components/ui/Switch.vue'

const { t, locale } = useI18n()
const updateStore = useUpdateStore()
const settings = ref<Settings>({
  theme: 'dark',
  language: 'zh',
  terminal_font: 'JetBrains Mono',
  terminal_font_size: '14',
  terminal_theme: 'default',
  terminal_opacity: 100,
  terminal_bg_image: 'none',
  session_timeout: 30,
  auto_update: true,
})
const loading = ref(true)
const saving = ref(false)
const saveMessage = ref('')
const activeTab = ref('appearance')
const contentRef = ref<HTMLElement>()
const autoUpdate = ref(true)
const autoUpdateSynced = ref(false)
watch(autoUpdate, (val) => {
  if (!autoUpdateSynced.value) return
  settingsApi.update({ auto_update: val })
})

const tabs = [
  { key: 'appearance', icon: '🎨', labelKey: 'settings.appearance' },
  { key: 'terminal', icon: '⌨', labelKey: 'settings.terminal' },
  { key: 'security', icon: '🔒', labelKey: 'settings.security' },
  { key: 'update', icon: '🔄', labelKey: 'settings.update' },
]

function scrollToSection(key: string) {
  activeTab.value = key
  const el = document.getElementById(`settings-${key}`)
  if (el) {
    el.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

// Password change
const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const changingPassword = ref(false)
const passwordError = ref('')
const passwordSuccess = ref('')

// Update functionality — uses global store to persist across page navigation

async function changePassword() {
  if (!currentPassword.value || !newPassword.value) return
  if (newPassword.value !== confirmPassword.value) {
    passwordError.value = t('settings.passwordMismatch')
    return
  }
  passwordError.value = ''
  passwordSuccess.value = ''
  changingPassword.value = true
  try {
    await settingsApi.changePassword(currentPassword.value, newPassword.value)
    passwordSuccess.value = t('settings.passwordChanged')
    currentPassword.value = ''
    newPassword.value = ''
    confirmPassword.value = ''
    setTimeout(() => passwordSuccess.value = '', 3000)
  } catch (e: unknown) {
    passwordError.value = e instanceof Error ? e.message : t('settings.passwordChangeFailed')
  } finally {
    changingPassword.value = false
  }
}

onMounted(async () => {
  try {
    const remote = await settingsApi.get()
    // Merge remote settings, preserving frontend-only fields
    settings.value = {
      ...remote,
      session_timeout: parseInt(localStorage.getItem('rex-session-timeout') || '30', 10),
    }
    document.documentElement.dataset.theme = settings.value.theme === 'dark' ? undefined : settings.value.theme
    localStorage.setItem('rex-theme', settings.value.theme)
    // Sync i18n locale from saved settings
    if (settings.value.language) {
      locale.value = settings.value.language as 'zh' | 'en'
      localStorage.setItem('rex-lang', settings.value.language)
    }
    // Load auto_update from backend
    autoUpdate.value = remote.auto_update !== false
    autoUpdateSynced.value = true
    // Check for updates on mount
    await updateStore.checkForUpdate()
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
  try {
    await settingsApi.update({ ...settings.value })
    localStorage.setItem('rex-theme', settings.value.theme)
    localStorage.setItem('rex-lang', settings.value.language)
    const terminalSettings = {
      theme: settings.value.terminal_theme,
      fontFamily: settings.value.terminal_font,
      fontSize: parseInt(settings.value.terminal_font_size, 10) || 14,
      opacity: settings.value.terminal_opacity,
      backgroundImage: settings.value.terminal_bg_image,
    }
    localStorage.setItem('rex-terminal-settings', JSON.stringify(terminalSettings))
    window.dispatchEvent(new CustomEvent('terminal-settings-changed', { detail: terminalSettings }))
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
  <div class="settings-layout">
    <!-- 左侧导航 -->
    <nav class="settings-nav">
      <h1 class="page-title">{{ t('settings.title') }}</h1>
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="settings-nav-item"
        :class="{ 'active': activeTab === tab.key }"
        @click="scrollToSection(tab.key)"
      >
        <span class="nav-icon">{{ tab.icon }}</span>
        <span>{{ t(tab.labelKey) }}</span>
      </button>
    </nav>

    <!-- 右侧内容 -->
    <div class="settings-content" ref="contentRef">
      <Card id="settings-appearance" class="settings-section">
        <h2 class="section-title">{{ t('settings.appearance') }}</h2>
        <div class="form-group">
          <label class="form-label">{{ t('settings.theme') }}</label>
          <Select
            v-model="settings.theme"
            :options="[
              { label: t('settings.dark'), value: 'dark' },
              { label: t('settings.light'), value: 'light' },
              { label: t('settings.highContrast'), value: 'high-contrast' },
            ]"
          />
        </div>
        <div class="form-group">
          <label class="form-label">{{ t('settings.language') }}</label>
          <Select
            v-model="settings.language"
            :options="[
              { label: t('settings.langZh'), value: 'zh' },
              { label: t('settings.langEn'), value: 'en' },
            ]"
            @update:model-value="onLanguageChange"
          />
        </div>
      </Card>

      <Card id="settings-terminal" class="settings-section">
        <h2 class="section-title">{{ t('settings.terminal') }}</h2>
        <div class="form-group">
          <label class="form-label">{{ t('settings.fontFamily') }}</label>
          <Input v-model="settings.terminal_font" />
        </div>
        <div class="form-group">
          <label class="form-label">{{ t('settings.fontSize') }}</label>
          <Input v-model="settings.terminal_font_size" type="number" />
        </div>
        <div class="form-group">
          <label class="form-label">{{ t('settings.terminalTheme') }}</label>
          <Select
            v-model="settings.terminal_theme"
            :options="[
              { label: t('settings.terminalThemeDefault'), value: 'default' },
              { label: t('settings.terminalThemeUbuntu'), value: 'ubuntu' },
              { label: t('settings.terminalThemeSolarized'), value: 'solarized-dark' },
            ]"
          />
        </div>
        <div class="form-group">
          <label class="form-label">{{ t('settings.bgOpacity') }}</label>
          <Input :model-value="String(settings.terminal_opacity)" @update:model-value="settings.terminal_opacity = Number($event)" type="number" />
        </div>
      </Card>

      <Card id="settings-security" class="settings-section">
        <h2 class="section-title">{{ t('settings.security') }}</h2>
        <div class="form-group">
          <label class="form-label">{{ t('settings.sessionTimeout') }}</label>
          <Select
            v-model.number="settings.session_timeout"
            :options="[
              { label: `15 ${t('settings.minutes')}`, value: 15 },
              { label: `30 ${t('settings.minutes')}`, value: 30 },
              { label: `60 ${t('settings.minutes')}`, value: 60 },
              { label: `120 ${t('settings.minutes')}`, value: 120 },
            ]"
          />
        </div>
        <!-- Password Change -->
        <div class="password-section">
          <h3 class="subsection-title">{{ t('settings.changePassword') }}</h3>
          <div class="form-group">
            <label class="form-label">{{ t('settings.currentPassword') }}</label>
            <Input v-model="currentPassword" type="password" autocomplete="current-password" />
          </div>
          <div class="form-group">
            <label class="form-label">{{ t('settings.newPassword') }}</label>
            <Input v-model="newPassword" type="password" autocomplete="new-password" />
          </div>
          <div class="form-group">
            <label class="form-label">{{ t('settings.confirmPassword') }}</label>
            <Input v-model="confirmPassword" type="password" autocomplete="new-password" />
          </div>
          <div v-if="passwordError" class="save-message error">{{ passwordError }}</div>
          <div v-if="passwordSuccess" class="save-message">{{ passwordSuccess }}</div>
          <Button
            variant="secondary"
            size="sm"
            :loading="changingPassword"
            :disabled="!currentPassword || !newPassword || newPassword !== confirmPassword"
            @click="changePassword"
          >
            {{ t('settings.updatePassword') }}
          </Button>
        </div>
      </Card>

      <!-- Update Section -->
      <Card id="settings-update" class="settings-section">
        <h2 class="section-title">{{ t('settings.update') }}</h2>
        <div class="form-group">
          <label class="form-label">{{ t('settings.autoUpdate') }}</label>
          <Switch v-model="autoUpdate" size="sm" />
        </div>
        <div v-if="updateStore.updateLoading" class="update-progress">
          <p class="update-status">{{ updateStore.updateStatusText }}</p>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: updateStore.updateProgress + '%' }"></div>
          </div>
        </div>
        <div v-else class="update-info">
          <div class="update-current">
            <span class="label">{{ t('settings.currentVersion') }}:</span>
            <span class="value">{{ updateStore.currentVersion }}</span>
          </div>
          <div v-if="updateStore.hasUpdate" class="update-available">
            <div class="update-latest">
              <span class="label">{{ t('settings.latestVersion') }}:</span>
              <span class="value">{{ updateStore.latestVersion }}</span>
            </div>
            <Button
              variant="primary"
              :loading="updateStore.updating"
              @click="updateStore.triggerUpdate"
            >
              {{ t('settings.updateNow') }}
            </Button>
          </div>
          <div v-else class="update-up-to-date">
            <p>{{ t('settings.upToDate') }}</p>
          </div>
          <div v-if="updateStore.updateError" class="update-error">
            <p>{{ updateStore.updateError }}</p>
            <Button variant="secondary" @click="updateStore.rollbackUpdate">
              {{ t('settings.rollback') }}
            </Button>
          </div>
        </div>
      </Card>

      <div class="save-bar">
        <span v-if="saveMessage" class="save-message" :class="{ error: saveMessage.includes('failed') }">{{ saveMessage }}</span>
        <Button variant="primary" :loading="saving" @click="saveSettings">{{ t('settings.saveSettings') }}</Button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 三栏布局：左侧导航 + 右侧两列内容 */
.settings-layout {
  display: flex;
  gap: var(--space-6);
  height: 100%;
}
.settings-nav {
  width: 200px;
  flex-shrink: 0;
  padding-right: var(--space-4);
  border-right: 1px solid var(--border);
}
.settings-nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: var(--space-2) var(--space-3);
  border-radius: 6px;
  font-size: var(--text-sm);
  color: var(--text-secondary);
  cursor: pointer;
  background: none;
  border: none;
  text-align: left;
  transition: all var(--transition);
}
.settings-nav-item:hover { background: var(--bg-hover); color: var(--text-primary); }
.settings-nav-item.active { background: var(--bg-elevated); color: var(--text-primary); font-weight: 500; }
.settings-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-4);
  overflow-y: auto;
}
.settings-section { margin-bottom: 0; }
.page-title { font-size: var(--text-xl); font-weight: 700; color: var(--text-primary); margin-bottom: var(--space-4); }
.section-title { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-4); }
.form-group { margin-bottom: var(--space-3); }
.form-label { display: block; font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-1); }
.form-input {
  width: 100%; background: var(--bg-deep); border: 1px solid var(--border); border-radius: 6px;
  padding: 8px 12px; color: var(--text-primary); font-size: var(--text-sm); outline: none;
}
.form-input:focus { border-color: var(--accent); }
.save-bar { display: flex; align-items: center; justify-content: flex-end; gap: var(--space-3); margin-top: var(--space-4); padding: var(--space-4); background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-lg); position: sticky; bottom: 0; }
.save-message { font-size: var(--text-sm); color: var(--success); }
.save-message.error { color: var(--danger); }
.password-section { margin-top: var(--space-4); padding-top: var(--space-4); border-top: 1px solid var(--border); }
.subsection-title { font-size: var(--text-sm); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-3); }
.update-progress { padding: var(--space-3) 0; }
.update-status { font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-2); }
.progress-bar { height: 4px; background: var(--bg-deep); border-radius: 2px; overflow: hidden; }
.progress-fill { height: 100%; background: var(--accent); transition: width 0.3s; }
.update-info { padding: var(--space-3) 0; }
.update-current { margin-bottom: var(--space-2); }
.update-available { display: flex; align-items: center; justify-content: space-between; margin-top: var(--space-3); }
.update-latest { display: flex; align-items: center; gap: var(--space-2); }
.update-up-to-date { color: var(--text-secondary); font-size: var(--text-sm); }
.update-error { margin-top: var(--space-3); padding: var(--space-3); background: var(--danger-soft); border-radius: var(--radius); border: 1px solid rgba(248, 81, 73, 0.3); }
.update-error p { color: var(--danger); font-size: var(--text-sm); margin-bottom: var(--space-2); }
.label { color: var(--text-secondary); font-size: var(--text-sm); }
.value { color: var(--text-primary); font-size: var(--text-sm); font-weight: 500; }

/* 移动端适配：单列布局 */
@media (max-width: 768px) {
  .settings-layout {
    flex-direction: column;
    gap: var(--space-3);
  }
  .settings-nav {
    width: 100%;
    flex-direction: row;
    overflow-x: auto;
    border-right: none;
    border-bottom: 1px solid var(--border);
    padding-right: 0;
    padding-bottom: var(--space-2);
    gap: var(--space-1);
  }
  .settings-nav-item {
    white-space: nowrap;
    padding: var(--space-2);
  }
  .settings-content {
    grid-template-columns: 1fr;
  }
}
</style>
