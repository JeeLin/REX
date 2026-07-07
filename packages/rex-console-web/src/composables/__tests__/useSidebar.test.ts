import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('vue-router', () => ({
  useRouter: () => ({
    currentRoute: { value: { name: 'dashboard' } },
    push: vi.fn(),
  }),
}))

vi.mock('@/api/env', () => ({
  listEnvsWithResources: vi.fn().mockResolvedValue([]),
}))

describe('useSidebar', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.resetModules()
  })

  async function getSidebar() {
    const { useSidebar } = await import('../useSidebar')
    return useSidebar()
  }

  it('starts with collapsed state from localStorage', async () => {
    localStorage.setItem('rex-sidebar-collapsed', 'true')
    const { collapsed } = await getSidebar()
    expect(collapsed.value).toBe(true)
  })

  it('toggles collapse state', async () => {
    const { collapsed, toggleCollapse } = await getSidebar()
    expect(collapsed.value).toBe(false)
    toggleCollapse()
    expect(collapsed.value).toBe(true)
    expect(localStorage.getItem('rex-sidebar-collapsed')).toBe('true')
  })

  it('manages favorites', async () => {
    const { addFavorite, removeFavorite, isFavorite } = await getSidebar()
    expect(isFavorite('res-1')).toBe(false)
    addFavorite('res-1')
    expect(isFavorite('res-1')).toBe(true)
    removeFavorite('res-1')
    expect(isFavorite('res-1')).toBe(false)
  })

  it('persists favorites to localStorage', async () => {
    const { addFavorite } = await getSidebar()
    addFavorite('res-2')
    const stored = JSON.parse(localStorage.getItem('rex-sidebar-favorites') || '[]')
    expect(stored).toContain('res-2')
  })

  it('toggles env expand', async () => {
    const { toggleEnvExpand, isEnvExpanded } = await getSidebar()
    expect(isEnvExpanded('env-1')).toBe(false)
    toggleEnvExpand('env-1')
    expect(isEnvExpanded('env-1')).toBe(true)
    toggleEnvExpand('env-1')
    expect(isEnvExpanded('env-1')).toBe(false)
  })

  it('filters environments by search query', async () => {
    const { searchQuery, filteredEnvs, envs } = await getSidebar()
    // Manually set envs since fetchEnvs requires API
    envs.value = [
      { id: 'e1', name: 'Production', resources: [] } as unknown as typeof envs.value[number],
      { id: 'e2', name: 'Staging', resources: [] } as unknown as typeof envs.value[number],
    ]
    searchQuery.value = 'prod'
    expect(filteredEnvs.value).toHaveLength(1)
    expect(filteredEnvs.value[0]!.name).toBe('Production')
  })

  it('closes mobile sidebar', async () => {
    const { mobileOpen, closeMobile } = await getSidebar()
    mobileOpen.value = true
    closeMobile()
    expect(mobileOpen.value).toBe(false)
  })
})
