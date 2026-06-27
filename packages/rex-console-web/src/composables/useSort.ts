import { ref, computed } from 'vue'

export type SortDirection = 'asc' | 'desc' | null

export function useSort<T>(
  items: () => T[],
  defaultKey: string = '',
  defaultDir: SortDirection = null,
) {
  const sortKey = ref(defaultKey)
  const sortDir = ref<SortDirection>(defaultDir)

  function toggleSort(key: string) {
    if (sortKey.value === key) {
      // Cycle: asc → desc → null
      if (sortDir.value === 'asc') sortDir.value = 'desc'
      else if (sortDir.value === 'desc') { sortDir.value = null; sortKey.value = '' }
      else sortDir.value = 'asc'
    } else {
      sortKey.value = key
      sortDir.value = 'asc'
    }
  }

  function setSort(key: string, dir: SortDirection) {
    sortKey.value = key
    sortDir.value = dir
  }

  const sorted = computed(() => {
    const list = [...items()]
    if (!sortKey.value || !sortDir.value) return list

    const key = sortKey.value
    const dir = sortDir.value === 'asc' ? 1 : -1

    list.sort((a, b) => {
      const va = (a as Record<string, unknown>)[key]
      const vb = (b as Record<string, unknown>)[key]
      if (va == null && vb == null) return 0
      if (va == null) return 1
      if (vb == null) return -1
      if (typeof va === 'string' && typeof vb === 'string') return va.localeCompare(vb) * dir
      if (typeof va === 'number' && typeof vb === 'number') return (va - vb) * dir
      return String(va).localeCompare(String(vb)) * dir
    })

    return list
  })

  return { sortKey, sortDir, toggleSort, setSort, sorted }
}
