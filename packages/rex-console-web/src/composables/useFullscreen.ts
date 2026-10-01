import { ref } from 'vue'

// Module-scope singleton shared by every consumer — see useFullscreen() below.
const isFullscreen = ref(false)

// Guards against re-entering toggle/exit while a Fullscreen API promise is
// still in flight (a second trigger would flip the flag in the catch block
// and desync it from the real fullscreenchange state).
let pending = false

// Evaluated on every call: document.fullscreenEnabled is not reactive, so a
// computed would cache the first read (tests and embedded webviews flip it at
// runtime).
function supported(): boolean {
  return typeof document !== 'undefined' && document.fullscreenEnabled !== false
}

function sync() {
  // No-op under SSR / non-DOM test environments.
  if (typeof document === 'undefined') return
  isFullscreen.value = !!document.fullscreenElement
}

// The state is module-scope, so the listener is too: registering it per
// instance would run the idempotent sync() once per mounted consumer
// (AppLayout + WorkspacePage) for a single fullscreenchange event.
if (typeof document !== 'undefined') {
  document.addEventListener('fullscreenchange', sync)
  sync()
}

/**
 * Shared fullscreen state for AppLayout (F11 / topbar button) and the
 * WorkspacePage toolbar button, so both always agree on the current mode.
 *
 * - `isFullscreen` mirrors `document.fullscreenElement`; when the Fullscreen
 *   API is unavailable or a request is rejected it degrades to a plain
 *   toggleable UI flag instead of throwing.
 * - `toggle()` / `exit()` never reject.
 */
export function useFullscreen() {
  async function toggle(): Promise<void> {
    if (pending) return
    if (!supported()) {
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
    if (!supported() || !document.fullscreenElement) {
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

  return { isFullscreen, toggle, exit }
}
