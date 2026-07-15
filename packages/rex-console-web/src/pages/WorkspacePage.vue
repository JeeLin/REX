<script setup lang="ts">
import { ref, computed, onBeforeUnmount } from 'vue'
import { Splitpanes, Pane } from 'splitpanes'
import 'splitpanes/dist/splitpanes.css'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import { useKeyboardShortcuts } from '@/composables/useKeyboardShortcuts'
import ConnectionTree from '@/features/workspace/ConnectionTree.vue'

interface Tab {
  id: string
  label: string
  protocol: 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3'
  host?: string
  status?: StatusDotStatus
}

interface SplitPane {
  id: string
  direction: 'row' | 'column'
  children: (SplitPane | string)[]
}

const tabs = ref<Tab[]>([
  { id: 'ssh-1', label: 'Web Server', protocol: 'ssh', host: '10.0.1.5', status: 'online' },
  { id: 'mysql-1', label: 'DB Primary', protocol: 'mysql', host: 'db.internal', status: 'online' },
])
const activeTab = ref('ssh-1')
const splitDirection = ref<'row' | 'column'>('row')
const panes = ref<string[]>(['ssh-1'])
const splitCount = ref(1)

const protoColor = (proto: Tab['protocol']) => `var(--proto-${proto})`
const now = ref(new Date().toLocaleTimeString('zh-CN', { hour12: false }))
const timer = setInterval(() => {
  now.value = new Date().toLocaleTimeString('zh-CN', { hour12: false })
}, 1000)
onBeforeUnmount(() => clearInterval(timer))

const activeTabInfo = computed(() => tabs.value.find(t => t.id === activeTab.value))

// 连接树
const treeCollapsed = ref(false)

function openResourceFromTree(node: { id: string; name: string; protocol?: string; host?: string; status?: StatusDotStatus }) {
  const id = `tab-${node.id}`
  if (!tabs.value.find(t => t.id === id)) {
    tabs.value.push({
      id,
      label: node.name,
      protocol: (node.protocol || 'ssh') as Tab['protocol'],
      host: node.host,
      status: node.status,
    })
  }
  activeTab.value = id
}

// 分栏操作
function splitHorizontal() {
  splitCount.value++
  splitDirection.value = 'row'
}
function splitVertical() {
  splitCount.value++
  splitDirection.value = 'column'
}
function closePane(idx: number) {
  if (splitCount.value > 1) {
    splitCount.value--
  }
}

