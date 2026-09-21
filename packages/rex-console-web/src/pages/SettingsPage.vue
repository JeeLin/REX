<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import { settingsApi, type Settings } from '@/api/settings'
import { useUpdateStore } from '@/stores/update'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'
import Select from '@/components/ui/Select.vue'
import Switch from '@/components/ui/Switch.vue'
import Badge from '@/components/ui/Badge.vue'
import { api } from '@/api/client'

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
  audit_logging: true,
})
const loading = ref(true)
const saving = ref(false)
const saveMessage = ref('')
const activeTab = ref('profile')
const contentRef = ref<HTMLElement>()

// Profile
const displayName = ref('admin')
const profileEmail = ref('admin@rex.local')

// Toggles
const autoUpdate = ref(true)
const autoUpdateSynced = ref(false)
watch(autoUpdate, (val) => {
  if (!autoUpdateSynced.value) return
  settingsApi.update({ auto_update: val })
})
const auditLogging = ref(true)
const auditLoggingSynced = ref(false)
watch(auditLogging, (val) => {
  if (!auditLoggingSynced.value) return
  settingsApi.update({ audit_logging: val })
})

// Theme
watch(() => settings.value.theme, (newTheme) => {
  const root = document.documentElement
  if (newTheme === 'dark') {
    delete root.dataset.theme
  } else if (newTheme === 'system') {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
    if (prefersDark) {
      delete root.dataset.theme
    } else {
      root.dataset.theme = 'light'
    }
  } else {
    root.dataset.theme = newTheme
  }
  localStorage.setItem('rex-theme', newTheme)
})

// Navigation sections
const sections = [
  { key: 'profile', icon: 'profile', labelKey: 'settings.profile' },
  { key: 'appearance', icon: 'appearance', labelKey: 'settings.appearance' },
  { key: 'terminal', icon: 'terminal', labelKey: 'settings.terminal' },
  { key: 'security', icon: 'security', labelKey: 'settings.security' },
  { key: 'updates', icon: 'updates', labelKey: 'settings.updates' },
  { key: 'data', icon: 'data', labelKey: 'settings.dataManagement' },
  { key: 'about', icon: 'about', labelKey: 'settings.about' },
]

