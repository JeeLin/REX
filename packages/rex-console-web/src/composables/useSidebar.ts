import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import type { EnvWithResources } from '@/api/env'
import { listEnvsWithResources } from '@/api/env'
import { useProtocol } from './useProtocol'
import { useRecent } from './useRecent'

const COLLAPSED_KEY = 'rex-sidebar-collapsed'
const EXPANDED_ENVS_KEY = 'rex-sidebar-expanded-envs'
const FAVORITES_KEY = 'rex-sidebar-favorites'

// ── 模块级共享状态 ──────────────────────────────────────
// 所有 useSidebar() 调用者共享同一份 refs
const collapsed = ref<boolean>(localStorage.getItem(COLLAPSED_KEY) === 'true')
const searchQuery = ref('')
const envs = ref<EnvWithResources[]>([])
const loading = ref(false)
const mobileOpen = ref(false)
const favorites = ref<Set<string>>(loadFavorites())

// ── 本地存储工具 ────────────────────────────────────────

function loadFavorites(): Set<string> {
  try {
    const raw = localStorage.getItem(FAVORITES_KEY)
    return raw ? new Set(JSON.parse(raw)) : new Set()
  } catch {
    return new Set()
  }
}

function loadExpandedEnvs(): Set<string> {
  try {
    const raw = localStorage.getItem(EXPANDED_ENVS_KEY)
    return raw ? new Set(JSON.parse(raw)) : new Set()
  } catch {
    return new Set()
  }
}

const expandedEnvIds = ref<Set<string>>(loadExpandedEnvs())

function saveFavorites() {
  localStorage.setItem(FAVORITES_KEY, JSON.stringify([...favorites.value]))
}

function saveExpandedEnvs() {
  localStorage.setItem(EXPANDED_ENVS_KEY, JSON.stringify([...expandedEnvIds.value]))
}

// ── Composable ──────────────────────────────────────────

export function useSidebar() {
  const router = useRouter()
  useProtocol()
  const { addToRecent } = useRecent()

  // ── Favorites ──

  function addFavorite(resourceId: string) {
    if (favorites.value.has(resourceId)) return
    favorites.value.add(resourceId)
    favorites.value = new Set(favorites.value)
    saveFavorites()
  }

  function removeFavorite(resourceId: string) {
    if (!favorites.value.has(resourceId)) return
    favorites.value.delete(resourceId)
    favorites.value = new Set(favorites.value)
    saveFavorites()
  }

  function isFavorite(resourceId: string): boolean {
    return favorites.value.has(resourceId)
  }

  const favoriteResources = computed(() => {
    const result: Array<{ id: string; name: string; protocol: string; envName: string }> = []
    for (const env of envs.value) {
      for (const r of env.resources) {
        if (favorites.value.has(r.id)) {
          result.push({ id: r.id, name: r.name, protocol: r.protocol, envName: env.name })
        }
      }
    }
    return result
  })

  // ── Collapse ──

  function toggleCollapse() {
    collapsed.value = !collapsed.value
    localStorage.setItem(COLLAPSED_KEY, String(collapsed.value))
  }

  // ── Env expand ──

  function toggleEnvExpand(envId: string) {
    if (expandedEnvIds.value.has(envId)) {
      expandedEnvIds.value.delete(envId)
    } else {
      expandedEnvIds.value.add(envId)
    }
    expandedEnvIds.value = new Set(expandedEnvIds.value)
    saveExpandedEnvs()
  }

  function isEnvExpanded(envId: string): boolean {
    return expandedEnvIds.value.has(envId)
  }

  // ── Fetch ──

  async function fetchEnvs() {
    loading.value = true
    try {
      envs.value = await listEnvsWithResources()
      if (expandedEnvIds.value.size === 0 && envs.value.length > 0) {
        expandedEnvIds.value = new Set([envs.value[0].id])
        saveExpandedEnvs()
      }
    } catch {
      // silent
    } finally {
      loading.value = false
    }
  }

  // ── Connect ──

  function connectToResource(resource: { id: string; protocol: string; name: string }, envName: string) {
    addToRecent({ resourceId: resource.id, name: resource.name, protocol: resource.protocol, envName })
    if (router.currentRoute.value.name === 'workspace') {
      window.dispatchEvent(new CustomEvent('open-in-workspace', {
        detail: { resource },
      }))
    } else {
      router.push({
        name: 'workspace',
        query: { open: resource.id, name: resource.name, proto: resource.protocol },
      })
    }
    mobileOpen.value = false
  }

  // ── Search ──

  const filteredEnvs = computed(() => {
    const q = searchQuery.value.toLowerCase().trim()
    if (!q) return envs.value
    return envs.value
      .map((env) => ({
        ...env,
        resources: env.resources.filter((r) => r.name.toLowerCase().includes(q)),
      }))
      .filter((env) => env.name.toLowerCase().includes(q) || env.resources.length > 0)
  })

  function closeMobile() {
    mobileOpen.value = false
  }

  return {
    collapsed,
    searchQuery,
    expandedEnvIds,
    envs,
    filteredEnvs,
    loading,
    mobileOpen,
    favorites,
    favoriteResources,
    toggleCollapse,
    toggleEnvExpand,
    isEnvExpanded,
    fetchEnvs,
    connectToResource,
    addFavorite,
    removeFavorite,
    isFavorite,
    closeMobile,
  }
}
