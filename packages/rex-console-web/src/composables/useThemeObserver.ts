import { onBeforeUnmount } from 'vue'

export function useThemeObserver(callback: () => void): void {
  const observer = new MutationObserver(callback)
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  })
  onBeforeUnmount(() => observer.disconnect())
}