function scrollToSection(key: string) {
  activeTab.value = key
  const el = document.getElementById(`settings-${key}`)
  if (el) {
    el.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

const sectionIds = sections.map(s => s.key)

function handleScroll() {
  const container = contentRef.value
  if (!container) return
  const scrollTop = container.scrollTop
  for (let i = sectionIds.length - 1; i >= 0; i--) {
    const el = document.getElementById(`settings-${sectionIds[i]!}`)
    if (el && el.offsetTop - 80 <= scrollTop) {
      activeTab.value = sectionIds[i]!
      return
    }
  }
  activeTab.value = 'profile'
}

onMounted(() => {
  contentRef.value?.addEventListener('scroll', handleScroll, { passive: true })
})
onBeforeUnmount(() => {
  contentRef.value?.removeEventListener('scroll', handleScroll)
})

// Password change
const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const changingPassword = ref(false)
const passwordError = ref('')
const passwordSuccess = ref('')

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

// Init
onMounted(async () => {
  displayName.value = localStorage.getItem('rex-display-name') || 'admin'
  profileEmail.value = localStorage.getItem('rex-profile-email') || 'admin@rex.local'
  try {
    const remote = await settingsApi.get()
    settings.value = {
      ...remote,
      session_timeout: parseInt(localStorage.getItem('rex-session-timeout') || '30', 10),
    }
    document.documentElement.dataset.theme = settings.value.theme === 'dark' ? undefined : settings.value.theme
    localStorage.setItem('rex-theme', settings.value.theme)
    if (settings.value.language) {
      locale.value = settings.value.language as 'zh' | 'en'
      localStorage.setItem('rex-lang', settings.value.language)
    }
    autoUpdate.value = remote.auto_update !== false
    autoUpdateSynced.value = true
    auditLogging.value = remote.audit_logging !== false
    auditLoggingSynced.value = true
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
    localStorage.setItem('rex-display-name', displayName.value)
    localStorage.setItem('rex-profile-email', profileEmail.value)
    saveMessage.value = t('settings.saved')
    setTimeout(() => saveMessage.value = '', 2000)
  } catch (e: unknown) {
    saveMessage.value = e instanceof Error ? e.message : t('settings.saveFailed')
  } finally {
    saving.value = false
  }
}

// Data Management
const exporting = ref(false)
const importing = ref(false)
const importFile = ref<File | null>(null)
const dataMessage = ref('')

async function exportData() {
  exporting.value = true
  dataMessage.value = ''
  try {
    const data = await api.get<{ version: string; environments: unknown[] }>('/environments/export')
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `rex-export-${new Date().toISOString().slice(0, 10)}.json`
    a.click()
    URL.revokeObjectURL(url)
    dataMessage.value = t('settings.exportSuccess')
    setTimeout(() => dataMessage.value = '', 3000)
  } catch (e: unknown) {
    dataMessage.value = e instanceof Error ? e.message : t('settings.exportFailed')
  } finally {
    exporting.value = false
  }
}

function onImportFileChange(e: Event) {
  const input = e.target as HTMLInputElement
  importFile.value = input.files?.[0] || null
}

async function importData() {
  if (!importFile.value) return
  importing.value = true
  dataMessage.value = ''
  try {
    const text = await importFile.value.text()
    const data = JSON.parse(text)
    await api.post('/environments/import', data)
    dataMessage.value = t('settings.importSuccess')
    importFile.value = null
    setTimeout(() => dataMessage.value = '', 3000)
  } catch (e: unknown) {
    dataMessage.value = e instanceof Error ? e.message : t('settings.importFailed')
  } finally {
    importing.value = false
  }
}
</script>

<template>
  <div class="settings-layout">
    <!-- Left sidebar navigation -->
    <nav class="settings-nav">
      <div class="nav-group">
        <button
          v-for="section in sections"
          :key="section.key"
          class="nav-item"
          :class="{ active: activeTab === section.key }"
          @click="scrollToSection(section.key)"
        >
          <!-- Profile -->
          <svg v-if="section.icon === 'profile'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
          <!-- Appearance -->
          <svg v-else-if="section.icon === 'appearance'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M4.93 4.93l1.41 1.41m11.32 11.32 1.41 1.41M2 12h2m16 0h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/></svg>
          <!-- Terminal -->
          <svg v-else-if="section.icon === 'terminal'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>
          <!-- Security -->
          <svg v-else-if="section.icon === 'security'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
          <!-- Updates -->
          <svg v-else-if="section.icon === 'updates'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
          <!-- Data -->
          <svg v-else-if="section.icon === 'data'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></svg>
          <!-- About -->
          <svg v-else-if="section.icon === 'about'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>
          <span>{{ t(section.labelKey) }}</span>
        </button>
      </div>
    </nav>

    <!-- Right content area -->
    <div ref="contentRef" class="settings-content">

      <!-- Profile -->
      <section id="settings-profile" class="settings-section">
        <div class="section-header">
          <h2>{{ t('settings.profile') }}</h2>
          <p class="section-desc">{{ t('settings.profileDesc') }}</p>
        </div>
        <div class="panel">
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.displayName') }}</b>
              <span>{{ t('settings.displayNameDesc') }}</span>
            </div>
            <input class="field-input" v-model="displayName" />
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.email') }}</b>
              <span>{{ t('settings.emailDesc') }}</span>
            </div>
            <input class="field-input" v-model="profileEmail" />
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.language') }}</b>
              <span>{{ t('settings.languageDesc') }}</span>
            </div>
            <Select
              v-model="settings.language"
              :options="[
                { label: t('settings.langZh'), value: 'zh' },
                { label: t('settings.langEn'), value: 'en' },
              ]"
              class="field-select"
              @update:model-value="onLanguageChange"
            />
          </div>
        </div>
      </section>

      <!-- Appearance -->
      <section id="settings-appearance" class="settings-section">
        <div class="section-header">
          <h2>{{ t('settings.appearance') }}</h2>
          <p class="section-desc">{{ t('settings.appearanceDesc') }}</p>
        </div>
        <div class="panel">
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.theme') }}</b>
              <span>{{ t('settings.themeDesc') }}</span>
            </div>
            <div class="theme-swatches">
              <button
                class="swatch"
                :class="{ 'swatch--on': settings.theme === 'dark' }"
                :title="t('settings.dark')"
                @click="settings.theme = 'dark'"
              >
                <div class="swatch-surface" style="background:#0E1116"></div>
                <span class="swatch-label">{{ t('settings.dark') }}</span>
              </button>
              <button
                class="swatch"
                :class="{ 'swatch--on': settings.theme === 'light' }"
                :title="t('settings.light')"
                @click="settings.theme = 'light'"
              >
                <div class="swatch-surface" style="background:#F8F9FA"></div>
                <span class="swatch-label">{{ t('settings.light') }}</span>
              </button>
              <button
                class="swatch"
                :class="{ 'swatch--on': settings.theme === 'system' }"
                :title="t('settings.themeSystem')"
                @click="settings.theme = 'system'"
              >
                <div class="swatch-surface swatch-sys">
                  <div style="width:50%;height:100%;background:#0E1116"></div>
                  <div style="width:50%;height:100%;background:#F8F9FA"></div>
                </div>
                <span class="swatch-label">{{ t('settings.themeSystem') }}</span>
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- Terminal -->
      <section id="settings-terminal" class="settings-section">
        <div class="section-header">
          <h2>{{ t('settings.terminal') }}</h2>
          <p class="section-desc">{{ t('settings.terminalDesc') }}</p>
        </div>
        <div class="panel">
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.font') }}</b>
              <span>{{ t('settings.fontDesc') }}</span>
            </div>
            <Select
              v-model="settings.terminal_font"
              :options="[
                { label: 'JetBrains Mono', value: 'JetBrains Mono' },
                { label: 'Cascadia Code', value: 'Cascadia Code' },
                { label: 'SF Mono', value: 'SF Mono' },
                { label: 'Fira Code', value: 'Fira Code' },
                { label: 'Source Code Pro', value: 'Source Code Pro' },
              ]"
              class="field-select"
            />
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.fontSize') }}</b>
              <span>{{ t('settings.fontSizeDesc') }}</span>
            </div>
            <div class="field-control">
              <button class="size-btn" @click="settings.terminal_font_size = String(Math.max(10, Number(settings.terminal_font_size) - 1))">−</button>
              <span class="size-value">{{ settings.terminal_font_size }}px</span>
              <button class="size-btn" @click="settings.terminal_font_size = String(Math.min(24, Number(settings.terminal_font_size) + 1))">+</button>
            </div>
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.terminalTheme') }}</b>
              <span>{{ t('settings.terminalThemeDesc') }}</span>
            </div>
            <Select
              v-model="settings.terminal_theme"
              :options="[
                { label: t('settings.terminalThemeDefault'), value: 'default' },
                { label: t('settings.terminalThemeUbuntu'), value: 'ubuntu' },
                { label: t('settings.terminalThemeSolarized'), value: 'solarized-dark' },
              ]"
              class="field-select"
            />
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.bgOpacityLabel') }}</b>
              <span>{{ t('settings.bgOpacityDesc') }}</span>
            </div>
            <div class="field-control opacity-control">
              <input
                type="range"
                min="0"
                max="100"
                :value="settings.terminal_opacity"
                @input="settings.terminal_opacity = Number(($event.target as HTMLInputElement).value)"
                class="opacity-slider"
              />
              <span class="opacity-value">{{ settings.terminal_opacity }}%</span>
            </div>
          </div>
        </div>
      </section>

      <!-- Security -->
      <section id="settings-security" class="settings-section">
        <div class="section-header">
          <h2>{{ t('settings.security') }}</h2>
          <p class="section-desc">{{ t('settings.securityDesc') }}</p>
        </div>
        <div class="panel">
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.sessionTimeout') }}</b>
              <span>{{ t('settings.sessionTimeoutDesc') }}</span>
            </div>
            <Select
              v-model.number="settings.session_timeout"
              :options="[
                { label: '30 ' + t('settings.minutes'), value: 30 },
                { label: '1 ' + t('settings.hours', 'h'), value: 60 },
                { label: t('settings.never', 'Never'), value: 0 },
              ]"
              class="field-select"
            />
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.encryptSecrets') }}</b>
              <span>{{ t('settings.encryptSecretsDesc') }}</span>
            </div>
            <Badge tone="success">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" style="margin-right:4px;vertical-align:-2px"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
              {{ t('settings.alwaysOn') }}
            </Badge>
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.auditLogging') }}</b>
              <span>{{ t('settings.auditLoggingDesc') }}</span>
            </div>
            <Switch v-model="auditLogging" size="sm" />
          </div>
        </div>

        <!-- Password Change -->
        <div class="panel">
          <div class="panel-header">
            <h3>{{ t('settings.password') }}</h3>
            <p class="panel-desc">{{ t('settings.passwordDesc') }}</p>
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.currentPassword') }}</b>
            </div>
            <Input v-model="currentPassword" type="password" autocomplete="current-password" class="field-input" />
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.newPassword') }}</b>
            </div>
            <Input v-model="newPassword" type="password" autocomplete="new-password" class="field-input" />
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.confirmPassword') }}</b>
            </div>
            <Input v-model="confirmPassword" type="password" autocomplete="new-password" class="field-input" />
          </div>
          <div class="field">
            <div class="field-label"></div>
            <div class="field-actions">
              <span v-if="passwordError" class="field-error">{{ passwordError }}</span>
              <span v-if="passwordSuccess" class="field-success">{{ passwordSuccess }}</span>
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
          </div>
        </div>
      </section>

      <!-- Updates -->
      <section id="settings-updates" class="settings-section">
        <div class="section-header">
          <h2>{{ t('settings.updates') }}</h2>
          <p class="section-desc">{{ t('settings.updatesDesc') }}</p>
        </div>
        <div class="panel">
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.hubAutoCheck') }}</b>
              <span>{{ t('settings.hubAutoCheckDesc') }}</span>
            </div>
            <Switch v-model="autoUpdate" size="sm" />
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.currentVersion') }}</b>
              <span class="mono">rex-hub {{ updateStore.currentVersion }}</span>
            </div>
            <Badge v-if="!updateStore.hasUpdate" tone="success">{{ t('settings.upToDate') }}</Badge>
            <Badge v-else tone="warning">{{ t('settings.updateAvailable') }}</Badge>
          </div>
          <div v-if="updateStore.updateLoading" class="field">
            <div class="field-label">
              <b>{{ t('settings.checking') }}</b>
              <span>{{ updateStore.updateStatusText }}</span>
            </div>
            <div class="update-progress">
              <div class="progress-bar">
                <div class="progress-fill" :style="{ width: updateStore.updateProgress + '%' }"></div>
              </div>
            </div>
          </div>
          <div v-if="updateStore.hasUpdate" class="field">
            <div class="field-label">
              <b>{{ t('settings.latestVersion') }}</b>
              <span class="mono">{{ updateStore.latestVersion }}</span>
            </div>
            <Button
              variant="primary"
              size="sm"
              :loading="updateStore.updating"
              @click="updateStore.triggerUpdate"
            >
              {{ t('settings.updateNow') }}
            </Button>
          </div>
          <div v-if="updateStore.updateError" class="field">
            <div class="field-label">
              <b>{{ t('settings.error') }}</b>
              <span class="field-error">{{ updateStore.updateError }}</span>
            </div>
            <Button variant="secondary" size="sm" @click="updateStore.rollbackUpdate">
              {{ t('settings.rollback') }}
            </Button>
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.checkNow') }}</b>
              <span>{{ t('settings.checkNowDesc') }}</span>
            </div>
            <Button
              variant="primary"
              size="sm"
              :loading="updateStore.updateLoading"
              @click="updateStore.checkForUpdate()"
            >
              {{ t('settings.checkForUpdates') }}
            </Button>
          </div>
        </div>
      </section>

      <!-- Data Management -->
      <section id="settings-data" class="settings-section">
        <div class="section-header">
          <h2>{{ t('settings.dataManagement') }}</h2>
          <p class="section-desc">{{ t('settings.dataManagementDesc') }}</p>
        </div>
        <div class="panel">
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.exportData') }}</b>
              <span>{{ t('settings.exportDataDesc') }}</span>
            </div>
            <Button
              variant="secondary"
              size="sm"
              :loading="exporting"
              @click="exportData"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right:4px"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
              {{ t('settings.export') }}
            </Button>
          </div>
          <div class="field">
            <div class="field-label">
              <b>{{ t('settings.importData') }}</b>
              <span>{{ t('settings.importDataDesc') }}</span>
            </div>
            <div class="field-actions">
              <label class="import-btn" :class="{ 'import-btn--ready': importFile }">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right:4px"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
                {{ importFile ? importFile.name : t('settings.chooseFile') }}
                <input type="file" accept=".json" @change="onImportFileChange" class="sr-only" />
              </label>
              <Button
                variant="primary"
                size="sm"
                :loading="importing"
                :disabled="!importFile"
                @click="importData"
              >
                {{ t('settings.import') }}
              </Button>
            </div>
          </div>
          <div v-if="dataMessage" class="field">
            <div class="field-label"></div>
            <span :class="dataMessage.includes('failed') || dataMessage.includes('Failed') ? 'field-error' : 'field-success'">{{ dataMessage }}</span>
          </div>
        </div>
      </section>

      <!-- About -->
      <section id="settings-about" class="settings-section">
        <div class="section-header">
          <h2>{{ t('settings.about') }}</h2>
          <p class="section-desc">{{ t('settings.aboutDesc') }}</p>
        </div>
        <div class="panel about-panel">
          <div class="about-logo">
            <div class="about-logo-icon">R</div>
            <div class="about-logo-text">
              <div class="about-name">REX Hub</div>
              <div class="about-version mono">v{{ updateStore.currentVersion || '—' }}</div>
            </div>
          </div>
          <div class="about-grid">
            <div class="about-item">
              <span class="about-item-label">{{ t('settings.platform') }}</span>
              <span class="about-item-value mono">{{ navigatorPlatform }}</span>
            </div>
            <div class="about-item">
              <span class="about-item-label">{{ t('settings.userAgent') }}</span>
              <span class="about-item-value mono">{{ navigatorUserAgent }}</span>
            </div>
            <div class="about-item">
              <span class="about-item-label">{{ t('settings.license') }}</span>
              <span class="about-item-value">MIT License</span>
            </div>
            <div class="about-item">
              <span class="about-item-label">{{ t('settings.sourceCode') }}</span>
              <span class="about-item-value">
                <a href="https://github.com/JeeLin/REX" target="_blank" rel="noopener" class="about-link">
                  GitHub
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-1px"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                </a>
              </span>
            </div>
          </div>
        </div>
      </section>

      <!-- Save bar -->
      <div class="save-bar">
        <span v-if="saveMessage" class="save-message" :class="{ error: saveMessage.includes('failed') }">{{ saveMessage }}</span>
        <Button variant="primary" :loading="saving" @click="saveSettings">{{ t('settings.saveSettings') }}</Button>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
