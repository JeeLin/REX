import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import type { EnvWithResources } from '@/api/env'
import { listEnvsWithResources } from '@/api/env'
import { useProtocol } from './useProtocol'

const COLLAPSED_KEY = 'rex-sidebar-collapsed'
const EXPANDED_ENVS_KEY = 'rex-sidebar-expanded-envs'

export function useSidebar() {
  const router = useRouter()
  const { connectToResource: connect } = useProtocol()

  const collapsed = ref(loadCollapsed())
  const searchQuery = ref('')
  const expandedEnvIds = ref<Set<string>>(loadExpandedEnvs())
  const envs = ref<EnvWithResources[]>([])
  const loading = ref(false)
  const mobileOpen = ref(false)

  function loadCollapsed(): boolean {
    return localStorage.getItem(COLLAPSED_KEY) === 'true'
  }

  function loadExpandedEnvs(): Set<string> {
    try {
      const raw = localStorage.getItem(EXPANDED_ENVS_KEY)
      return raw ? new Set(JSON.parse(raw)) : new Set()
    } catch {
      return new Set()
    }
  }

  function saveExpandedEnvs() {
    localStorage.setItem(EXPANDED_ENVS_KEY, JSON.stringify([...expandedEnvIds.value]))
  }

  function toggleCollapse() {
    collapsed.value = !collapsed.value
    localStorage.setItem(COLLAPSED_KEY, String(collapsed.value))
  }

  function toggleEnvExpand(envId: string) {
    if (expandedEnvIds.value.has(envId)) {
      expandedEnvIds.value.delete(envId)
    } else {
      expandedEnvIds.value.add(envId)
    }
    // Trigger reactivity
    expandedEnvIds.value = new Set(expandedEnvIds.value)
    saveExpandedEnvs()
  }

  function isEnvExpanded(envId: string): boolean {
    return expandedEnvIds.value.has(envId)
  }

  async function fetchEnvs() {
    loading.value = true
    try {
      envs.value = await listEnvsWithResources()
      // Auto-expand first env if none expanded
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

  function connectToResource(resource: { id: string; protocol: string; name: string }, envName: string) {
    // 如果已在工作空间页面，直接通知
    if (router.currentRoute.value.name === 'workspace') {
      window.dispatchEvent(new CustomEvent('open-in-workspace', {
        detail: { resource },
      }))
    } else {
      // 导航到工作空间，通过 query 传递资源信息
      router.push({
        name: 'workspace',
        query: { open: resource.id, name: resource.name, proto: resource.protocol },
      })
    }
    mobileOpen.value = false
  }

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
    toggleCollapse,
    toggleEnvExpand,
    isEnvExpanded,
    fetchEnvs,
    connectToResource,
    closeMobile,
  }
}
