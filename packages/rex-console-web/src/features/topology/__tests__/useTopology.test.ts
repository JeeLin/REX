import { describe, it, expect, vi, beforeEach } from 'vitest'
import { api } from '@/api/client'
import { useTopology } from '../useTopology'

vi.mock('@/api/client', () => ({
  api: {
    get: vi.fn(),
  },
}))

describe('useTopology', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('initializes with empty state', () => {
    const { nodes, edges, loading, error } = useTopology()
    expect(nodes.value).toEqual([])
    expect(edges.value).toEqual([])
    expect(loading.value).toBe(false)
    expect(error.value).toBeNull()
  })

  it('loads topology data on refresh', async () => {
    const mockData = {
      nodes: [
        { id: 'env-1', type: 'environment', label: 'prod', status: 'online' },
        { id: 'resource-1', type: 'resource', label: 'web', status: 'connected', protocol: 'ssh' },
      ],
      edges: [
        { id: 'e1', source: 'env-1', target: 'resource-1', type: 'has_resource' },
      ],
    }
    ;(api.get as any).mockResolvedValue(mockData)

    const { nodes, edges, loading, error, refresh } = useTopology()
    await refresh()

    expect(api.get).toHaveBeenCalledWith('/environments/topology')
    expect(nodes.value).toEqual(mockData.nodes)
    expect(edges.value).toEqual(mockData.edges)
    expect(loading.value).toBe(false)
    expect(error.value).toBeNull()
  })

  it('sets error on fetch failure', async () => {
    ;(api.get as any).mockRejectedValue(new Error('network'))

    const { error, loading, refresh } = useTopology()
    await refresh()

    expect(error.value).toBe('network')
    expect(loading.value).toBe(false)
  })
})
