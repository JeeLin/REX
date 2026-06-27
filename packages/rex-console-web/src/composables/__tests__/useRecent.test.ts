import { describe, it, expect } from 'vitest'
import { useRecent } from '../useRecent'

// localStorage is provided by happy-dom

describe('useRecent', () => {
  it('addToRecent saves item to localStorage', () => {
    localStorage.clear()
    const { addToRecent } = useRecent()
    addToRecent({ resourceId: 'r1', name: 'test-server', protocol: 'ssh', envName: 'production' })

    const stored = localStorage.getItem('rex-recent')
    expect(stored).toBeTruthy()
    const parsed = JSON.parse(stored!)
    expect(parsed.length).toBeGreaterThanOrEqual(1)
    expect(parsed.some((r: any) => r.resourceId === 'r1')).toBe(true)
  })

  it('clearRecent empties localStorage', () => {
    const { clearRecent } = useRecent()
    clearRecent()
    const stored = localStorage.getItem('rex-recent')
    expect(stored).toBe('[]')
  })

  it('addToRecent sets usedAt timestamp', () => {
    const { recent, addToRecent } = useRecent()
    addToRecent({ resourceId: 'r-ts', name: 'ts-test', protocol: 'redis', envName: 'dev' })

    const item = recent.value.find((r) => r.resourceId === 'r-ts')
    expect(item).toBeDefined()
    expect(item!.usedAt).toBeGreaterThan(0)
  })
})
