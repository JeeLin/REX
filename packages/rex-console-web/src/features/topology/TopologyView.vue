<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { VueFlow, useVueFlow } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import { MiniMap } from '@vue-flow/minimap'
import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'
import '@vue-flow/minimap/dist/style.css'

import TopologyNode from './TopologyNode.vue'
import TopologyLegend from './TopologyLegend.vue'
import { useTopology } from './useTopology'
import type { TopoNode, TopoEdge } from './useTopology'

const props = defineProps<{
  envId?: string
}>()

const router = useRouter()
const { nodes: topoNodes, edges: topoEdges, loading, error, refresh } = useTopology()
const { fitView, onNodeClick } = useVueFlow()

// Convert topology data to vue-flow format
const vfNodes = computed(() => {
  const positions = computeLayout(topoNodes.value, props.envId)
  return topoNodes.value.map((n) => ({
    id: n.id,
    type: 'topology',
    position: positions[n.id] || { x: 0, y: 0 },
    data: { node: n },
    draggable: true,
  }))
})

const vfEdges = computed(() => {
  return topoEdges.value.map((e) => ({
    id: e.id,
    source: e.source,
    target: e.target,
    type: 'smoothstep',
    animated: true,
    style: { stroke: 'var(--border-strong)', strokeWidth: 1.5 },
  }))
})

// Simple hierarchical layout: environments in center row, agents below, resources below agents
function computeLayout(nodes: TopoNode[], envId?: string): Record<string, { x: number; y: number }> {
  const pos: Record<string, { x: number; y: number }> = {}
  const envs = nodes.filter((n) => n.type === 'environment')
  const agents = nodes.filter((n) => n.type === 'agent')
  const resources = nodes.filter((n) => n.type === 'resource')

  // If filtering to one environment, arrange tightly
  const hSpacing = 220
  const vSpacing = 160
  const startX = 80

  // Environments row (y=0)
  envs.forEach((n, i) => {
    pos[n.id] = { x: startX + i * hSpacing, y: 0 }
  })

  // Agents row (y=vSpacing)
  agents.forEach((n, i) => {
    pos[n.id] = { x: startX + i * hSpacing, y: vSpacing }
  })

  // Resources row (y=2*vSpacing)
  resources.forEach((n, i) => {
    pos[n.id] = { x: startX + i * hSpacing, y: 2 * vSpacing }
  })

  return pos
}

// Node click handler
onNodeClick(({ node }) => {
  const topoNode: TopoNode | undefined = topoNodes.value.find((n) => n.id === node.id)
  if (!topoNode) return

  if (topoNode.type === 'environment') {
    const envUuid = topoNode.id.replace('env-', '')
    router.push(`/environments/${envUuid}`)
  } else if (topoNode.type === 'agent') {
    // Agent → navigate to its parent environment
    const edge = topoEdges.value.find(
      (e) => e.target === topoNode.id && e.type === 'has_agent',
    )
    if (edge) {
      const envUuid = edge.source.replace('env-', '')
      router.push(`/environments/${envUuid}`)
    }
  } else if (topoNode.type === 'resource') {
    router.push('/workspace')
  }
})

// Auto-refresh every 30 seconds
let refreshTimer: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  refresh()
  refreshTimer = setInterval(refresh, 30_000)
})

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer)
})

// Fit view after data loads
watch(
  () => loading.value,
  (isLoading) => {
    if (!isLoading && vfNodes.value.length > 0) {
      setTimeout(() => fitView({ padding: 0.2 }), 100)
    }
  },
)

// Re-fit when envId changes
watch(
  () => props.envId,
  () => {
    refresh()
  },
)
</script>

<template>
  <div class="topo-view">
    <!-- Header bar -->
    <div class="topo-header">
      <TopologyLegend />
      <div class="topo-header-spacer" />
      <span v-if="loading" class="topo-loading">Loading...</span>
      <span v-if="error" class="topo-error">{{ error }}</span>
    </div>

    <!-- Flow canvas -->
    <div class="topo-canvas">
      <VueFlow
        v-model:nodes="vfNodes"
        v-model:edges="vfEdges"
        :default-viewport="{ zoom: 0.8, x: 0, y: 0 }"
        :min-zoom="0.2"
        :max-zoom="2"
        fit-view-on-init
      >
        <template #node-topology="nodeProps">
          <TopologyNode v-bind="nodeProps" />
        </template>
        <Background :gap="20" :size="1" pattern-color="var(--border)" />
        <Controls position="bottom-right" />
        <MiniMap
          position="bottom-left"
          :pannable="true"
          :zoomable="true"
          :height="100"
          :width="160"
          node-color="#58A6FF"
        />
      </VueFlow>

      <!-- Empty state -->
      <div v-if="!loading && vfNodes.length === 0" class="topo-empty">
        <div class="topo-empty-icon">⛁</div>
        <div class="topo-empty-text">No topology data</div>
        <div class="topo-empty-sub">Environments, agents, and resources will appear here.</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.topo-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 400px;
}

.topo-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 0;
  margin-bottom: 8px;
}

.topo-header-spacer {
  flex: 1;
}

.topo-loading {
  font-size: 12px;
  font-family: var(--font-mono);
  color: var(--text-muted);
}

.topo-error {
  font-size: 12px;
  font-family: var(--font-mono);
  color: var(--danger);
}

.topo-canvas {
  flex: 1;
  position: relative;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-page);
  overflow: hidden;
}

.topo-empty {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

.topo-empty-icon {
  font-size: 32px;
  margin-bottom: 12px;
  opacity: 0.4;
}

.topo-empty-text {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.topo-empty-sub {
  font-size: 12px;
  color: var(--text-muted);
  opacity: 0.6;
}
</style>
