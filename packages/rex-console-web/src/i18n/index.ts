import { createI18n } from 'vue-i18n'
import zh from './locales/zh.json'
import en from './locales/en.json'

const stored = typeof localStorage !== 'undefined' ? localStorage.getItem('rex-lang') : null
const fallback = typeof navigator !== 'undefined' && navigator.language.startsWith('zh') ? 'zh' : 'en'

export const i18n = createI18n({
  legacy: false,
  locale: stored || fallback,
  fallbackLocale: 'en',
  messages: { zh, en },
})

export type AppLocale = 'zh' | 'en'
