import { reactive } from 'vue'

// ── Terminal settings ──
export const terminalSettings = reactive({
  fontSize: Number(localStorage.getItem('rex-term-font-size')) || 13,
  fontFamily: localStorage.getItem('rex-term-font-family') || 'JetBrains Mono',
  cursorBlink: localStorage.getItem('rex-term-cursor-blink') !== 'false',
  keepalive: Number(localStorage.getItem('rex-term-keepalive')) || 60,
})

export function updateTerminalSetting<K extends keyof typeof terminalSettings>(
  key: K,
  value: (typeof terminalSettings)[K],
) {
  ;(terminalSettings[key] as unknown) = value
  localStorage.setItem(`rex-term-${key === 'cursorBlink' ? 'cursor-blink' : key}`, String(value))
}

// ── Security settings ──
export const securitySettings = reactive({
  sessionTimeout: Number(localStorage.getItem('rex-session-timeout')) || 30,
  auditEnabled: localStorage.getItem('rex-audit-enabled') !== 'false',
})

export function updateSecuritySetting<K extends keyof typeof securitySettings>(
  key: K,
  value: (typeof securitySettings)[K],
) {
  ;(securitySettings[key] as unknown) = value
  const storageKey = key === 'auditEnabled' ? 'rex-audit-enabled' : `rex-${key}`
  localStorage.setItem(storageKey, String(value))
}
