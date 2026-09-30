import type { Ref } from 'vue'

const LANG_KEY = 'rex-lang'

// Shared setter for language persistence: switch the i18n locale ref and persist it.
// Not yet the only write point — some pages still write rex-lang directly
// (LoginPage / SettingsPage) and could be migrated later.
export function setLanguage(locale: string, localeRef: Ref<string>) {
  localStorage.setItem(LANG_KEY, locale)
  localeRef.value = locale
}
