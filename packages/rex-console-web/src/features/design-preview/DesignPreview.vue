<script setup lang="ts">
import { ref } from 'vue'
import { RouterLink } from 'vue-router'
import Button from '@/components/ui/Button.vue'
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import Tabs from '@/components/ui/Tabs.vue'
import Table from '@/components/ui/Table.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import Drawer from '@/components/ui/Drawer.vue'
import Modal from '@/components/ui/Modal.vue'
import Tooltip from '@/components/ui/Tooltip.vue'
import Toast from '@/components/ui/Toast.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import Input from '@/components/ui/Input.vue'
import Select from '@/components/ui/Select.vue'
import Checkbox from '@/components/ui/Checkbox.vue'
import Switch from '@/components/ui/Switch.vue'
import Avatar from '@/components/ui/Avatar.vue'
import Alert from '@/components/ui/Alert.vue'
import ToggleGroup from '@/components/ui/ToggleGroup.vue'
import Scrollbar from '@/components/ui/Scrollbar.vue'

const theme = ref<'dark' | 'light'>('dark')
function toggleTheme() {
  theme.value = theme.value === 'dark' ? 'light' : 'dark'
  if (theme.value === 'light') {
    document.documentElement.dataset.theme = 'light'
  } else {
    delete document.documentElement.dataset.theme
  }
}

// Tabs demo
const tabs = ref(['Terminal', 'SQL', 'Redis'])
const activeTab = ref('Terminal')

// Table demo
const columns = [
  { key: 'protocol', label: 'Protocol', width: '100px' },
  { key: 'host', label: 'Host' },
  { key: 'status', label: 'Status', width: '90px', align: 'right' as const },
]
const rows = [
  { protocol: 'SSH', host: '10.0.1.5', status: 'online' as StatusDotStatus },
  { protocol: 'MySQL', host: 'db.internal', status: 'online' as StatusDotStatus },
  { protocol: 'Redis', host: 'cache.local', status: 'offline' as StatusDotStatus },
  { protocol: 'SFTP', host: 'nas.home', status: 'connecting' as StatusDotStatus },
]

// Drawer / Modal demos
const showDrawer = ref(false)
const showModal = ref(false)

// Toast demo
const toastRef = ref<InstanceType<typeof Toast> | null>(null)
function fireToast(tone: 'success' | 'error' | 'info') {
  const msgs = { success: 'Saved successfully', error: 'Connection failed', info: 'Operation queued' }
  toastRef.value?.push(msgs[tone], tone)
}

// Form demos
const inputVal = ref('')
const selectVal = ref('')
const checkboxVal = ref(false)
const switchVal = ref(false)
const toggleVal = ref('dark')
const selectOptions = [
  { label: 'SSH', value: 'ssh' },
  { label: 'MySQL', value: 'mysql' },
  { label: 'Redis', value: 'redis' },
  { label: 'PostgreSQL', value: 'postgresql', disabled: true },
]
</script>

