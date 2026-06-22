import { ref, onMounted, onUnmounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { securitySettings } from '@/stores/settings'

const lastActivity = ref(Date.now())
let timer: ReturnType<typeof setInterval> | null = null

function resetTimer() {
  lastActivity.value = Date.now()
}

function handleActivity() {
  resetTimer()
}

export function useSessionTimeout() {
  const router = useRouter()

  function startTimer() {
    stopTimer()
    timer = setInterval(() => {
      if (securitySettings.sessionTimeout === 0) return // "never"
      const elapsed = (Date.now() - lastActivity.value) / 1000 / 60 // minutes
      if (elapsed >= securitySettings.sessionTimeout) {
        localStorage.removeItem('rex-token')
        localStorage.removeItem('rex-expires-at')
        router.push('/login')
      }
    }, 30_000) // check every 30s
  }

  function stopTimer() {
    if (timer) {
      clearInterval(timer)
      timer = null
    }
  }

  onMounted(() => {
    window.addEventListener('mousemove', handleActivity)
    window.addEventListener('keydown', handleActivity)
    window.addEventListener('touchstart', handleActivity)
    startTimer()
  })

  onUnmounted(() => {
    window.removeEventListener('mousemove', handleActivity)
    window.removeEventListener('keydown', handleActivity)
    window.removeEventListener('touchstart', handleActivity)
    stopTimer()
  })

  watch(() => securitySettings.sessionTimeout, () => {
    resetTimer()
    startTimer()
  })

  return { resetTimer }
}