const navigatorPlatform = navigator.platform || '—'
const navigatorUserAgent = navigator.userAgent.split(' ').pop() || '—'
</script>

<style scoped>
/* Layout */
.settings-layout {
  display: flex;
  gap: var(--space-8);
  max-width: 960px;
  height: 100%;
  margin: 0 auto;
}

/* Left nav */
.settings-nav {
  width: 180px;
  flex-shrink: 0;
  position: sticky;
  top: 0;
  align-self: flex-start;
  padding-top: var(--space-6);
}

.nav-group {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: 8px 12px;
  border-radius: var(--radius);
  font-size: var(--text-base);
  color: var(--text-muted);
  cursor: pointer;
  background: none;
  border: none;
  text-align: left;
  transition: all var(--transition);
  font-family: inherit;
}

.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.nav-item.active {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

.nav-item svg {
  flex-shrink: 0;
}

/* Right content */
.settings-content {
  flex: 1;
  min-width: 0;
  padding: var(--space-6) 0 var(--space-12);
}

/* Section */
.settings-section {
  margin-bottom: var(--space-8);
}

.section-header {
  margin-bottom: var(--space-4);
}

.section-header h2 {
  margin: 0 0 4px;
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.section-desc {
  margin: 0;
  font-size: var(--text-sm);
  color: var(--text-muted);
}

/* Panel */
.panel {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--space-1) var(--space-5);
}

.panel + .panel {
  margin-top: var(--space-3);
}

.panel-header {
  padding: var(--space-4) 0 0;
}

.panel-header h3 {
  margin: 0 0 4px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.panel-desc {
  color: var(--text-muted);
  font-size: 12.5px;
  margin: 0;
}

/* Field row */
.field {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 0;
  border-top: 1px solid var(--border);
  min-height: 48px;
}

.field:first-of-type {
  border-top: 0;
}

.field-label {
  flex: 1;
  min-width: 0;
}

.field-label b {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text-primary);
}

.field-label span {
  display: block;
  color: var(--text-muted);
  font-size: 12px;
  margin-top: 2px;
}

.field-label .mono {
  font-family: var(--font-mono);
  font-size: 12px;
}

.field-input {
  height: 36px;
  padding: 0 12px;
  border-radius: 7px;
  border: 1px solid var(--border-strong);
  background: var(--bg-surface);
  color: var(--text-primary);
  font: inherit;
  font-size: 13px;
  min-width: 220px;
}

.field-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.field-select {
  min-width: 220px;
}

/* Right-side controls alignment */
.field > :deep(.field-select),
.field > :deep(.field-input),
.field > :deep(.switch),
.field > .seg,
.field > .theme-swatches,
.field > .field-actions,
.field > .update-progress,
.field > .badge,
.field > .field-control,
.field > .opacity-control {
  margin-left: auto;
}

.field-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.field-error {
  font-size: var(--text-sm);
  color: var(--danger);
}

.field-success {
  font-size: var(--text-sm);
  color: var(--success);
}

/* Theme swatches */
.theme-swatches {
  display: flex;
  gap: 12px;
}

.swatch {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 0;
  border: 2px solid var(--border);
  border-radius: var(--radius);
  background: transparent;
  cursor: pointer;
  transition: border-color var(--transition);
}

.swatch:hover {
  border-color: var(--border-strong);
}

.swatch--on {
  border-color: var(--accent);
}

.swatch-surface {
  width: 40px;
  height: 28px;
  border-radius: 4px;
  overflow: hidden;
}

.swatch-sys {
  display: flex;
}

.swatch-label {
  font-size: 11px;
  color: var(--text-muted);
  padding-bottom: 4px;
}

.swatch--on .swatch-label {
  color: var(--accent);
}

/* Font size stepper */
.field-control {
  display: flex;
  align-items: center;
  gap: 0;
  border: 1px solid var(--border-strong);
  border-radius: 7px;
  overflow: hidden;
  height: 36px;
}

.size-btn {
  width: 36px;
  height: 100%;
  border: none;
  background: var(--bg-elevated);
  color: var(--text-secondary);
  font-size: 16px;
  cursor: pointer;
  transition: all var(--transition);
  font-family: inherit;
}

.size-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.size-value {
  min-width: 52px;
  text-align: center;
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text-primary);
}

