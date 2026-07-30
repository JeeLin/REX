import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { api, AuthError, ApiError, tokenRefreshEvent } from '../client'

describe('ApiClient', () => {
  let fetchSpy: ReturnType<typeof vi.fn>

  beforeEach(() => {
    fetchSpy = vi.fn()
    global.fetch = fetchSpy as typeof global.fetch
    localStorage.setItem('rex-token', 'test-token')
  })

  afterEach(() => {
    vi.restoreAllMocks()
    localStorage.clear()
  })

  it('throws AuthError on 401 for non-auth endpoints', async () => {
    fetchSpy.mockResolvedValue({ ok: false, status: 401 })
    await expect(api.request('/test')).rejects.toThrow(AuthError)
  })

  it('throws AuthError on 401 for auth endpoints without triggering modal', async () => {
    fetchSpy.mockResolvedValue({ ok: false, status: 401 })
    const handler = vi.fn()
    tokenRefreshEvent.addEventListener('unauthorized', handler)

    await expect(api.post('/auth/login', { password: 'wrong' })).rejects.toThrow(AuthError)
    expect(handler).not.toHaveBeenCalled()

    tokenRefreshEvent.removeEventListener('unauthorized', handler)
  })

  it('throws ApiError on non-401 error', async () => {
    fetchSpy.mockResolvedValue({
      ok: false,
      status: 500,
      json: () => Promise.resolve({ error: { code: 'ERROR', message: 'test' } }),
    })
    await expect(api.request('/test')).rejects.toThrow(ApiError)
  })

  it('includes Authorization header', async () => {
    fetchSpy.mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve({}) })
    await api.request('/test')
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/test',
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: 'Bearer test-token',
        }),
      }),
    )
  })
})
