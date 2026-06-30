/**
 * Copy text to clipboard with execCommand fallback for restricted contexts
 * (e.g. HTTP pages where navigator.clipboard is unavailable).
 */
export function copyWithFallback(text: string): void {
  navigator.clipboard?.writeText(text).catch(() => {
    const ta = document.createElement('textarea')
    ta.value = text
    ta.style.position = 'fixed'
    ta.style.opacity = '0'
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
  })
}
