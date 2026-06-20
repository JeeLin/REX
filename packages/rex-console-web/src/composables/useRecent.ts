import { ref } from 'vue'

export interface RecentItem {
  resourceId: string
  name: string
  protocol: string
  envName: string
  usedAt: number
}

const STORAGE_KEY = 'rex-recent'
const MAX_ITEMS = 10

const recent = ref<RecentItem[]>(loadFromStorage())

function loadFromStorage(): RecentItem[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    return raw ? JSON.parse(raw) : []
  } catch {
    return []
  }
}

function saveToStorage() {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(recent.value))
}

export function useRecent() {
  function addToRecent(item: Omit<RecentItem, 'usedAt'>) {
    // Remove existing entry for same resource
    const filtered = recent.value.filter((r) => r.resourceId !== item.resourceId)
    // Add to front with timestamp
    filtered.unshift({ ...item, usedAt: Date.now() })
    // Trim to max
    recent.value = filtered.slice(0, MAX_ITEMS)
    saveToStorage()
  }

  function clearRecent() {
    recent.value = []
    saveToStorage()
  }

  return { recent, addToRecent, clearRecent }
}
