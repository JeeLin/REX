<script setup lang="ts">
import { ref } from 'vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'

interface Connection {
  id: string
  name: string
  host: string
  protocol: 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3'
  status: StatusDotStatus
}

const connections: Connection[] = [
  { id: '1', name: 'Web Server', host: '10.0.1.5', protocol: 'ssh', status: 'online' },
  { id: '2', name: 'DB Primary', host: 'db.internal', protocol: 'mysql', status: 'online' },
  { id: '3', name: 'Cache', host: 'cache.local', protocol: 'redis', status: 'offline' },
  { id: '4', name: 'Analytics', host: 'analytics.db', protocol: 'postgresql', status: 'connecting' },
]

const activeTab = ref('ssh-1')
const tabs = [
  { id: 'ssh-1', label: 'SSH · 10.0.1.5', protocol: 'ssh' as const },
  { id: 'mysql-1', label: 'MySQL · db.internal', protocol: 'mysql' as const },
]

const protoColor = (proto: Connection['protocol']) => `var(--proto-${proto})`
const now = ref(new Date().toLocaleTimeString('zh-CN', { hour12: false }))
setInterval(() => {
  now.value = new Date().toLocaleTimeString('zh-CN', { hour12: false })
}, 1000)
</script>

<template>
  <div class="workspace">
    <!-- Tab bar -->
    <div class="ws-tabs">
      <div
        v-for="tab in tabs"
        :key="tab.id"
        class="ws-tab mono"
        :class="{ 'ws-tab--active': activeTab === tab.id }"
        @click="activeTab = tab.id"
      >
        <span class="ws-tab-dot" :style="{ background: protoColor(tab.protocol) }" />
        {{ tab.label }}
      </div>
      <button class="ws-tab-add" title="New connection">+</button>
    </div>

    <div class="ws-body">
      <!-- Connection tree -->
      <aside class="ws-tree">
        <div class="ws-tree-header mono">Connections</div>
        <div class="ws-tree-list">
          <div v-for="conn in connections" :key="conn.id" class="ws-tree-item">
            <StatusDot :status="conn.status" />
            <span class="ws-tree-proto mono" :style="{ color: protoColor(conn.protocol) }">
              {{ conn.protocol.toUpperCase() }}
            </span>
            <span class="ws-tree-name">{{ conn.name }}</span>
          </div>
        </div>
      </aside>

      <!-- Content area (terminal placeholder) -->
      <div class="ws-content">
        <div class="ws-terminal">
          <div class="ws-term-line muted">
            <span class="mono" style="color: var(--success)">$</span>
            Connected to 10.0.1.5 via SSH
          </div>
          <div class="ws-term-line muted">
            <span class="mono" style="color: var(--accent)">▸</span>
            Terminal / SQL console will render here (M2+)
          </div>
        </div>
      </div>
    </div>

    <!-- Status bar -->
    <div class="ws-statusbar mono">
      <span class="ws-status-item">
        <StatusDot status="online" />
        SSH · 10.0.1.5
      </span>
      <span class="ws-status-item">UTF-8</span>
      <span class="ws-status-item">LF</span>
      <span class="ws-status-spacer" />
      <span class="ws-status-item">{{ now }}</span>
    </div>
  </div>
</template>

<style scoped>
.workspace {
  height: 100%;
  display: flex;
  flex-direction: column;
  margin: calc(-1 * var(--space-5));
  background: var(--bg-deep);
}

/* Tab bar */
.ws-tabs {
  height: var(--tabbar-height);
  display: flex;
  align-items: stretch;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  overflow-x: auto;
  scrollbar-width: none;
}
.ws-tabs::-webkit-scrollbar { display: none; }
.ws-tab {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: 0 var(--space-4);
  font-size: var(--text-sm);
  color: var(--text-muted);
  border-right: 1px solid var(--border);
  cursor: pointer;
  white-space: nowrap;
  transition: color var(--transition), background var(--transition);
}
.ws-tab:hover {
  color: var(--text-secondary);
  background: var(--bg-hover);
}
.ws-tab--active {
  color: var(--text-primary);
  background: var(--bg-deep);
  border-bottom: 2px solid var(--accent);
}
.ws-tab-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}
.ws-tab-add {
  padding: 0 var(--space-3);
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-lg);
  cursor: pointer;
  transition: color var(--transition);
}
.ws-tab-add:hover {
  color: var(--accent);
}

/* Body */
.ws-body {
  flex: 1;
  display: flex;
  min-height: 0;
}

/* Connection tree */
.ws-tree {
  width: 200px;
  flex-shrink: 0;
  background: var(--bg-page);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
}
.ws-tree-header {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--border);
}
.ws-tree-list {
  padding: var(--space-2) 0;
  overflow-y: auto;
}
.ws-tree-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  cursor: pointer;
  transition: background var(--transition);
}
.ws-tree-item:hover {
  background: var(--bg-hover);
}
.ws-tree-proto {
  font-size: var(--text-xs);
  font-weight: 600;
}
.ws-tree-name {
  color: var(--text-primary);
}

/* Content area */
.ws-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: var(--bg-deep);
}
.ws-terminal {
  flex: 1;
  padding: var(--space-4);
  font-family: var(--font-mono);
  font-size: var(--text-base);
  line-height: 1.6;
}
.ws-term-line {
  margin-bottom: var(--space-1);
}

/* Status bar */
.ws-statusbar {
  height: var(--statusbar-height);
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: 0 var(--space-3);
  background: var(--bg-elevated);
  border-top: 1px solid var(--border);
  font-size: var(--text-xs);
  color: var(--text-muted);
}
.ws-status-item {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}
.ws-status-spacer {
  flex: 1;
}
</style>