/* Opacity slider */
.opacity-control {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  border: none;
  height: auto;
}

.opacity-slider {
  width: 120px;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: var(--border-strong);
  border-radius: 2px;
  outline: none;
}

.opacity-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--accent);
  cursor: pointer;
  border: 2px solid var(--bg-surface);
  box-shadow: 0 0 0 1px var(--border-strong);
}

.opacity-slider::-moz-range-thumb {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--accent);
  cursor: pointer;
  border: 2px solid var(--bg-surface);
}

.opacity-value {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--text-muted);
  min-width: 36px;
}

/* Import button */
.import-btn {
  display: inline-flex;
  align-items: center;
  height: var(--btn-height-sm);
  padding: 0 12px;
  border-radius: 7px;
  border: 1px dashed var(--border-strong);
  background: var(--bg-surface);
  color: var(--text-muted);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: all var(--transition);
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.import-btn:hover {
  border-color: var(--accent);
  color: var(--text-primary);
}

.import-btn--ready {
  border-style: solid;
  border-color: var(--accent);
  color: var(--accent);
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border-width: 0;
}

/* Update progress */
.update-progress {
  min-width: 220px;
}

.progress-bar {
  height: 4px;
  background: var(--bg-deep);
  border-radius: 3px;
  overflow: hidden;
  width: 120px;
}

.progress-fill {
  height: 100%;
  background: var(--accent);
  transition: width 0.3s;
}

/* About panel */
.about-panel {
  padding: var(--space-5);
}

.about-logo {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding-bottom: var(--space-5);
  margin-bottom: var(--space-4);
  border-bottom: 1px solid var(--border);
}

.about-logo-icon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-lg);
  background: linear-gradient(135deg, var(--accent), var(--brand-deep));
  color: var(--on-brand);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  font-weight: 700;
  font-family: var(--font-mono);
  flex-shrink: 0;
}

