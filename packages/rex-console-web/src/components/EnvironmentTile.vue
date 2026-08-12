<script setup lang="ts">
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import Badge from '@/components/ui/Badge.vue'

defineProps<{
  name: string
  description?: string
  agentStatus: StatusDotStatus | null
  resourceCount: number
  connectionMode: string
  showActions?: boolean
}>()

defineEmits<{
  click: []
  edit: []
  delete: []
}>()
</script>

<template>
  <div class="env-tile" role="button" tabindex="0" @click="$emit('click')" @keydown.enter="$emit('click')">
    <div class="env-tile-top">
      <div class="env-tile-icon" :class="`env-tile-icon--${connectionMode}`">
        {{ connectionMode === 'agent' ? '⬡' : '◉' }}
      </div>
      <div v-if="showActions" class="env-tile-actions" @click.stop>
        <button class="env-tile-action" @click="$emit('edit')">✎</button>
        <button class="env-tile-action env-tile-action--danger" @click="$emit('delete')">✕</button>
      </div>
    </div>
    <div class="env-tile-body">
      <span class="env-tile-name mono">{{ name }}</span>
      <span class="env-tile-desc muted">{{ description || '—' }}</span>
    </div>
    <div class="env-tile-agent">
      <template v-if="agentStatus">
        <StatusDot :status="agentStatus" />
        <span class="mono env-tile-agent-text">Agent {{ agentStatus }}</span>
      </template>
      <template v-else>
        <span class="muted env-tile-agent-text">No agent</span>
      </template>
    </div>
    <div class="env-tile-footer">
      <Badge tone="accent" size="sm">{{ resourceCount }} resources</Badge>
      <Badge :tone="connectionMode === 'agent' ? 'warning' : 'info'" size="sm">
        {{ connectionMode }}
      </Badge>
    </div>
  </div>
</template>

<style scoped>
.env-tile {
  display: flex; flex-direction: column; gap: var(--space-3);
  padding: var(--space-4);
  background: var(--bg-surface); border: 1px solid var(--border);
  border-radius: var(--radius-lg); cursor: pointer; text-align: left;
  transition: border-color var(--transition), transform var(--transition);
}
.env-tile:hover { border-color: var(--accent); transform: translateY(-1px); }
.env-tile-top { display: flex; align-items: flex-start; justify-content: space-between; }
.env-tile-icon {
  width: 36px; height: 36px; display: flex; align-items: center; justify-content: center;
  border-radius: var(--radius); font-size: 18px; flex-shrink: 0;
}
.env-tile-icon--direct { color: var(--info); background: var(--info-soft); }
.env-tile-icon--agent { color: var(--accent); background: var(--accent-soft); }
.env-tile-actions { display: flex; gap: 2px; opacity: 0; transition: opacity var(--transition); }
.env-tile:hover .env-tile-actions { opacity: 1; }
.env-tile-action {
  width: 26px; height: 26px; border: none; background: transparent;
  color: var(--text-muted); cursor: pointer; border-radius: var(--radius-sm);
  font-size: 13px; display: flex; align-items: center; justify-content: center;
  transition: background var(--transition), color var(--transition);
}
.env-tile-action:hover { background: var(--bg-hover); color: var(--text-primary); }
.env-tile-action--danger:hover { color: var(--danger); }
.env-tile-body { display: flex; flex-direction: column; gap: var(--space-1); }
.env-tile-name { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); }
.env-tile-desc { font-size: var(--text-xs); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.env-tile-agent {
  display: flex; align-items: center; gap: var(--space-2);
  padding-top: var(--space-2); border-top: 1px solid var(--border-subtle);
  font-size: var(--text-xs);
}
.env-tile-agent-text { color: var(--text-secondary); }
.env-tile-footer { display: flex; gap: var(--space-2); justify-content: flex-end; }
.muted { color: var(--text-muted); }
.mono { font-family: var(--font-mono); }
</style>
