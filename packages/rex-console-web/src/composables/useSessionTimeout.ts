import { ref, onMounted, onBeforeUnmount } from 'vue'

/**
 * Session timeout composable — monitors user activity and triggers
 * auto-logout after a configurable idle period. Shows a warning
 * dialog 60 seconds before expiry.
 *
 * Usage:
 *   const { showWarning, remainingSeconds, extendSession, logout, stop } =
 *     useSessionTimeout(timeoutMinutes, onLogout)
 */
export function useSessionTimeout(
  timeoutMinutes: number,
  onLogout: () => void,
) {
  const showWarning = ref(false)
  const remainingSeconds = ref(0)

  let idleTimer: ReturnType<typeof setInterval> | null = null
  let warningTimer: ReturnType<typeof setInterval> | null = null
  let lastActivity = Date.now()
  let stopped = false

  const ACTIVITY_EVENTS = ['mousedown', 'keydown', 'scroll', 'touchstart']
  const CHECK_INTERVAL = 5_000 // 5 seconds
  const WARNING_BEFORE_MS = 60_000 // warn 60s before timeout

  function onActivity() {
    if (stopped) return
    lastActivity = Date.now()
    // If warning is showing and user interacts, extend session
    if (showWarning.value) {
      extendSession()
    }
  }

  function startWarningCountdown() {
    remainingSeconds.value = 60
    if (warningTimer) clearInterval(warningTimer)
    warningTimer = setInterval(() => {
      remainingSeconds.value--
      if (remainingSeconds.value <= 0) {
        logout()
      }
    }, 1_000)
  }

  function logout() {
    stop()
    onLogout()
  }

  function extendSession() {
    showWarning.value = false
    remainingSeconds.value = 0
    if (warningTimer) {
      clearInterval(warningTimer)
      warningTimer = null
    }
    lastActivity = Date.now()
  }

  function checkIdle() {
    if (stopped) return
    const idleMs = Date.now() - lastActivity
    const timeoutMs = timeoutMinutes * 60 * 1_000

    if (idleMs >= timeoutMs) {
      logout()
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

  function stop() {
    stopped = true
    stopMonitoring()
    if (warningTimer) {
      clearInterval(warningTimer)
      warningTimer = null
    }
    showWarning.value = false
    remainingSeconds.value = 0
  }

  onMounted(() => {
    stopped = false
    lastActivity = Date.now()
    startMonitoring()
  })

  onBeforeUnmount(stop)

  return { showWarning, remainingSeconds, extendSession, logout, stop }
}
