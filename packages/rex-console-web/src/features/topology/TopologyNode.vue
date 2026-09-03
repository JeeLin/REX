<script setup lang="ts">
import type { TopoNode } from './useTopology'

const props = defineProps<{
  data: {
    node: TopoNode
  }
}>()

const protocolColors: Record<string, string> = {
  ssh: '#3FB950',
  sftp: '#8B5CF6',
  sql: '#58A6FF',
  redis: '#F85149',
  s3: '#79C0FF',
  sip: '#3FB950',
}

function nodeColor(): string {
  const type = props.data.node.type
  if (type === 'environment') return '#58A6FF'
  if (type === 'agent') return '#79C0FF'
  if (type === 'resource' && props.data.node.protocol) {
    return protocolColors[props.data.node.protocol] || '#58A6FF'
  }
  return '#58A6FF'
}

function nodeIcon(): string {
  const type = props.data.node.type
  if (type === 'environment') return '◉'
  if (type === 'agent') return '⬡'
  if (type === 'resource') {
    const proto = props.data.node.protocol
    if (proto === 'ssh') return '$'
    if (proto === 'sftp') return '📁'
    if (proto === 'sql') return 'dB'
    if (proto === 'redis') return 'R'
    if (proto === 's3') return '☁'
    if (proto === 'sip') return '☎'
    return '?'
  }
  return '?'
}

function statusClass(): string {
  const s = props.data.node.status
  if (s === 'online' || s === 'connected') return 'topo-node-dot--success'
  return 'topo-node-dot--offline'
}
</script>

<template>
  <div class="topo-node" :style="{ '--node-color': nodeColor() }">
    <div class="topo-node-icon" :style="{ background: nodeColor() }">
      {{ nodeIcon() }}
    </div>
    <div class="topo-node-body">
      <div class="topo-node-label">{{ data.node.label }}</div>
      <div v-if="data.node.metadata?.hostname" class="topo-node-meta">
        {{ data.node.metadata.hostname }}
      </div>
      <div v-else-if="data.node.protocol" class="topo-node-meta topo-node-meta--proto">
        {{ data.node.protocol }}
      </div>
    </div>
    <div class="topo-node-dot" :class="statusClass()" />
  </div>
</template>

<style scoped>
.topo-node {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  min-width: 160px;
  max-width: 240px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
  position: relative;
}

.topo-node:hover {
  border-color: var(--border-strong);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

.topo-node-icon {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  display: grid;
  place-items: center;
  color: var(--on-ink);
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}

.topo-node-body {
  flex: 1;
  min-width: 0;
}

.topo-node-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.topo-node-meta {
  font-size: 10px;
  font-family: var(--font-mono);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.topo-node-meta--proto {
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.topo-node-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.topo-node-dot--success {
  background: var(--success);
  box-shadow: 0 0 6px var(--success);
}

.topo-node-dot--offline {
  background: var(--text-muted);
  opacity: 0.5;
}
</style>
