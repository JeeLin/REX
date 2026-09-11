<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
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

const { t } = useI18n()
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
    type: 'bezier',
    animated: true,
    style: { stroke: 'var(--border-strong)', strokeWidth: 1.5 },
  }))
})

// Radial layout: Hub/Environment at center, agents in inner ring, resources in outer ring
function computeLayout(nodes: TopoNode[], envId?: string): Record<string, { x: number; y: number }> {
  const pos: Record<string, { x: number; y: number }> = {}
  const cx = 500, cy = 350 // center

  if (envId) {
    // Single environment view: env at center, agents around it, resources further out
    const envs = nodes.filter((n) => n.type === 'environment')
    const agents = nodes.filter((n) => n.type === 'agent')
    const resources = nodes.filter((n) => n.type === 'resource')

    envs.forEach((n) => { pos[n.id] = { x: cx, y: cy } })

    const agentR = 200
    agents.forEach((n, i) => {
      const angle = (2 * Math.PI * i) / Math.max(agents.length, 1) - Math.PI / 2
      pos[n.id] = { x: cx + agentR * Math.cos(angle), y: cy + agentR * Math.sin(angle) }
    })

    const resR = 380
    resources.forEach((n, i) => {
      const angle = (2 * Math.PI * i) / Math.max(resources.length, 1) - Math.PI / 2
      pos[n.id] = { x: cx + resR * Math.cos(angle), y: cy + resR * Math.sin(angle) }
    })
  } else {
    // Full topology: environments in center ring, agents in middle, resources outer
    const envs = nodes.filter((n) => n.type === 'environment')
    const agents = nodes.filter((n) => n.type === 'agent')
    const resources = nodes.filter((n) => n.type === 'resource')

    // Environments at center (if single, at center; if multiple, small ring)
    if (envs.length === 1) {
      if (envs[0]) pos[envs[0].id] = { x: cx, y: cy }
    } else {
      envs.forEach((n, i) => {
        const angle = (2 * Math.PI * i) / envs.length - Math.PI / 2
        pos[n.id] = { x: cx + 60 * Math.cos(angle), y: cy + 60 * Math.sin(angle) }
      })
    }

    // Agents in middle ring
    const agentR = 240
    agents.forEach((n, i) => {
      const angle = (2 * Math.PI * i) / Math.max(agents.length, 1) - Math.PI / 2
      pos[n.id] = { x: cx + agentR * Math.cos(angle), y: cy + agentR * Math.sin(angle) }
    })

    // Resources in outer ring
    const resR = 420
    resources.forEach((n, i) => {
      const angle = (2 * Math.PI * i) / Math.max(resources.length, 1) - Math.PI / 2
      pos[n.id] = { x: cx + resR * Math.cos(angle), y: cy + resR * Math.sin(angle) }
    })
  }

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
      <span v-if="loading" class="topo-loading">{{ t('environments.topoLoading') }}</span>
      <span v-if="error" class="topo-error">{{ t('environments.topoError') }}: {{ error }}</span>
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
        <div class="topo-empty-text">{{ t('environments.topoEmpty') }}</div>
        <div class="topo-empty-sub">{{ t('environments.topoEmptySub') }}</div>
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

@media (max-width: 760px) {
  .topo-view {
    min-height: 300px;
  }

  .topo-header {
    flex-wrap: wrap;
    gap: 8px;
  }

  .topo-canvas {
    border-radius: var(--radius-lg);
  }
}
</style>
