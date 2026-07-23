import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAuthStore } from '../auth'

// Mock the API client
vi.mock('@/api/client', () => ({
  api: {
    request: vi.fn(),
    post: vi.fn(),
  },
}))

import { api } from '@/api/client'
const mockApi = vi.mocked(api)

describe('auth store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    vi.clearAllMocks()
  })

  describe('initial state', () => {
    it('should have null token initially', () => {
      const store = useAuthStore()
      expect(store.token).toBeNull()
      expect(store.isAuthenticated).toBe(false)
    })

    it('should load token from localStorage', () => {
      localStorage.setItem('rex-token', 'saved-token')
      const store = useAuthStore()
      expect(store.token).toBe('saved-token')
      expect(store.isAuthenticated).toBe(true)
    })
  })

  describe('checkAuth', () => {
    it('should set requiresSetup from API response', async () => {
      mockApi.request.mockResolvedValue({ requires_setup: true })
      const store = useAuthStore()
      await store.checkAuth()
      expect(store.requiresSetup).toBe(true)
    })

    it('should handle API failure gracefully', async () => {
      mockApi.request.mockRejectedValue(new Error('network error'))
      const store = useAuthStore()
      await store.checkAuth()
      expect(store.requiresSetup).toBe(true)
    })
  })

  describe('login', () => {
    it('should store token on successful login', async () => {
      mockApi.post.mockResolvedValue({ token: 'new-token', expiresAt: '2025-12-31' })
      const store = useAuthStore()
      await store.login('password')
      expect(store.token).toBe('new-token')
      expect(localStorage.getItem('rex-token')).toBe('new-token')
      expect(store.isAuthenticated).toBe(true)
    })

    it('should set error on failed login', async () => {
      mockApi.post.mockRejectedValue(new Error('invalid password'))
      const store = useAuthStore()
      await expect(store.login('wrong')).rejects.toThrow('invalid password')
      expect(store.error).toBe('invalid password')
      expect(store.isAuthenticated).toBe(false)
    })

    it('should set loading state correctly', async () => {
      mockApi.post.mockImplementation(() => new Promise(resolve => setTimeout(() => resolve({ token: 't', expiresAt: '' }), 100)))
      const store = useAuthStore()
      const loginPromise = store.login('pw')
      expect(store.loading).toBe(true)
      await loginPromise
      expect(store.loading).toBe(false)
    })
  })

  describe('setupPassword', () => {
    it('should store token and clear requiresSetup', async () => {
      mockApi.post.mockResolvedValue({ token: 'setup-token', expiresAt: '2025-12-31' })
      const store = useAuthStore()
      store.requiresSetup = true
      await store.setupPassword('newpass')
      expect(store.token).toBe('setup-token')
      expect(store.requiresSetup).toBe(false)
    })
  })

  describe('logout', () => {
    it('should clear token', () => {
      localStorage.setItem('rex-token', 'to-be-removed')
      const store = useAuthStore()
      store.token = 'to-be-removed'
      store.logout()
      expect(store.token).toBeNull()
      expect(localStorage.getItem('rex-token')).toBeNull()
      expect(store.isAuthenticated).toBe(false)
    })
  })
})
