import { ref, readonly } from 'vue'

// --- Singleton session timeout manager ---
// Used by the router guard to start/stop monitoring.
// The warning dialog component reads from the exported refs.

const showWarning = ref(false)
const remainingSeconds = ref(0)

let idleTimer: ReturnType<typeof setInterval> | null = null
let warningTimer: ReturnType<typeof setInterval> | null = null
let lastActivity = Date.now()
let active = false
let timeoutMs = 30 * 60 * 1_000 // default 30 minutes
let onLogoutCallback: (() => void) | null = null

const ACTIVITY_EVENTS = ['mousedown', 'keydown', 'scroll', 'touchstart']
const CHECK_INTERVAL = 5_000
const WARNING_BEFORE_MS = 60_000

function onActivity() {
  if (!active) return
  lastActivity = Date.now()
  if (showWarning.value) {
    extendSession()
  }
}

function startWarningCountdown() {
  remainingSeconds.value = WARNING_BEFORE_MS / 1_000
  if (warningTimer) clearInterval(warningTimer)
  warningTimer = setInterval(() => {
    remainingSeconds.value--
    if (remainingSeconds.value <= 0) {
      doLogout()
    }
  }, 1_000)
}

function doLogout() {
  stopSession()
  onLogoutCallback?.()
}

function checkIdle() {
  if (!active) return
  const idleMs = Date.now() - lastActivity
  if (idleMs >= timeoutMs) {
    doLogout()
  } else if (idleMs >= timeoutMs - WARNING_BEFORE_MS && !showWarning.value) {
    showWarning.value = true
    startWarningCountdown()
  }
}

function startMonitoring() {
  stopMonitoring()
  ACTIVITY_EVENTS.forEach((e) =>
    document.addEventListener(e, onActivity, { passive: true }),
  )
  idleTimer = setInterval(checkIdle, CHECK_INTERVAL)
}

function stopMonitoring() {
  if (idleTimer) {
    clearInterval(idleTimer)
    idleTimer = null
  }
  ACTIVITY_EVENTS.forEach((e) =>
    document.removeEventListener(e, onActivity),
  )
}

/** Start session timeout monitoring. Called by the router guard after login. */
export function startSession(minutes: number, logoutFn: () => void) {
  stopSession()
  const safe = Number.isFinite(minutes) && minutes >= 1 ? minutes : 30
  timeoutMs = safe * 60 * 1_000
  onLogoutCallback = logoutFn
  active = true
  lastActivity = Date.now()
  startMonitoring()
}

/** Stop session timeout monitoring. Called on logout. */
export function stopSession() {
  active = false
  stopMonitoring()
  if (warningTimer) {
    clearInterval(warningTimer)
    warningTimer = null
  }
  showWarning.value = false
  remainingSeconds.value = 0
}

/** Extend the current session (reset idle timer). */
export function extendSession() {
  showWarning.value = false
  remainingSeconds.value = 0
  if (warningTimer) {
    clearInterval(warningTimer)
    warningTimer = null
  }
  lastActivity = Date.now()
}

// --- Composable for components (warning dialog) ---

/**
 * Composable that provides read-only access to session timeout state.
 * Use in the AppLayout or a global component to show the warning dialog.
 */
export function useSessionTimeout() {
  return {
    showWarning: readonly(showWarning),
    remainingSeconds: readonly(remainingSeconds),
    extendSession,
  }
}
