<script setup lang="ts">
import { ref } from 'vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import { useKeyboardShortcuts } from '@/composables/useKeyboardShortcuts'
import Toast from '@/components/ui/Toast.vue'

interface Tab {
  id: string
  label: string
  protocol: 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3'
  host?: string
  status?: StatusDotStatus
}

const tabs = ref<Tab[]>([
  { id: 'ssh-1', label: 'Web Server', protocol: 'ssh', host: '10.0.1.5', status: 'online' },
  { id: 'mysql-1', label: 'DB Primary', protocol: 'mysql', host: 'db.internal', status: 'online' },
])
const activeTab = ref('ssh-1')

const protoColor = (proto: Tab['protocol']) => `var(--proto-${proto})`
const now = ref(new Date().toLocaleTimeString('zh-CN', { hour12: false }))
setInterval(() => {
  now.value = new Date().toLocaleTimeString('zh-CN', { hour12: false })
}, 1000)

const toastRef = ref<InstanceType<typeof Toast> | null>(null)

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
])
</script>

<template>
  <div class="workspace">
    <Toast ref="toastRef" />

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

    <!-- Content area -->
    <div class="ws-content">
      <div class="ws-terminal">
        <div class="ws-term-line muted">
          <span class="mono" style="color: var(--success)">$</span>
          Connected to {{ tabs.find(t => t.id === activeTab)?.host || 'localhost' }} via {{ tabs.find(t => t.id === activeTab)?.protocol.toUpperCase() || 'SSH' }}
        </div>
        <div class="ws-term-line muted">
          <span class="mono" style="color: var(--accent)">▸</span>
          Terminal / SQL console will render here (M3+)
        </div>
      </div>
    </div>

    <!-- Status bar -->
    <div class="ws-statusbar mono">
      <span class="ws-status-item">
        <StatusDot :status="tabs.find(t => t.id === activeTab)?.status || 'online'" />
        {{ tabs.find(t => t.id === activeTab)?.protocol.toUpperCase() }} · {{ tabs.find(t => t.id === activeTab)?.host }}
      </span>
      <span class="ws-status-item">UTF-8</span>
      <span class="ws-status-item">LF</span>
      <span class="ws-status-spacer" />
      <span class="ws-status-item ws-quick-actions">
        <button class="ws-action-btn" title="New tab (Ctrl+T)">+</button>
        <button class="ws-action-btn" title="Split view">⊞</button>
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
  .ws-tab-host {
    display: none;
  }
  .ws-statusbar .ws-status-item:nth-child(n+2) {
    display: none;
  }
  .ws-statusbar .ws-status-item:last-child {
    display: flex;
  }
  .ws-quick-actions {
    display: flex !important;
  }
}
</style>
