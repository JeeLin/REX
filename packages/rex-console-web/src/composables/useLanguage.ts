import type { Ref } from 'vue'

const LANG_KEY = 'rex-lang'

// Single write point for the language: switch the i18n locale ref and persist it.
export function setLanguage(locale: string, localeRef: Ref<string>) {
  localStorage.setItem(LANG_KEY, locale)
  localeRef.value = locale
}
