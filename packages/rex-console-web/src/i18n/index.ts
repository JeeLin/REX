import { createI18n } from 'vue-i18n'
import zh from './zh'
import en from './en'

export const i18n = createI18n({
  legacy: false,
  locale: localStorage.getItem('rex-lang') || 'zh',
  fallbackLocale: 'en',
  messages: { zh, en },
})

/**
 * 独立的翻译函数，可在 Vue 组件外使用（如 api/client.ts）
 * 内部读取 i18n 实例的当前 locale
 */
export function t(key: string, params?: Record<string, unknown>): string {
  const { global } = i18n
  return params ? (global.t(key, params) as string) : (global.t(key) as string)
}
