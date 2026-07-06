import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { getUserSettings, updateUserSettings } from '@/api/settings'

export const useSettingsStore = defineStore('settings', () => {
  // ── Terminal settings ──
  const terminalSettings = reactive({
    fontSize: Number(localStorage.getItem('rex-term-font-size')) || 13,
    fontFamily: localStorage.getItem('rex-term-font-family') || 'JetBrains Mono',
    cursorBlink: localStorage.getItem('rex-term-cursor-blink') !== 'false',
    keepalive: Number(localStorage.getItem('rex-term-keepalive')) || 60,
  })

  function updateTerminalSetting<K extends keyof typeof terminalSettings>(
    key: K,
    value: (typeof terminalSettings)[K],
  ) {
    ;(terminalSettings[key] as unknown) = value
    const storageKey = `rex-term-${key === 'cursorBlink' ? 'cursor-blink' : key}`
    localStorage.setItem(storageKey, String(value))
    syncToBackend()
  }

  // ── Security settings ──
  const securitySettings = reactive({
    sessionTimeout: Number(localStorage.getItem('rex-session-timeout')) || 30,
    auditEnabled: localStorage.getItem('rex-audit-enabled') !== 'false',
    configEncryption: localStorage.getItem('rex-config-encryption') !== 'false',
  })

  function updateSecuritySetting<K extends keyof typeof securitySettings>(
    key: K,
    value: (typeof securitySettings)[K],
  ) {
    ;(securitySettings[key] as unknown) = value
    const storageKey = key === 'auditEnabled' ? 'rex-audit-enabled' : `rex-${key}`
    localStorage.setItem(storageKey, String(value))
    syncToBackend()
  }

  // ── Appearance settings ──
  const appearanceSettings = reactive({
    sidebarCollapsible: localStorage.getItem('rex-sidebar-collapsible') === 'true',
  })

  function updateAppearanceSetting<K extends keyof typeof appearanceSettings>(
    key: K,
    value: (typeof appearanceSettings)[K],
  ) {
    ;(appearanceSettings[key] as unknown) = value
    localStorage.setItem(`rex-${key}`, String(value))
    syncToBackend()
  }

  // ── Backend sync ──

  let syncTimer: ReturnType<typeof setTimeout> | null = null

  /** Debounced sync to backend (300ms) */
  function syncToBackend() {
    if (syncTimer) clearTimeout(syncTimer)
    syncTimer = setTimeout(async () => {
      try {
        await updateUserSettings({
          session_timeout: securitySettings.sessionTimeout,
          audit_enabled: securitySettings.auditEnabled,
          config_encryption: securitySettings.configEncryption,
          sidebar_collapsible: appearanceSettings.sidebarCollapsible,
          terminal_font_size: terminalSettings.fontSize,
          terminal_font_family: terminalSettings.fontFamily,
          terminal_cursor_blink: terminalSettings.cursorBlink,
          terminal_keepalive: terminalSettings.keepalive,
        })
      } catch {
        // ignore — localStorage is already saved as fallback
      }
    }, 300)
  }

  /** Load settings from backend, falling back to localStorage */
  async function loadSettingsFromBackend() {
    try {
      const remote = await getUserSettings()

      // Terminal settings
      if (remote.terminal_font_size !== undefined) {
        terminalSettings.fontSize = remote.terminal_font_size
        localStorage.setItem('rex-term-font-size', String(remote.terminal_font_size))
      }
      if (remote.terminal_font_family !== undefined) {
        terminalSettings.fontFamily = remote.terminal_font_family
        localStorage.setItem('rex-term-font-family', remote.terminal_font_family)
      }
      if (remote.terminal_cursor_blink !== undefined) {
        terminalSettings.cursorBlink = remote.terminal_cursor_blink
        localStorage.setItem('rex-term-cursor-blink', String(remote.terminal_cursor_blink))
      }
      if (remote.terminal_keepalive !== undefined) {
        terminalSettings.keepalive = remote.terminal_keepalive
        localStorage.setItem('rex-term-keepalive', String(remote.terminal_keepalive))
      }

      // Security settings
      if (remote.session_timeout !== undefined) {
        securitySettings.sessionTimeout = remote.session_timeout
        localStorage.setItem('rex-session-timeout', String(remote.session_timeout))
      }
      if (remote.audit_enabled !== undefined) {
        securitySettings.auditEnabled = remote.audit_enabled
        localStorage.setItem('rex-audit-enabled', String(remote.audit_enabled))
      }
      if (remote.config_encryption !== undefined) {
        securitySettings.configEncryption = remote.config_encryption
        localStorage.setItem('rex-config-encryption', String(remote.config_encryption))
      }

      // Appearance settings
      if (remote.sidebar_collapsible !== undefined) {
        appearanceSettings.sidebarCollapsible = remote.sidebar_collapsible
        localStorage.setItem('rex-sidebar-collapsible', String(remote.sidebar_collapsible))
      }
    } catch {
      // ignore — use localStorage values as-is
    }
  }

  return {
    terminalSettings,
    updateTerminalSetting,
    securitySettings,
    updateSecuritySetting,
    appearanceSettings,
    updateAppearanceSetting,
    loadSettingsFromBackend,
  }
})

// Re-export for backward compatibility — consumers import from this module directly
export { useSettingsStore as settingsStore }

/** Convenience accessors for components that only need to read values */
export function getTerminalSettings() {
  return useSettingsStore().terminalSettings
}

export function getSecuritySettings() {
  return useSettingsStore().securitySettings
}

export function getAppearanceSettings() {
  return useSettingsStore().appearanceSettings
}