<template>
  <div class="dp">
    <Toast ref="toastRef" />
    <header class="dp-header">
      <h1 class="dp-title mono">REX Design System Preview</h1>
      <button class="btn-toggle mono" @click="toggleTheme">
        {{ theme === 'dark' ? '☀ light' : '● dark' }}
      </button>
    </header>

    <!-- Tokens -->
    <section class="dp-section">
      <h2 class="dp-h2">Color Tokens</h2>
      <div class="swatch-grid">
        <div
          v-for="c in [
            { name: '--accent', var: 'var(--accent)' },
            { name: '--success', var: 'var(--success)' },
            { name: '--danger', var: 'var(--danger)' },
            { name: '--info', var: 'var(--info)' },
            { name: '--purple', var: 'var(--purple)' },
            { name: '--warning', var: 'var(--warning)' },
            { name: '--bg-deep', var: 'var(--bg-deep)' },
            { name: '--bg-page', var: 'var(--bg-page)' },
            { name: '--bg-surface', var: 'var(--bg-surface)' },
            { name: '--bg-elevated', var: 'var(--bg-elevated)' },
          ]" :key="c.name" class="swatch"
        >
          <div class="swatch-color" :style="{ background: c.var }" />
          <span class="swatch-label mono">{{ c.name }}</span>
        </div>
      </div>
    </section>

    <!-- Spacing -->
    <section class="dp-section">
      <h2 class="dp-h2">Spacing Scale</h2>
      <div class="spacing-grid">
        <div v-for="n in [1, 2, 3, 4, 5, 6, 8]" :key="n" class="spacing-item">
          <div class="spacing-bar" :style="{ width: `var(--space-${n})` }" />
          <span class="mono">--space-{{ n }}</span>
        </div>
      </div>
    </section>

    <!-- Typography -->
    <section class="dp-section">
      <h2 class="dp-h2">Typography</h2>
      <div class="type-grid">
        <div class="type-row"><span class="mono">--text-2xl 28px</span><span style="font-size:var(--text-2xl)">Aa 极客</span></div>
        <div class="type-row"><span class="mono">--text-xl 20px</span><span style="font-size:var(--text-xl)">Aa 极客</span></div>
        <div class="type-row"><span class="mono">--text-lg 16px</span><span style="font-size:var(--text-lg)">Aa 极客</span></div>
        <div class="type-row"><span class="mono">--text-base 13px</span><span style="font-size:var(--text-base)">Aa 极客 — Inter</span></div>
        <div class="type-row"><span class="mono">--text-base (mono)</span><span style="font-size:var(--text-base);font-family:var(--font-mono)">Aa 极客 — JetBrains</span></div>
      </div>
    </section>

    <!-- Button -->
    <section class="dp-section">
      <h2 class="dp-h2">Buttons</h2>
      <div class="comp-row">
        <Button variant="primary">Primary</Button>
        <Button variant="secondary">Secondary</Button>
        <Button variant="danger">Danger</Button>
        <Button variant="ghost">Ghost</Button>
      </div>
      <div class="comp-row mt">
        <Button variant="primary" size="sm">Sm</Button>
        <Button variant="primary" size="md">Md</Button>
        <Button variant="primary" size="lg">Lg</Button>
        <Button variant="primary" disabled>Disabled</Button>
      </div>
    </section>

    <!-- Badge + StatusDot -->
    <section class="dp-section">
      <h2 class="dp-h2">Badges & Status Dots</h2>
      <div class="comp-row">
        <Badge tone="accent">Accent</Badge>
        <Badge tone="success"><StatusDot status="online" /> Online</Badge>
        <Badge tone="danger"><StatusDot status="error" /> Error</Badge>
        <Badge tone="info"><StatusDot status="connecting" /> Connecting</Badge>
        <Badge tone="purple">PostgreSQL</Badge>
        <Badge tone="warning">SQLite</Badge>
        <Badge tone="neutral">Neutral</Badge>
      </div>
      <div class="comp-row mt">
        <span class="label mono">Status dots:</span>
        <StatusDot status="online" />
        <StatusDot status="connecting" />
        <StatusDot status="offline" />
        <StatusDot status="error" />
      </div>
    </section>

    <!-- Tabs -->
    <section class="dp-section">
      <h2 class="dp-h2">Tabs</h2>
      <Card title="Workspace Tabs" style="height: 160px">
        <Tabs v-model="activeTab" :tabs="tabs">
          <template #item="{ tab }">
            <span class="mono">{{ tab }}</span>
          </template>
          <div style="padding: 12px; color: var(--text-secondary)">
            Active tab: <strong class="mono">{{ activeTab }}</strong> — content area
          </div>
        </Tabs>
      </Card>
    </section>

    <!-- Table -->
    <section class="dp-section">
      <h2 class="dp-h2">Table</h2>
      <Card :padded="false" title="Connection List">
        <Table :columns="columns" :rows="rows" :row-key="(r: (typeof rows)[number]) => r.host">
          <template #cell-status="{ row }">
            <Badge :tone="row.status === 'online' ? 'success' : row.status === 'connecting' ? 'warning' : 'danger'">
              <StatusDot :status="row.status" /> {{ row.status }}
            </Badge>
          </template>
        </Table>
      </Card>
    </section>

    <!-- Drawer / Modal / Tooltip -->
    <section class="dp-section">
      <h2 class="dp-h2">Overlay Components</h2>
      <div class="comp-row">
        <Button variant="secondary" @click="showDrawer = true">Open Drawer</Button>
        <Button variant="secondary" @click="showModal = true">Open Modal</Button>
        <Tooltip text="This is a tooltip"><Button variant="ghost">Hover me</Button></Tooltip>
      </div>
      <div class="comp-row mt">
        <span class="label mono">Toast:</span>
        <Button variant="ghost" @click="fireToast('success')">✓ Success</Button>
        <Button variant="ghost" @click="fireToast('error')">✕ Error</Button>
        <Button variant="ghost" @click="fireToast('info')">ℹ Info</Button>
      </div>
    </section>

    <!-- Form Components -->
    <section class="dp-section">
      <h2 class="dp-h2">Form Components</h2>
      <div class="comp-grid">
        <div class="comp-cell">
          <span class="label mono">Input</span>
          <Input v-model="inputVal" placeholder="Search connections..." clearable />
        </div>
        <div class="comp-cell">
          <span class="label mono">Input (error)</span>
          <Input model-value="invalid" error="Connection refused" />
        </div>
        <div class="comp-cell">
          <span class="label mono">Select</span>
          <Select v-model="selectVal" :options="selectOptions" placeholder="Protocol" />
        </div>
        <div class="comp-cell">
          <span class="label mono">Checkbox</span>
          <Checkbox v-model="checkboxVal" label="Remember me" />
        </div>
        <div class="comp-cell">
          <span class="label mono">Switch</span>
          <div class="comp-row">
            <Switch v-model="switchVal" />
            <span class="label">{{ switchVal ? 'ON' : 'OFF' }}</span>
          </div>
        </div>
        <div class="comp-cell">
          <span class="label mono">ToggleGroup</span>
          <ToggleGroup
            v-model="toggleVal" :options="[
              { label: 'Dark', value: 'dark', icon: '●' },
              { label: 'Light', value: 'light', icon: '☀' },
            ]"
          />
        </div>
        <div class="comp-cell">
          <span class="label mono">Avatar</span>
          <div class="comp-row">
            <Avatar size="sm" fallback="S" />
            <Avatar size="md" fallback="M" />
            <Avatar size="lg" fallback="R" />
          </div>
        </div>
      </div>
    </section>

    <!-- Feedback Components -->
    <section class="dp-section">
      <h2 class="dp-h2">Feedback Components</h2>
      <div class="comp-grid">
        <div class="comp-cell">
          <span class="label mono">Alert</span>
          <Alert type="info" title="Info">This is an informational message.</Alert>
        </div>
        <div class="comp-cell">
          <span class="label mono">Alert (success)</span>
          <Alert type="success">Connection established successfully.</Alert>
        </div>
        <div class="comp-cell">
          <span class="label mono">Alert (warning)</span>
          <Alert type="warning" closable>Agent version mismatch detected.</Alert>
        </div>
        <div class="comp-cell">
          <span class="label mono">Alert (error)</span>
          <Alert type="error">Connection refused.</Alert>
        </div>
        <div class="comp-cell">
          <span class="label mono">Badge sizes</span>
          <div class="comp-row">
            <Badge tone="accent" size="sm">Small</Badge>
            <Badge tone="accent" size="md">Medium</Badge>
            <Badge tone="success" dot>Online</Badge>
          </div>
        </div>
        <div class="comp-cell">
          <span class="label mono">Button loading</span>
          <div class="comp-row">
            <Button variant="primary" loading>Loading</Button>
            <Button variant="secondary" block style="max-width:200px">Block</Button>
          </div>
        </div>
      </div>
    </section>

    <!-- Scrollbar -->
    <section class="dp-section">
      <h2 class="dp-h2">Scrollbar</h2>
      <Scrollbar height="120px">
        <div style="padding: 12px; color: var(--text-secondary); line-height: 2">
          <div v-for="i in 20" :key="i">Scroll item {{ i }} — custom 6px thin scrollbar with hover effect</div>
        </div>
      </Scrollbar>
    </section>

    <!-- Empty State -->
    <section class="dp-section">
      <h2 class="dp-h2">Empty State</h2>
      <Card>
        <EmptyState
          icon="⌘"
          title="No connections yet"
          description="Add a connection to get started. Connect to SSH, MySQL, Redis, or any supported protocol."
        >
          <Button variant="primary">Add Connection</Button>
        </EmptyState>
      </Card>
    </section>

    <!-- Page Navigation -->
    <section class="dp-section">
      <h2 class="dp-h2">Pages</h2>
      <div class="page-links">
        <RouterLink
          v-for="p in [
            { to: '/workspace', label: 'Workspace', desc: 'Terminal, SQL, Redis console' },
            { to: '/dashboard', label: 'Dashboard', desc: 'Stats and quick connect' },
            { to: '/environments', label: 'Environments', desc: 'Group connections' },
            { to: '/agents', label: 'Agents', desc: 'Manage agent nodes' },
            { to: '/audit-log', label: 'Audit Log', desc: 'Connection history' },
            { to: '/settings', label: 'Settings', desc: 'Appearance and terminal' },
          ]" :key="p.to" :to="p.to" class="page-link"
        >
          <span class="page-link-label">{{ p.label }}</span>
          <span class="page-link-desc muted">{{ p.desc }}</span>
        </RouterLink>
      </div>
    </section>

    <Drawer v-model="showDrawer" title="Sample Drawer">
      <p style="color: var(--text-secondary)">Drawer content goes here. Side: right, blurred overlay.</p>
    </Drawer>
    <Modal v-model="showModal" title="Sample Modal">
      <p style="color: var(--text-secondary)">Modal dialog with blurred overlay. ESC or click outside to close.</p>
      <template #footer>
        <Button variant="ghost" @click="showModal = false">Cancel</Button>
        <Button variant="primary" @click="showModal = false">Confirm</Button>
      </template>
    </Modal>
  </div>
