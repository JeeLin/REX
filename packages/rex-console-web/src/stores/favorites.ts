import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

export interface RecentItem {
  id: string
  name: string
  protocol: string
  time: number
}

const FAVORITES_KEY = 'rex-favorites'
const RECENT_KEY = 'rex-recent'
const MAX_RECENT = 20

function loadStringSet(key: string): string[] {
  try {
    const raw = localStorage.getItem(key)
    return raw ? JSON.parse(raw) : []
  } catch {
    return []
  }
}

function loadRecent(key: string): RecentItem[] {
  try {
    const raw = localStorage.getItem(key)
    return raw ? JSON.parse(raw) : []
  } catch {
    return []
  }
}

export const useFavoritesStore = defineStore('favorites', () => {
  const favorites = ref<string[]>(loadStringSet(FAVORITES_KEY))
  const recent = ref<RecentItem[]>(loadRecent(RECENT_KEY))

  watch(favorites, (val) => {
    localStorage.setItem(FAVORITES_KEY, JSON.stringify(val))
  }, { deep: true })

  watch(recent, (val) => {
    localStorage.setItem(RECENT_KEY, JSON.stringify(val))
  }, { deep: true })

  function toggleFavorite(id: string) {
    const idx = favorites.value.indexOf(id)
    if (idx >= 0) {
      favorites.value.splice(idx, 1)
    } else {
      favorites.value.push(id)
    }
  }

  function isFavorite(id: string): boolean {
    return favorites.value.includes(id)
  }

  function addRecent(resource: { id: string; name: string; protocol: string }) {
    const existing = recent.value.findIndex(r => r.id === resource.id)
    if (existing >= 0) {
      recent.value.splice(existing, 1)
    }
    recent.value.unshift({ ...resource, time: Date.now() })
    if (recent.value.length > MAX_RECENT) {
      recent.value = recent.value.slice(0, MAX_RECENT)
    }
  }

  return { favorites, recent, toggleFavorite, isFavorite, addRecent }
})
