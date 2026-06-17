import { defineStore } from 'pinia'
import { ref } from 'vue'

export type Theme = 'dark' | 'light' | 'system'
export type Lang = 'zh' | 'en'

export const useUserStore = defineStore('user', () => {
  const theme = ref<Theme>((localStorage.getItem('rex-theme') as Theme) || 'dark')
  const lang = ref<Lang>((localStorage.getItem('rex-lang') as Lang) || 'zh')

  function setTheme(newTheme: Theme) {
    theme.value = newTheme
    localStorage.setItem('rex-theme', newTheme)
    applyTheme(newTheme)
  }

  function setLang(newLang: Lang) {
    lang.value = newLang
    localStorage.setItem('rex-lang', newLang)
  }

  function applyTheme(t: Theme) {
    if (t === 'system') {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
      document.documentElement.setAttribute('data-theme', prefersDark ? 'dark' : 'light')
    } else {
      document.documentElement.setAttribute('data-theme', t)
    }
  }

  // 初始化时应用主题
  applyTheme(theme.value)

  return { theme, lang, setTheme, setLang }
})
