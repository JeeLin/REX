import { ref } from 'vue'
import { api } from '@/api/client'

export interface TopoNode {
  id: string
  type: 'environment' | 'agent' | 'resource'
  label: string
  status: string
  protocol?: string
  metadata?: Record<string, any>
}

export interface TopoEdge {
  id: string
  source: string
  target: string
  type: string
}

export interface Topology {
  nodes: TopoNode[]
  edges: TopoEdge[]
}

export function useTopology() {
  const nodes = ref<TopoNode[]>([])
  const edges = ref<TopoEdge[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function refresh() {
    loading.value = true
    error.value = null
    try {
      const data = await api.get<Topology>('/environments/topology')
      nodes.value = data.nodes
      edges.value = data.edges
    } catch (e: any) {
      error.value = e.message || 'Failed to load topology'
    } finally {
      loading.value = false
    }
  }

  return { nodes, edges, loading, error, refresh }
}
