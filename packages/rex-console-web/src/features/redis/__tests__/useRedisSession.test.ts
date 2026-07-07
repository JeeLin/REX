import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { useRedisSession } from '../useRedisSession'

// Mock WebSocket
class MockWebSocket {
  static instances: MockWebSocket[] = []
  readyState = 1 // OPEN
  onopen: (() => void) | null = null
  onmessage: ((event: { data: string }) => void) | null = null
  onerror: (() => void) | null = null
  onclose: (() => void) | null = null
  sent: string[] = []

  constructor() {
    MockWebSocket.instances.push(this)
  }

  send(data: string) {
    this.sent.push(data)
  }

  close() {
    this.readyState = 3 // CLOSED
    this.onclose?.()
  }

  simulateMessage(data: object) {
    this.onmessage?.({ data: JSON.stringify(data) })
  }
}

// Mock location
Object.defineProperty(globalThis, 'location', {
  value: { protocol: 'http:', host: 'localhost:3000' },
})

describe('useRedisSession', () => {
  beforeEach(() => {
    MockWebSocket.instances = []
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('exports REDIS_COMMANDS list', () => {
    const session = useRedisSession(() => 'res-1')
    expect(session.REDIS_COMMANDS).toBeInstanceOf(Array)
    expect(session.REDIS_COMMANDS.length).toBeGreaterThan(0)
    expect(session.REDIS_COMMANDS).toContain('GET')
    expect(session.REDIS_COMMANDS).toContain('SET')
    expect(session.REDIS_COMMANDS).toContain('DEL')
  })

  it('starts disconnected', () => {
    const session = useRedisSession(() => 'res-1')
    expect(session.connected.value).toBe(false)
    expect(session.error.value).toBeNull()
    expect(session.serverInfo.value).toBeNull()
  })

  it('addToHistory adds entry and trims to 200', () => {
    const session = useRedisSession(() => 'res-1')

    session.addToHistory('GET foo')
    expect(session.history.value).toHaveLength(1)
    expect(session.history.value[0]!.command).toBe('GET foo')

    // Add 250 entries
    for (let i = 0; i < 250; i++) {
      session.addToHistory(`CMD ${i}`)
    }
    expect(session.history.value.length).toBeLessThanOrEqual(200)
  })

  it('historyUp navigates history', () => {
    const session = useRedisSession(() => 'res-1')
    session.addToHistory('CMD 1')
    session.addToHistory('CMD 2')
    session.addToHistory('CMD 3')

    expect(session.historyUp()).toBe('CMD 3')
    expect(session.historyUp()).toBe('CMD 2')
    expect(session.historyUp()).toBe('CMD 1')
    // Stay at the end
    expect(session.historyUp()).toBe('CMD 1')
  })

  it('historyDown navigates back', () => {
    const session = useRedisSession(() => 'res-1')
    session.addToHistory('CMD 1')
    session.addToHistory('CMD 2')

    session.historyUp() // -> CMD 2
    session.historyUp() // -> CMD 1
    expect(session.historyDown()).toBe('CMD 2')
    expect(session.historyDown()).toBe('')
  })

  it('historyUp returns null on empty history', () => {
    const session = useRedisSession(() => 'res-1')
    expect(session.historyUp()).toBeNull()
  })

  it('clearHistory resets state', () => {
    const session = useRedisSession(() => 'res-1')
    session.addToHistory('CMD 1')
    session.addToHistory('CMD 2')
    session.clearHistory()
    expect(session.history.value).toHaveLength(0)
  })

  it('disconnect cleans up state', () => {
    const session = useRedisSession(() => 'res-1')
    session.disconnect()
    expect(session.connected.value).toBe(false)
    expect(session.serverInfo.value).toBeNull()
  })
})
