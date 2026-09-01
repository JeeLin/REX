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
import Badge from '@/components/ui/Badge.vue'

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

// Scroll spy: update activeTab based on scroll position
import { onBeforeUnmount } from 'vue'

const sectionIds = ['profile', 'appearance', 'terminal', 'security', 'update']

function handleScroll() {
  const container = contentRef.value
  if (!container) return
  const scrollTop = container.scrollTop
  for (let i = sectionIds.length - 1; i >= 0; i--) {
    const el = document.getElementById(`settings-${sectionIds[i]}`)
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
    <!-- Left sidebar navigation -->
    <nav class="settings-nav">
      <button
        class="settings-nav-item"
        :class="{ active: activeTab === 'profile' }"
        @click="scrollToSection('profile')"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
        <span>Profile</span>
      </button>
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="settings-nav-item"
        :class="{ 'active': activeTab === tab.key }"
        @click="scrollToSection(tab.key)"
      >
        <svg v-if="tab.key === 'appearance'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M4.93 4.93l1.41 1.41m11.32 11.32 1.41 1.41M2 12h2m16 0h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/></svg>
        <svg v-else-if="tab.key === 'terminal'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>
        <svg v-else-if="tab.key === 'security'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
        <svg v-else-if="tab.key === 'update'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
        <span>{{ t(tab.labelKey) }}</span>
      </button>
    </nav>

    <!-- Right content area -->
    <div ref="contentRef" class="settings-content">
      <!-- Profile -->
      <div id="settings-profile" class="settings-section">
        <div class="panel">
          <h3>Profile</h3>
          <p class="panel-desc">Your local operator identity. REX is single-user — this is you.</p>
          <div class="field">
            <div class="field-label">
              <b>Display name</b>
              <span>Shown in the top bar and audit log.</span>
            </div>
            <input class="field-input" value="admin" />
          </div>
          <div class="field">
            <div class="field-label">
              <b>Email</b>
              <span>Used for deployment notifications.</span>
            </div>
            <input class="field-input" value="admin@rex.local" />
          </div>
          <div class="field">
            <div class="field-label">
              <b>Language</b>
              <span>Interface language.</span>
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
      </div>

      <!-- Appearance -->
      <div id="settings-appearance" class="settings-section">
        <div class="panel">
          <h3>Appearance</h3>
          <p class="panel-desc">Dark-first, geek aesthetic. Tune the surface to your taste.</p>
          <div class="field">
            <div class="field-label">
              <b>Theme</b>
              <span>Dark is recommended for long ops sessions.</span>
            </div>
            <div class="theme-swatches">
              <button
                class="swatch"
                :class="{ 'swatch--on': settings.theme === 'dark' }"
                title="Dark"
                @click="settings.theme = 'dark'"
              >
                <div class="swatch-surface" style="background:#0E1116"></div>
              </button>
              <button
                class="swatch"
                :class="{ 'swatch--on': settings.theme === 'light' }"
                title="Light"
                @click="settings.theme = 'light'"
              >
                <div class="swatch-surface" style="background:#F8F9FA"></div>
              </button>
              <button
                class="swatch"
                :class="{ 'swatch--on': settings.theme === 'system' }"
                title="System"
                @click="settings.theme = 'system'"
              >
                <div class="swatch-surface swatch-sys">
                  <div style="width:50%;height:100%;background:#0E1116"></div>
                  <div style="width:50%;height:100%;background:#F8F9FA"></div>
                </div>
              </button>
            </div>
          </div>
          <div class="field">
            <div class="field-label">
              <b>Accent</b>
              <span>Primary action and selection color.</span>
            </div>
            <div class="seg">
              <button class="on">Orange</button>
              <button>Blue</button>
              <button>Green</button>
            </div>
          </div>
          <div class="field">
            <div class="field-label">
              <b>Sidebar density</b>
              <span>Compact shows more rows.</span>
            </div>
            <div class="seg">
              <button class="on">Comfortable</button>
              <button>Compact</button>
            </div>
          </div>
        </div>
      </div>

      <!-- Terminal -->
      <div id="settings-terminal" class="settings-section">
        <div class="panel">
          <h3>Terminal</h3>
          <p class="panel-desc">Defaults applied to every new SSH session.</p>
          <div class="field">
            <div class="field-label">
              <b>Font</b>
              <span>Monospace family for the terminal.</span>
            </div>
            <Select
              v-model="settings.terminal_font"
              :options="[
                { label: 'JetBrains Mono', value: 'JetBrains Mono' },
                { label: 'Cascadia Code', value: 'Cascadia Code' },
                { label: 'SF Mono', value: 'SF Mono' },
              ]"
              class="field-select"
            />
          </div>
          <div class="field">
            <div class="field-label">
              <b>Font size</b>
              <span>Size in pixels for the terminal text.</span>
            </div>
            <input class="field-input" style="min-width:100px" :value="settings.terminal_font_size" @input="settings.terminal_font_size = ($event.target as HTMLInputElement).value" type="number" />
          </div>
          <div class="field">
            <div class="field-label">
              <b>Terminal theme</b>
              <span>Color scheme for the terminal emulator.</span>
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
              <b>Background opacity</b>
              <span>Transparency of the terminal background.</span>
            </div>
            <input class="field-input" style="min-width:100px" :value="settings.terminal_opacity" @input="settings.terminal_opacity = Number(($event.target as HTMLInputElement).value)" type="number" />
          </div>
          <div class="field">
            <div class="field-label">
              <b>Cursor blink</b>
              <span>Animate the terminal cursor.</span>
            </div>
            <div class="switch on"><i></i></div>
          </div>
          <div class="field">
            <div class="field-label">
              <b>Keep-alive</b>
              <span>Send heartbeat every 30s.</span>
            </div>
            <div class="switch on"><i></i></div>
          </div>
        </div>
      </div>

      <!-- Security -->
      <div id="settings-security" class="settings-section">
        <div class="panel">
          <h3>Security</h3>
          <p class="panel-desc">Self-hosted, single-user. Credentials are encrypted at rest with AES-256.</p>
          <div class="field">
            <div class="field-label">
              <b>Session timeout</b>
              <span>Auto-lock after inactivity.</span>
            </div>
            <Select
              v-model.number="settings.session_timeout"
              :options="[
                { label: `15 ${t('settings.minutes')}`, value: 15 },
                { label: `30 ${t('settings.minutes')}`, value: 30 },
                { label: `60 ${t('settings.minutes')}`, value: 60 },
                { label: `120 ${t('settings.minutes')}`, value: 120 },
              ]"
              class="field-select"
            />
          </div>
          <div class="field">
            <div class="field-label">
              <b>Encrypt stored secrets</b>
              <span>Resource passwords &amp; keys.</span>
            </div>
            <div class="switch on"><i></i></div>
          </div>
          <div class="field">
            <div class="field-label">
              <b>Audit logging</b>
              <span>Record every operation.</span>
            </div>
            <div class="switch on"><i></i></div>
          </div>
        </div>

        <!-- Password Change -->
        <div class="panel">
          <h3>Password</h3>
          <p class="panel-desc">Change the local admin password. Requires current password.</p>
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
      </div>

      <!-- Updates -->
      <div id="settings-update" class="settings-section">
        <div class="panel">
          <h3>Updates</h3>
          <p class="panel-desc">Hub and Agent versions must match — no cross-version compatibility.</p>
          <div class="field">
            <div class="field-label">
              <b>Hub auto-check</b>
              <span>Notify on new Hub releases.</span>
            </div>
            <Switch v-model="autoUpdate" size="sm" />
          </div>
          <div class="field">
            <div class="field-label">
              <b>Current version</b>
              <span>rex-hub {{ updateStore.currentVersion }}</span>
            </div>
            <Badge v-if="!updateStore.hasUpdate" tone="success">up to date</Badge>
            <Badge v-else tone="warning">update available</Badge>
          </div>
          <div v-if="updateStore.updateLoading" class="field">
            <div class="field-label">
              <b>Checking</b>
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
              <b>Latest version</b>
              <span>{{ updateStore.latestVersion }}</span>
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
              <b>Error</b>
              <span>{{ updateStore.updateError }}</span>
            </div>
            <Button variant="secondary" size="sm" @click="updateStore.rollbackUpdate">
              {{ t('settings.rollback') }}
            </Button>
          </div>
          <div class="field">
            <div class="field-label">
              <b>Check now</b>
              <span>Manually check for Hub / Agent updates.</span>
            </div>
            <Button
              variant="primary"
              size="sm"
              :loading="updateStore.updateLoading"
              @click="updateStore.checkForUpdate()"
            >
              Check for updates
            </Button>
          </div>
        </div>
      </div>

      <!-- Save bar -->
      <div class="save-bar">
        <span v-if="saveMessage" class="save-message" :class="{ error: saveMessage.includes('failed') }">{{ saveMessage }}</span>
        <Button variant="primary" :loading="saving" @click="saveSettings">{{ t('settings.saveSettings') }}</Button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Settings layout: left nav + right content */
