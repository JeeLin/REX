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

// 早期应用主题（避免闪烁）
if (typeof localStorage !== 'undefined') {
  const theme = localStorage.getItem('rex-theme') || 'dark'
  document.documentElement.dataset.theme = theme === 'light' ? 'light' : undefined
}

export type AppLocale = 'zh' | 'en'
