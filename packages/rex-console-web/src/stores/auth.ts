import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/api/client'
import type { AuthCheckResponse, LoginResponse } from '@/types/auth'

/** 从 localStorage 或 sessionStorage 读取 token */
function readToken(): string | null {
  return localStorage.getItem('rex-token') || sessionStorage.getItem('rex-token')
}

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(readToken())
  const requiresSetup = ref(false)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const isAuthenticated = computed(() => !!token.value)

  /** 检查认证状态（页面加载时调用） */
  async function checkAuth() {
    try {
      const res = await api.request<AuthCheckResponse>('/auth/check')
      requiresSetup.value = res.requires_setup
    } catch (e) {
      if (e instanceof Error && e.name === 'AuthError') {
        // 401 时：如果之前有 token（说明是过期），不清除——触发弹窗
        // 如果之前没有 token（未登录），清除并保持 requiresSetup=false
        const hadToken = localStorage.getItem('rex-token') || sessionStorage.getItem('rex-token')
        if (hadToken) {
          // 保留旧 token 在存储中，router guard 会检测到并触发弹窗
          // 但设置 isAuthenticated=false 让 router guard 生效
          token.value = null
          requiresSetup.value = false
          return
        }
        requiresSetup.value = false
        return
      }
      requiresSetup.value = true
    }
  }

  /** 首次设置密码 */
  async function setupPassword(password: string) {
    loading.value = true
    error.value = null
    try {
      const res = await api.post<LoginResponse>('/auth/password', { password })
      token.value = res.token
      localStorage.setItem('rex-token', res.token)
      requiresSetup.value = false
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  /** 登录 */
  async function login(password: string, remember = true) {
    loading.value = true
    error.value = null
    try {
      const res = await api.post<LoginResponse>('/auth/login', { password })
      token.value = res.token
      // 清除两个存储位置的旧 token
      localStorage.removeItem('rex-token')
      sessionStorage.removeItem('rex-token')
      // remember=true → localStorage（持久化），false → sessionStorage（会话级）
      if (remember) {
        localStorage.setItem('rex-token', res.token)
      } else {
        sessionStorage.setItem('rex-token', res.token)
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  /** Token 刷新（用于过期弹窗恢复） */
  function setToken(newToken: string) {
    token.value = newToken
    localStorage.setItem('rex-token', newToken)
    sessionStorage.removeItem('rex-token')
  }

  /** 登出 */
  function logout() {
    token.value = null
    localStorage.removeItem('rex-token')
    sessionStorage.removeItem('rex-token')
  }

  return {
    token, requiresSetup, loading, error, isAuthenticated,
    checkAuth, setupPassword, login, setToken, logout,
  }
})
