import { onMounted, onUnmounted, ref } from 'vue'

/**
 * Shared fullscreen state for AppLayout (F11 / topbar button) and the
 * WorkspacePage toolbar button, so both always agree on the current mode.
 *
 * - `isFullscreen` mirrors `document.fullscreenElement`; when the Fullscreen
 *   API is unavailable or a request is rejected it degrades to a plain
 *   toggleable UI flag instead of throwing.
 * - `supported` is `document.fullscreenEnabled !== false`.
 * - `toggle()` / `exit()` never reject.
 */
export function useFullscreen() {
  const isFullscreen = ref(false)
  const supported = typeof document !== 'undefined' && document.fullscreenEnabled !== false

  function sync() {
    isFullscreen.value = !!document.fullscreenElement
  }

  async function toggle(): Promise<void> {
    if (!supported) {
      isFullscreen.value = !isFullscreen.value
      return
    }
    try {
      if (document.fullscreenElement) {
        await document.exitFullscreen()
      } else {
        await document.documentElement.requestFullscreen()
      }
      sync()
    } catch {
      isFullscreen.value = !isFullscreen.value
    }
  }

  async function exit(): Promise<void> {
    if (!supported || !document.fullscreenElement) {
      isFullscreen.value = false
      return
    }
    try {
      await document.exitFullscreen()
      sync()
    } catch {
      isFullscreen.value = false
    }
  }

  onMounted(() => {
    document.addEventListener('fullscreenchange', sync)
    sync()
  })
  onUnmounted(() => document.removeEventListener('fullscreenchange', sync))

  return { isFullscreen, supported, toggle, exit }
}
