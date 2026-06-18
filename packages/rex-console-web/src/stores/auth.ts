import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { loginApi } from '@/api/auth'

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem('rex-token'))
  const expiresAt = ref<string | null>(localStorage.getItem('rex-expires-at'))

  const isLoggedIn = computed(() => {
    if (!token.value) return false
    if (expiresAt.value) {
      const expMs = Number(expiresAt.value) * 1000
      if (Date.now() >= expMs) {
        logout()
        return false
      }
    }
    return true
  })

  async function login(password: string) {
    const result = await loginApi({ password })
    token.value = result.token
    expiresAt.value = result.expires_at
    localStorage.setItem('rex-token', result.token)
    localStorage.setItem('rex-expires-at', result.expires_at)
  }

  function logout() {
    token.value = null
    expiresAt.value = null
    localStorage.removeItem('rex-token')
    localStorage.removeItem('rex-expires-at')
  }

  return { token, expiresAt, isLoggedIn, login, logout }
})
