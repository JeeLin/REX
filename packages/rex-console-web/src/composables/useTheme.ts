import type { Ref } from 'vue'

const THEME_KEY = 'rex-theme'
const LANG_KEY = 'rex-lang'

// Single write point for the theme: localStorage plus the dataset attribute the
// stylesheet keys on (dark is the default, so it clears the attribute).
export function setTheme(mode: 'light' | 'dark') {
  localStorage.setItem(THEME_KEY, mode)
  if (mode === 'dark') {
    delete document.documentElement.dataset.theme
  } else {
    document.documentElement.dataset.theme = mode
  }
}

// Single write point for the language: switch the i18n locale ref and persist it.
export function setLanguage(locale: string, localeRef: Ref<string>) {
  localStorage.setItem(LANG_KEY, locale)
  localeRef.value = locale
}

export function toggleTheme() {
  const current = localStorage.getItem(THEME_KEY) || 'dark'
  setTheme(current === 'dark' ? 'light' : 'dark')
}
