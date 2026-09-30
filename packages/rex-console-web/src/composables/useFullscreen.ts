import { computed, onMounted, onUnmounted, ref } from 'vue'

// Module-scope singleton: AppLayout (F11 / topbar button) and the WorkspacePage
// toolbar button share one flag so both always show the same mode.
const isFullscreen = ref(false)

// Guards against re-entering toggle/exit while a Fullscreen API promise is
// still in flight (a second trigger would flip the flag in the catch block
// and desync it from the real fullscreenchange state).
let pending = false

/**
 * Shared fullscreen state for AppLayout (F11 / topbar button) and the
 * WorkspacePage toolbar button, so both always agree on the current mode.
 *
 * - `isFullscreen` mirrors `document.fullscreenElement`; when the Fullscreen
 *   API is unavailable or a request is rejected it degrades to a plain
 *   toggleable UI flag instead of throwing.
 * - `supported` is a computed `document.fullscreenEnabled !== false`.
 * - `toggle()` / `exit()` never reject.
 */
export function useFullscreen() {
  // Evaluated lazily: callers may flip document.fullscreenEnabled at any time
  // (tests and embedded webviews toggle it at runtime).
  const supported = computed(() => typeof document !== 'undefined' && document.fullscreenEnabled !== false)

  function sync() {
    isFullscreen.value = !!document.fullscreenElement
  }

  async function toggle(): Promise<void> {
    if (pending) return
    if (!supported.value) {
      isFullscreen.value = !isFullscreen.value
      return
    }
    pending = true
    try {
      if (document.fullscreenElement) {
        await document.exitFullscreen()
      } else {
        await document.documentElement.requestFullscreen()
      }
      sync()
    } catch {
      isFullscreen.value = !isFullscreen.value
    } finally {
      pending = false
    }
  }

  async function exit(): Promise<void> {
    if (pending) return
    if (!supported.value || !document.fullscreenElement) {
      isFullscreen.value = false
      return
    }
    pending = true
    try {
      await document.exitFullscreen()
      sync()
    } catch {
      isFullscreen.value = false
    } finally {
      pending = false
    }
  }

  onMounted(() => {
    document.addEventListener('fullscreenchange', sync)
    sync()
  })
  onUnmounted(() => document.removeEventListener('fullscreenchange', sync))

  return { isFullscreen, supported, toggle, exit }
}
