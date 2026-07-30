<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { settingsApi, type Settings, updateApi, type UpdateInfo, type UpdateStatus } from '@/api/settings'
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
  session_timeout: 30,
})
const loading = ref(true)
const saving = ref(false)
const saveMessage = ref('')
const activeTab = ref('appearance')

const tabs = [
  { key: 'appearance', icon: '🎨', labelKey: 'settings.appearance' },
  { key: 'terminal', icon: '⌨', labelKey: 'settings.terminal' },
  { key: 'security', icon: '🔒', labelKey: 'settings.security' },
  { key: 'update', icon: '🔄', labelKey: 'settings.update' },
]

// Password change
const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const changingPassword = ref(false)
const passwordError = ref('')
const passwordSuccess = ref('')

// Update functionality
const currentVersion = ref(__APP_VERSION__)
const latestVersion = ref('')
const hasUpdate = ref(false)
const updateLoading = ref(false)
const updating = ref(false)
const updateError = ref('')
const updateStatusText = ref('')
const updateProgress = ref(0)

// Version should come from backend in real implementation
async function checkForUpdate() {
  updateLoading.value = true
  updateError.value = ''
  try {
    const info: UpdateInfo = await updateApi.check()
    latestVersion.value = info.latest_version
    hasUpdate.value = info.has_update
  } catch (e: unknown) {
    updateError.value = e instanceof Error ? e.message : t('settings.updateCheckFailed')
  } finally {
    updateLoading.value = false
  }
}

async function triggerUpdate() {
  updating.value = true
  updateError.value = ''
  try {
    await updateApi.trigger()
    // Start polling for status
    const interval = setInterval(async () => {
      try {
        const status: UpdateStatus = await updateApi.status()
        updateStatusText.value = status.phase
        updateProgress.value = status.attempt * 25 // Simple progress indication
        if (status.phase === 'committed' || status.phase === 'rolled_back' || status.phase === 'failed') {
          clearInterval(interval)
          updating.value = false
          await checkForUpdate() // Refresh version info
        }
      } catch (e: unknown) {
        clearInterval(interval)
        updating.value = false
        updateError.value = e instanceof Error ? e.message : t('settings.updateStatusFailed')
      }
    }, 2000)
  } catch (e: unknown) {
    updating.value = false
    updateError.value = e instanceof Error ? e.message : t('settings.updateTriggerFailed')
  }
}

async function rollbackUpdate() {
  updating.value = true
  updateError.value = ''
  try {
    await updateApi.rollback()
    // Similar polling as trigger
    const interval = setInterval(async () => {
      try {
        const status: UpdateStatus = await updateApi.status()
        updateStatusText.value = status.phase
        if (status.phase === 'idle') {
          clearInterval(interval)
          updating.value = false
          await checkForUpdate()
        }
      } catch (e: unknown) {
        clearInterval(interval)
        updating.value = false
        updateError.value = e instanceof Error ? e.message : t('settings.rollbackStatusFailed')
      }
    }, 2000)
  } catch (e: unknown) {
    updating.value = false
    updateError.value = e instanceof Error ? e.message : t('settings.rollbackTriggerFailed')
  }
}

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
    // Check for updates on mount
    await checkForUpdate()
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
        @click="activeTab = tab.key"
      >
        <span class="nav-icon">{{ tab.icon }}</span>
        <span>{{ t(tab.labelKey) }}</span>
      </button>
    </nav>

    <!-- 右侧内容（两列） -->
    <div class="settings-content">
      <!-- 外观设置 -->
      <Card v-show="activeTab === 'appearance'" class="settings-section">
        <h2 class="section-title">{{ t('settings.appearance') }}</h2>
      <div class="form-group">
        <label class="form-label">{{ t('settings.theme') }}</label>
        <select v-model="settings.theme" class="form-input">
          <option value="dark">{{ t('settings.dark') }}</option>
          <option value="light">{{ t('settings.light') }}</option>
          <option value="high-contrast">{{ t('settings.highContrast') }}</option>
        </select>
      </div>
      <div class="form-group">
        <label class="form-label">{{ t('settings.language') }}</label>
        <select v-model="settings.language" class="form-input" @change="onLanguageChange">
          <option value="zh">{{ t('settings.langZh') }}</option>
          <option value="en">{{ t('settings.langEn') }}</option>
        </select>
      </div>
    </Card>

    <Card v-show="activeTab === 'terminal'" class="settings-section">
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
    </Card>

    <Card v-show="activeTab === 'security'" class="settings-section">
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
      <!-- Password Change -->
      <div class="password-section">
        <h3 class="subsection-title">{{ t('settings.changePassword') }}</h3>
        <div class="form-group">
          <label class="form-label">{{ t('settings.currentPassword') }}</label>
          <input v-model="currentPassword" type="password" class="form-input" autocomplete="current-password" />
        </div>
        <div class="form-group">
          <label class="form-label">{{ t('settings.newPassword') }}</label>
          <input v-model="newPassword" type="password" class="form-input" autocomplete="new-password" />
        </div>
        <div class="form-group">
          <label class="form-label">{{ t('settings.confirmPassword') }}</label>
          <input v-model="confirmPassword" type="password" class="form-input" autocomplete="new-password" />
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
    <Card v-show="activeTab === 'update'" class="settings-section">
      <h2 class="section-title">{{ t('settings.update') }}</h2>
      <div v-if="updateLoading" class="update-progress">
        <p class="update-status">{{ updateStatusText }}</p>
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: updateProgress + '%' }"></div>
        </div>
      </div>
      <div v-else class="update-info">
        <div class="update-current">
          <span class="label">{{ t('settings.currentVersion') }}:</span>
          <span class="value">{{ currentVersion }}</span>
        </div>
        <div v-if="hasUpdate" class="update-available">
          <div class="update-latest">
            <span class="label">{{ t('settings.latestVersion') }}:</span>
            <span class="value">{{ latestVersion }}</span>
          </div>
          <Button
            variant="primary"
            :loading="updating"
            @click="triggerUpdate"
          >
            {{ t('settings.updateNow') }}
          </Button>
        </div>
        <div v-else class="update-up-to-date">
          <p>{{ t('settings.upToDate') }}</p>
        </div>
        <div v-if="updateError" class="update-error">
          <p>{{ updateError }}</p>
          <Button variant="secondary" @click="rollbackUpdate">
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
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-4);
  align-content: start;
}
.settings-section { margin-bottom: 0; }
.page-title { font-size: var(--text-xl); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-4); }
.section-title { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-4); }
.form-group { margin-bottom: var(--space-3); }
.form-label { display: block; font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-1); }
.form-input {
  width: 100%; background: var(--bg-deep); border: 1px solid var(--border); border-radius: 6px;
  padding: 8px 12px; color: var(--text-primary); font-size: var(--text-sm); outline: none;
}
.form-input:focus { border-color: var(--accent); }
.save-bar { grid-column: 1 / -1; display: flex; align-items: center; justify-content: flex-end; gap: var(--space-3); margin-top: var(--space-4); }
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
.update-error { margin-top: var(--space-3); padding: var(--space-3); background: rgba(239, 68, 68, 0.1); border-radius: 6px; border: 1px solid rgba(239, 68, 68, 0.3); }
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
