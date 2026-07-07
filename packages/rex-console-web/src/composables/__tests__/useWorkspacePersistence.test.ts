import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

describe('useWorkspacePersistence', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    localStorage.clear()
    vi.resetModules()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  async function getPersistence() {
    const { useWorkspacePersistence } = await import('../useWorkspacePersistence')
    return useWorkspacePersistence()
  }

  it('restore returns false when no stored state', async () => {
    const { restore } = await getPersistence()
    expect(restore()).toBe(false)
  })

  it('restore returns false for expired state', async () => {
    const state = {
      version: 1,
      tabs: [{ name: 'Tab1', proto: 'ssh', resourceId: 'r1', panelIndex: 0 }],
      activeTabId: null,
      timestamp: Date.now() - 25 * 60 * 60 * 1000, // 25 hours ago
    }
    localStorage.setItem('rex-workspace-state', JSON.stringify(state))
    const { restore } = await getPersistence()
    expect(restore()).toBe(false)
    expect(localStorage.getItem('rex-workspace-state')).toBeNull()
  })

  it('restore returns false for invalid JSON', async () => {
    localStorage.setItem('rex-workspace-state', 'not-json')
    const { restore } = await getPersistence()
    expect(restore()).toBe(false)
  })

  it('restore returns false for missing tabs array', async () => {
    localStorage.setItem('rex-workspace-state', JSON.stringify({ version: 1, tabs: null }))
    const { restore } = await getPersistence()
    expect(restore()).toBe(false)
  })

  it('save persists state to localStorage', async () => {
    const { save } = await getPersistence()
    save()
    const stored = localStorage.getItem('rex-workspace-state')
    expect(stored).toBeTruthy()
    const parsed = JSON.parse(stored!)
    expect(parsed.version).toBe(1)
    expect(parsed.tabs).toEqual([])
    expect(parsed.timestamp).toBeGreaterThan(0)
  })
})
