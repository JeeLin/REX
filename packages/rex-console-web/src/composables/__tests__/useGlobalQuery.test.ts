import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useGlobalQuery } from '../useGlobalQuery'

describe('useGlobalQuery', () => {
  const resources = [
    { id: 'r1', name: 'MySQL Dev', protocol: 'mysql' },
    { id: 'r2', name: 'MySQL Prod', protocol: 'mysql' },
    { id: 'r3', name: 'PG Dev', protocol: 'postgresql' },
  ]

  beforeEach(() => {
    vi.restoreAllMocks()
  })

  it('starts with empty state', () => {
    const { selectedResources, sqlQuery, isExecuting, results } = useGlobalQuery(resources)
    expect(selectedResources.value).toEqual([])
    expect(sqlQuery.value).toBe('')
    expect(isExecuting.value).toBe(false)
    expect(results.value).toEqual([])
  })

  it('checkCompatibility returns true when no resources selected', () => {
    const { checkCompatibility } = useGlobalQuery(resources)
    expect(checkCompatibility('mysql')).toBe(true)
  })

  it('checkCompatibility returns true for same protocol', () => {
    const { selectedResources, checkCompatibility } = useGlobalQuery(resources)
    selectedResources.value = ['r1']
    expect(checkCompatibility('mysql')).toBe(true)
  })

  it('checkCompatibility returns false for different protocol', () => {
    const { selectedResources, checkCompatibility } = useGlobalQuery(resources)
    selectedResources.value = ['r1']
    expect(checkCompatibility('postgresql')).toBe(false)
  })

  it('selectAllCompatible selects all resources with same protocol', () => {
    const { selectedResources, selectAllCompatible } = useGlobalQuery(resources)
    selectedResources.value = ['r1']
    selectAllCompatible()
    expect(selectedResources.value).toEqual(['r1', 'r2'])
  })

  it('does not execute when no resources selected', async () => {
    const { executeGlobalQuery, isExecuting } = useGlobalQuery(resources)
    const fetchSpy = vi.spyOn(globalThis, 'fetch')
    await executeGlobalQuery()
    expect(fetchSpy).not.toHaveBeenCalled()
    expect(isExecuting.value).toBe(false)
  })

  it('does not execute when SQL is empty', async () => {
    const { selectedResources, executeGlobalQuery, isExecuting } = useGlobalQuery(resources)
    selectedResources.value = ['r1']
    const fetchSpy = vi.spyOn(globalThis, 'fetch')
    await executeGlobalQuery()
    expect(fetchSpy).not.toHaveBeenCalled()
    expect(isExecuting.value).toBe(false)
  })

  it('cancelQuery stops execution', () => {
    const { isExecuting, cancelQuery } = useGlobalQuery(resources)
    isExecuting.value = true
    cancelQuery()
    expect(isExecuting.value).toBe(false)
  })

})