.settings-layout {
  display: flex;
  gap: var(--space-6);
  max-width: 920px;
  height: 100%;
}

/* Left sidebar nav */
.settings-nav {
  width: 200px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  position: sticky;
  top: 0;
  align-self: flex-start;
}

.settings-nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: 8px 12px;
  border-radius: 7px;
  font-size: var(--text-base);
  color: var(--text-muted);
  cursor: pointer;
  background: none;
  border: none;
  text-align: left;
  transition: all var(--transition);
  font-family: inherit;
}

.settings-nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.settings-nav-item.active {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

/* Right content */
.settings-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-5);
  overflow-y: auto;
}

.settings-section {
  margin-bottom: 0;
}

/* Panel card */
.panel {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--space-5);
}

.panel + .panel {
  margin-top: var(--space-4);
}

.panel h3 {
  margin: 0 0 4px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.panel-desc {
  color: var(--text-muted);
  font-size: 12.5px;
  margin: 0 0 var(--space-4);
}

/* Field row */
.field {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 0;
  border-top: 1px solid var(--border);
}

.field:first-of-type {
  border-top: 0;
}

.field-label {
  flex: 1;
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

.field-input {
  height: 36px;
  padding: 0 12px;
  border-radius: 7px;
  border: 1px solid var(--border-strong);
  background: var(--bg-deep);
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

.theme-swatches {
  display: flex;
  gap: 10px;
}
.swatch {
  width: 44px;
  height: 36px;
  border-radius: 8px;
  border: 2px solid var(--border);
  background: transparent;
  padding: 2px;
  cursor: pointer;
  overflow: hidden;
  transition: border-color var(--transition);
}
.swatch--on {
  border-color: var(--accent);
}
.swatch-surface {
  width: 100%;
  height: 100%;
  border-radius: 4px;
}
.swatch-sys {
  display: flex;
  overflow: hidden;
}
.field-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 220px;
  justify-content: flex-end;
}

.field-error {
  font-size: var(--text-sm);
  color: var(--danger);
}

.field-success {
  font-size: var(--text-sm);
  color: var(--success);
}

/* Segmented control */
.seg {
  display: inline-flex;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  overflow: hidden;
  flex: none;
}

.seg button {
  height: 32px;
  padding: 0 14px;
  background: var(--bg-surface);
  color: var(--text-muted);
  border: 0;
  border-right: 1px solid var(--border);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
  transition: all var(--transition);
}

.seg button:last-child {
  border-right: 0;
}

.seg button.on {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

/* Toggle switch */
.switch {
  width: 42px;
  height: 24px;
  border-radius: var(--radius-pill);
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  position: relative;
  cursor: pointer;
  flex: none;
  transition: background var(--transition);
}

.switch i {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--text-muted);
  transition: transform var(--transition), background var(--transition);
  display: block;
}

.switch.on {
  background: var(--accent-soft);
  border-color: var(--accent);
}

.switch.on i {
  transform: translateX(18px);
  background: var(--accent);
}

/* Save bar */
.save-bar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-3);
  margin-top: var(--space-4);
  padding: var(--space-4);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  position: sticky;
  bottom: 0;
}

.save-message {
  font-size: var(--text-sm);
  color: var(--success);
}

.save-message.error {
  color: var(--danger);
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

/* Responsive: single column on mobile */
@media (max-width: 760px) {
  .settings-layout {
    flex-direction: column;
  }
  .settings-nav {
    width: 100%;
    flex-direction: row;
    flex-wrap: wrap;
    position: static;
  }
}
</style>