.about-name {
  font-size: var(--text-lg);
  font-weight: 700;
  color: var(--text-primary);
}

.about-version {
  font-size: var(--text-sm);
  color: var(--text-muted);
}

.about-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: var(--space-3) var(--space-6);
  align-items: baseline;
}

.about-item {
  display: contents;
}

.about-item-label {
  font-size: var(--text-sm);
  color: var(--text-muted);
}

.about-item-value {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.about-link {
  color: var(--accent);
  text-decoration: none;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  transition: color var(--transition);
}

.about-link:hover {
  color: var(--accent-hover);
}

/* Save bar */
.save-bar {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  position: sticky;
  bottom: 0;
  padding: var(--space-4) 0;
  background: linear-gradient(transparent, var(--bg-page) 20%);
  z-index: 10;
}

.save-message {
  font-size: var(--text-sm);
  color: var(--success);
}

.save-message.error {
  color: var(--danger);
}

.mono {
  font-family: var(--font-mono);
}

/* Responsive */
@media (max-width: 760px) {
  .settings-layout {
    flex-direction: column;
    gap: 0;
  }
  .settings-nav {
    width: 100%;
    position: static;
    padding: var(--space-3) 0;
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
  }
  .nav-group {
    flex-direction: row;
    gap: 0;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
  }
  .nav-group::-webkit-scrollbar { display: none; }
  .nav-item {
    white-space: nowrap;
    padding: 8px 14px;
    font-size: var(--text-sm);
  }
  .nav-item span { display: none; }
  .nav-item svg { margin: 0; }
  .nav-item.active span { display: inline; }
  .settings-content {
    padding: var(--space-4) 0;
  }
  .field {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }
  .field > :deep(.field-select),
  .field > :deep(.field-input),
  .field > :deep(.switch),
  .field > .field-actions,
  .field > .field-control,
  .field > .opacity-control {
    margin-left: 0;
    width: 100%;
  }
  .field-select { min-width: 0; width: 100%; }
  .field-input { min-width: 0; width: 100%; }
}
</style>
