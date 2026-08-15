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

  it('get appends query params when provided', async () => {
    fetchSpy.mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve({}) })
    await api.get('/audit-log', { limit: '20', offset: '0' })
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/audit-log?limit=20&offset=0',
      expect.anything(),
    )
  })

  it('get omits query string when no params', async () => {
    fetchSpy.mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve({}) })
    await api.get('/audit-log')
    expect(fetchSpy).toHaveBeenCalledWith('/api/audit-log', expect.anything())
  })

  it('put serializes body as JSON', async () => {
    fetchSpy.mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve({}) })
    await api.put('/res/1', { name: 'x' })
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/res/1',
      expect.objectContaining({ method: 'PUT' }),
    )
  })

  it('del sends DELETE', async () => {
    fetchSpy.mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve({}) })
    await api.del('/res/1')
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/res/1',
      expect.objectContaining({ method: 'DELETE' }),
    )
  })

  it('upload sends FormData via POST with auth header', async () => {
    fetchSpy.mockResolvedValue({ ok: true, status: 200, json: () => Promise.resolve({}) })
    const fd = new FormData()
    fd.append('file', new Blob(['x']), 'f.txt')
    await api.upload('/files', fd)
    const call = fetchSpy.mock.calls[0]!
    expect(call[0]).toBe('/api/files')
    const opts = call[1] as RequestInit
    expect(opts.method).toBe('POST')
    expect(opts.body).toBe(fd)
    const headers = opts.headers as Record<string, string>
    expect(headers['Authorization']).toBe('Bearer test-token')
  })
})
