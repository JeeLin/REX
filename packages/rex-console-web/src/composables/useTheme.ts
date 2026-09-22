const THEME_KEY = 'rex-theme'

export function toggleTheme() {
  const current = localStorage.getItem(THEME_KEY) || 'dark'
  const next = current === 'dark' ? 'light' : 'dark'
  localStorage.setItem(THEME_KEY, next)
  if (next === 'dark') {
    delete document.documentElement.dataset.theme
  } else {
    document.documentElement.dataset.theme = next
  }
}