</template>

<style scoped>
.dp {
  max-width: 900px;
  margin: 0 auto;
  padding-bottom: 64px;
}
.dp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-6);
}
.dp-title {
  font-size: var(--text-xl);
  color: var(--text-primary);
}
.btn-toggle {
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  color: var(--text-secondary);
  padding: 6px 14px;
  font-size: var(--text-sm);
  cursor: pointer;
  transition: color var(--transition);
}
.btn-toggle:hover {
  color: var(--text-primary);
}
.dp-section {
  margin-bottom: var(--space-6);
  padding-bottom: var(--space-6);
  border-bottom: 1px solid var(--border);
}
.dp-h2 {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: var(--space-4);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.swatch-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
  gap: var(--space-3);
}
.swatch {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.swatch-color {
  width: 100%;
  height: 48px;
  border-radius: var(--radius);
  border: 1px solid var(--border);
}
.swatch-label {
  font-size: var(--text-xs);
  color: var(--text-muted);
}
.spacing-grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.spacing-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-muted);
}
.spacing-bar {
  height: 16px;
  background: var(--accent);
  border-radius: 2px;
  min-width: 4px;
}
.type-grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.type-row {
  display: flex;
  align-items: baseline;
  gap: var(--space-4);
  color: var(--text-primary);
}
.type-row .mono {
  min-width: 200px;
  font-size: var(--text-xs);
  color: var(--text-muted);
}
.comp-row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.mt {
  margin-top: var(--space-3);
}
.comp-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: var(--space-4);
}
.comp-cell {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.label {
  font-size: var(--text-xs);
  color: var(--text-muted);
}
.page-links {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: var(--space-3);
}
.page-link {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: var(--space-3);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  text-decoration: none;
  transition: border-color var(--transition), background var(--transition);
}
.page-link:hover {
  border-color: var(--accent);
  background: var(--bg-elevated);
}
.page-link-label {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
}
.page-link-desc {
  font-size: var(--text-xs);
}
</style>
