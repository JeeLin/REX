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
  group: string
}

const groups = ['Production', 'Staging']

const connections: Connection[] = [
  { id: '1', name: 'Web Server', host: '10.0.1.5', protocol: 'ssh', status: 'online', group: 'Production' },
  { id: '2', name: 'DB Primary', host: 'db.internal', protocol: 'mysql', status: 'online', group: 'Production' },
  { id: '3', name: 'Cache', host: 'cache.local', protocol: 'redis', status: 'offline', group: 'Production' },
  { id: '4', name: 'Analytics', host: 'analytics.db', protocol: 'postgresql', status: 'connecting', group: 'Staging' },
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

// 右键菜单
const contextMenu = ref({ show: false, x: 0, y: 0, conn: null as Connection | null })
function onContextMenu(e: MouseEvent, conn: Connection) {
  e.preventDefault()
  contextMenu.value = { show: true, x: e.clientX, y: e.clientY, conn }
}
function closeContextMenu() {
  contextMenu.value.show = false
}

// 连接树折叠
const collapsedGroups = ref(new Set<string>())
function toggleGroup(group: string) {
  if (collapsedGroups.value.has(group)) {
    collapsedGroups.value.delete(group)
  } else {
    collapsedGroups.value.add(group)
  }
}
</script>

<template>
  <div class="workspace" @click="closeContextMenu">
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
          <template v-for="group in groups" :key="group">
            <div class="ws-tree-group" @click="toggleGroup(group)">
              <span class="ws-tree-chevron" :class="{ 'ws-collapsed': collapsedGroups.has(group) }">▸</span>
              <span class="ws-tree-group-name mono">{{ group }}</span>
            </div>
            <div v-if="!collapsedGroups.has(group)">
              <div
                v-for="conn in connections.filter(c => c.group === group)"
                :key="conn.id"
                class="ws-tree-item"
                @contextmenu="onContextMenu($event, conn)"
              >
                <StatusDot :status="conn.status" />
                <span class="ws-tree-proto mono" :style="{ color: protoColor(conn.protocol) }">
                  {{ conn.protocol.toUpperCase() }}
                </span>
                <span class="ws-tree-name">{{ conn.name }}</span>
                <span class="ws-tree-host mono muted">{{ conn.host }}</span>
              </div>
            </div>
          </template>
        </div>
      </aside>

      <!-- Content area -->
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

    <!-- 右键菜单 -->
    <Teleport to="body">
      <div
        v-if="contextMenu.show"
        class="ws-context-menu"
        :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
        @click.stop
      >
        <div class="ws-ctx-item">Open in new tab</div>
        <div class="ws-ctx-item">Edit connection</div>
        <div class="ws-ctx-divider" />
        <div class="ws-ctx-item ws-ctx-item--danger">Disconnect</div>
      </div>
    </Teleport>
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
  width: 220px;
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
.ws-tree-group {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-muted);
  cursor: pointer;
  user-select: none;
}
.ws-tree-group:hover {
  color: var(--text-secondary);
}
.ws-tree-chevron {
  font-size: 10px;
  transition: transform var(--transition);
}
.ws-collapsed {
  transform: rotate(0deg);
}
.ws-tree-group-name {
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}
.ws-tree-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  padding-left: var(--space-6);
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
.ws-tree-host {
  font-size: var(--text-xs);
  margin-left: auto;
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

/* 右键菜单 */
.ws-context-menu {
  position: fixed;
  z-index: 9999;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  padding: var(--space-1) 0;
  min-width: 160px;
  box-shadow: var(--shadow-lg);
}
.ws-ctx-item {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  cursor: pointer;
}
.ws-ctx-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.ws-ctx-item--danger {
  color: var(--danger);
}
.ws-ctx-divider {
  height: 1px;
  background: var(--border);
  margin: var(--space-1) 0;
}
</style>
