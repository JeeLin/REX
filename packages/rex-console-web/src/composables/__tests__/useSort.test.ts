import { describe, it, expect } from 'vitest'
import { useSort } from '../useSort'

describe('useSort', () => {
  const items = [
    { name: 'Charlie', age: 30 },
    { name: 'Alice', age: 25 },
    { name: 'Bob', age: 35 },
  ]

  it('returns unsorted items by default', () => {
    const { sorted } = useSort(() => items)
    expect(sorted.value).toEqual(items)
  })

  it('sorts by key ascending', () => {
    const { sorted, toggleSort } = useSort(() => items)
    toggleSort('name')
    expect(sorted.value.map(i => i.name)).toEqual(['Alice', 'Bob', 'Charlie'])
  })

  it('sorts by key descending', () => {
    const { sorted, toggleSort } = useSort(() => items)
    toggleSort('name')
    toggleSort('name')
    expect(sorted.value.map(i => i.name)).toEqual(['Charlie', 'Bob', 'Alice'])
  })

  it('clears sort on third toggle', () => {
    const { sorted, sortKey, sortDir, toggleSort } = useSort(() => items)
    toggleSort('name')
    toggleSort('name')
    toggleSort('name')
    expect(sortKey.value).toBe('')
    expect(sortDir.value).toBeNull()
    expect(sorted.value).toEqual(items)
  })

  it('sorts numbers correctly', () => {
    const { sorted, toggleSort } = useSort(() => items)
    toggleSort('age')
    expect(sorted.value.map(i => i.age)).toEqual([25, 30, 35])
  })

  it('setSort sets key and direction directly', () => {
    const { sorted, setSort } = useSort(() => items)
    setSort('name', 'desc')
    expect(sorted.value.map(i => i.name)).toEqual(['Charlie', 'Bob', 'Alice'])
  })

  it('handles null values', () => {
    const itemsWithNull = [
      { name: 'Bob', age: 30 },
      { name: null, age: 25 },
      { name: 'Alice', age: 35 },
    ]
    const { sorted, toggleSort } = useSort(() => itemsWithNull)
    toggleSort('name')
    expect(sorted.value.map(i => i.name)).toEqual(['Alice', 'Bob', null])
  })

  it('switching sort key resets direction to asc', () => {
    const { sortKey, sortDir, toggleSort } = useSort(() => items)
    toggleSort('name')
    toggleSort('name')
    expect(sortDir.value).toBe('desc')
    toggleSort('age')
    expect(sortKey.value).toBe('age')
    expect(sortDir.value).toBe('asc')
  })

  it('uses default key and direction', () => {
    const { sorted, sortKey, sortDir } = useSort(() => items, 'name', 'asc')
    expect(sortKey.value).toBe('name')
    expect(sortDir.value).toBe('asc')
    expect(sorted.value.map(i => i.name)).toEqual(['Alice', 'Bob', 'Charlie'])
  })
})
