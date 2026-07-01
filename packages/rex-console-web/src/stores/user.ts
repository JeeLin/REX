import { defineStore } from 'pinia'
import { ref } from 'vue'
import { updateUserSettings } from '@/api/settings'

export type Theme = 'dark' | 'light' | 'system'
export type Lang = 'zh' | 'en'

export const useUserStore = defineStore('user', () => {
  const theme = ref<Theme>((localStorage.getItem('rex-theme') as Theme) || 'dark')
  const lang = ref<Lang>((localStorage.getItem('rex-lang') as Lang) || 'zh')

  function setTheme(newTheme: Theme) {
    theme.value = newTheme
    localStorage.setItem('rex-theme', newTheme)
    applyTheme(newTheme)
    // Sync to backend (fire and forget)
    updateUserSettings({ theme: newTheme }).catch(() => {})
  }

  function setLang(newLang: Lang) {
    lang.value = newLang
    localStorage.setItem('rex-lang', newLang)
    // Sync to backend (fire and forget)
    updateUserSettings({ lang: newLang }).catch(() => {})
  }

  function applyTheme(t: Theme) {
    if (t === 'system') {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
      document.documentElement.setAttribute('data-theme', prefersDark ? 'dark' : 'light')
    } else {
      document.documentElement.setAttribute('data-theme', t)
    }
  }

  /** Load theme/lang from backend and apply */
  async function loadFromBackend() {
    try {
      const { getUserSettings } = await import('@/api/settings')
      const remote = await getUserSettings()
      if (remote.theme && remote.theme !== theme.value) {
        theme.value = remote.theme as Theme
        localStorage.setItem('rex-theme', remote.theme)
        applyTheme(remote.theme as Theme)
      }
      if (remote.lang && remote.lang !== lang.value) {
        lang.value = remote.lang as Lang
        localStorage.setItem('rex-lang', remote.lang)
      }
    } catch {
      // ignore — use localStorage values
    }
  }

  // 初始化时应用主题
  applyTheme(theme.value)

  return { theme, lang, setTheme, setLang, loadFromBackend }
})