// 快捷键
useKeyboardShortcuts([
  { key: 't', ctrl: true, handler: () => {
    const id = `tab-${Date.now()}`
    tabs.value.push({ id, label: 'New Tab', protocol: 'ssh' })
    activeTab.value = id
  } },
  { key: 'w', ctrl: true, handler: () => {
    const idx = tabs.value.findIndex(t => t.id === activeTab.value)
    if (idx >= 0 && tabs.value.length > 1) {
      tabs.value.splice(idx, 1)
      activeTab.value = tabs.value[Math.max(0, idx - 1)]!.id
    }
  } },
  { key: 'Tab', ctrl: true, handler: () => {
    const idx = tabs.value.findIndex(t => t.id === activeTab.value)
    activeTab.value = tabs.value[(idx + 1) % tabs.value.length]!.id
  } },
  { key: '\\', ctrl: true, handler: splitHorizontal },
  { key: '\\', ctrl: true, shift: true, handler: splitVertical },
])
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
        <span>{{ tab.label }}</span>
        <span v-if="tab.host" class="ws-tab-host muted">{{ tab.host }}</span>
        <button class="ws-tab-close" @click.stop="() => { const i = tabs.findIndex(t => t.id === tab.id); if (i >= 0 && tabs.length > 1) { tabs.splice(i, 1); activeTab = tabs[Math.max(0, i - 1)]!.id } }">×</button>
      </div>
      <button class="ws-tab-add" title="New connection (Ctrl+T)">+</button>
    </div>

    <!-- Connection tree sidebar -->
    <div v-show="!treeCollapsed" class="ws-tree" :style="{ width: '220px' }">
      <ConnectionTree @open-resource="openResourceFromTree" />
    </div>
    <button class="ws-tree-toggle" @click="treeCollapsed = !treeCollapsed" :title="treeCollapsed ? 'Show tree' : 'Hide tree'">
      {{ treeCollapsed ? '»' : '«' }}
    </button>

    <!-- Split panes -->
    <div class="ws-body">
      <Splitpanes
        :horizontal="splitDirection === 'column'"
        class="ws-split"
        @resized="() => {}"
      >
        <Pane v-for="i in splitCount" :key="i" :size="100 / splitCount" :min-size="20">
          <div class="ws-pane">
            <div class="ws-pane-header mono">
              <span>{{ activeTabInfo?.label || 'Tab' }}</span>
              <div class="ws-pane-actions">
                <button class="ws-pane-btn" @click="splitHorizontal" title="Split horizontal (Ctrl+\)">⊞</button>
                <button class="ws-pane-btn" @click="splitVertical" title="Split vertical (Ctrl+Shift+\)">⊟</button>
                <button v-if="splitCount > 1" class="ws-pane-btn" @click="closePane(i - 1)" title="Close pane">×</button>
              </div>
            </div>
            <div class="ws-terminal">
              <div class="ws-term-line muted">
                <span class="mono" style="color: var(--success)">$</span>
                Connected to {{ activeTabInfo?.host || 'localhost' }} via {{ activeTabInfo?.protocol.toUpperCase() || 'SSH' }}
              </div>
              <div class="ws-term-line muted">
                <span class="mono" style="color: var(--accent)">▸</span>
                Terminal / SQL console will render here (M3+)
              </div>
            </div>
          </div>
        </Pane>
      </Splitpanes>
    </div>

    <!-- Status bar -->
    <div class="ws-statusbar mono">
      <span class="ws-status-item">
        <StatusDot :status="activeTabInfo?.status || 'online'" />
        {{ activeTabInfo?.protocol.toUpperCase() }} · {{ activeTabInfo?.host }}
      </span>
      <span class="ws-status-item">UTF-8</span>
      <span class="ws-status-item">LF</span>
      <span class="ws-status-spacer" />
      <span class="ws-status-item ws-quick-actions">
        <button class="ws-action-btn" @click="splitHorizontal" title="Split horizontal">⊞</button>
        <button class="ws-action-btn" @click="splitVertical" title="Split vertical">⊟</button>
        <button class="ws-action-btn" title="Find">🔍</button>
      </span>
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
  padding: 0 var(--space-3);
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
.ws-tab-host {
  font-size: var(--text-xs);
}
.ws-tab-close {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-md);
  cursor: pointer;
  padding: 0 2px;
  line-height: 1;
  border-radius: var(--radius-sm);
  transition: color var(--transition), background var(--transition);
}
.ws-tab-close:hover {
  color: var(--danger);
  background: rgba(248, 81, 73, 0.15);
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

/* Connection tree sidebar */
.ws-tree {
  flex-shrink: 0;
  background: var(--bg-surface);
  border-right: 1px solid var(--border);
  overflow: hidden;
}
.ws-tree-toggle {
  width: 16px;
  flex-shrink: 0;
  background: var(--bg-surface);
  border: none;
  border-right: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 10px;
  cursor: pointer;
  transition: color var(--transition);
}
.ws-tree-toggle:hover {
  color: var(--accent);
}

/* Split panes */
.ws-body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.ws-split {
  height: 100%;
}
:deep(.splitpanes__splitter) {
  background-color: var(--border);
  min-width: 3px;
  min-height: 3px;
}
:deep(.splitpanes__splitter:hover) {
  background-color: var(--accent);
}

/* Pane */
.ws-pane {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-deep);
}
.ws-pane-header {
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--space-3);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  font-size: var(--text-xs);
  color: var(--text-muted);
}
.ws-pane-actions {
  display: flex;
  gap: var(--space-1);
}
.ws-pane-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-xs);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: var(--radius-sm);
  transition: color var(--transition);
}
.ws-pane-btn:hover {
  color: var(--accent);
}

/* Terminal */
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
.ws-quick-actions {
  display: flex;
  gap: var(--space-1);
}
.ws-action-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-xs);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: var(--radius-sm);
  transition: color var(--transition), background var(--transition);
}
.ws-action-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

/* 手机端适配 */
@media (max-width: 768px) {
  .ws-tab-host { display: none; }
  .ws-statusbar .ws-status-item:nth-child(n+2) { display: none; }
  .ws-statusbar .ws-status-item:last-child { display: flex; }
  .ws-quick-actions { display: flex !important; }
}
</style>
