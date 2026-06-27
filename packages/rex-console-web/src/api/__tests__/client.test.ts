import { describe, it, expect, vi } from 'vitest'

// Mock router to avoid import chain issues
vi.mock('@/router', () => ({
  default: { push: vi.fn() },
}))

describe('API client', () => {
  it('exports an axios instance with correct config', async () => {
    const { default: client } = await import('../client')
    expect(client).toBeDefined()
    expect(client.defaults.baseURL).toBe('/api')
    expect(client.defaults.timeout).toBe(15_000)
  })

  it('has request and response interceptors', async () => {
    const { default: client } = await import('../client')
    expect(client.interceptors.request.handlers.length).toBeGreaterThan(0)
    expect(client.interceptors.response.handlers.length).toBeGreaterThan(0)
  })
})
