<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount } from 'vue'
import { onClickOutside } from '@vueuse/core'
import Select from '@/components/ui/Select.vue'
import Input from '@/components/ui/Input.vue'
import Button from '@/components/ui/Button.vue'

const emit = defineEmits<{ connect: [config: { protocol: string; host: string; port: string; user: string; password: string }] }>()

const protocol = ref('ssh')
const host = ref('')
const port = ref('')
const user = ref('')
const password = ref('')
const showHistory = ref(false)
const historyRef = ref<HTMLElement | null>(null)

onClickOutside(historyRef, () => { showHistory.value = false })

const protocols = [
  { label: 'SSH', value: 'ssh' },
  { label: 'MySQL', value: 'mysql' },
  { label: 'Redis', value: 'redis' },
  { label: 'PostgreSQL', value: 'postgresql' },
  { label: 'SFTP', value: 'sftp' },
]

const defaultPorts: Record<string, string> = {
  ssh: '22', mysql: '3306', redis: '6379', postgresql: '5432', sftp: '22',
}

watch(protocol, (p) => {
  if (!port.value || Object.values(defaultPorts).includes(port.value)) {
    port.value = defaultPorts[p] || ''
  }
})

// Connection history
const HISTORY_KEY = 'rex-qc-history'
const MAX_HISTORY = 10

interface HistoryEntry {
  protocol: string
  host: string
  port: string
  user: string
  timestamp: number
}

const history = ref<HistoryEntry[]>([])

function loadHistory() {
  try {
    const raw = localStorage.getItem(HISTORY_KEY)
    if (raw) history.value = JSON.parse(raw)
  } catch { history.value = [] }
}

function saveHistory(entry: HistoryEntry) {
  const filtered = history.value.filter(h => !(h.host === entry.host && h.protocol === entry.protocol && h.user === entry.user))
  filtered.unshift(entry)
  history.value = filtered.slice(0, MAX_HISTORY)
  localStorage.setItem(HISTORY_KEY, JSON.stringify(history.value))
}

function selectHistory(entry: HistoryEntry) {
  protocol.value = entry.protocol
  host.value = entry.host
  port.value = entry.port
  user.value = entry.user
  showHistory.value = false
}

function onConnect() {
  if (!host.value) return
  saveHistory({
    protocol: protocol.value,
    host: host.value,
    port: port.value || defaultPorts[protocol.value] || '',
    user: user.value,
    timestamp: Date.now(),
  })
  emit('connect', {
    protocol: protocol.value,
    host: host.value,
    port: port.value || defaultPorts[protocol.value] || '',
    user: user.value,
    password: password.value,
  })
}

loadHistory()

function formatTime(ts: number): string {
  const d = new Date(ts)
  return `${d.getMonth() + 1}/${d.getDate()} ${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}`
}
</script>

<template>
  <div class="quick-connect">
    <Select v-model="protocol" :options="protocols" size="sm" />
    <Input v-model="host" placeholder="Host" size="sm" class="qc-host" />
    <Input v-model="port" placeholder="Port" size="sm" class="qc-port" />
    <Input v-model="user" placeholder="User" size="sm" class="qc-user" />
    <Input v-model="password" placeholder="Password" size="sm" class="qc-pass" type="password" />
    <div class="qc-connect-wrap">
      <Button variant="primary" size="sm" :disabled="!host" @click="onConnect">Connect</Button>
      <button v-if="history.length" class="qc-history-btn" @click="showHistory = !showHistory">▾</button>
      <div v-if="showHistory" ref="historyRef" class="qc-history">
        <div class="qc-history-header">Recent Connections</div>
        <div v-for="(h, i) in history" :key="i" class="qc-history-item" @click="selectHistory(h)">
          <span class="qc-history-proto mono">{{ h.protocol.toUpperCase() }}</span>
          <span class="qc-history-host">{{ h.host }}<template v-if="h.user"> ({{ h.user }})</template></span>
          <span class="qc-history-time muted">{{ formatTime(h.timestamp) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.quick-connect {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 36px;
}
.qc-host { width: 160px; }
.qc-port { width: 70px; }
.qc-user { width: 100px; }
.qc-pass { width: 110px; }
.qc-connect-wrap {
  position: relative;
  display: flex;
  align-items: center;
}
.qc-history-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 10px;
  margin-left: 2px;
  padding: 2px;
}
.qc-history-btn:hover {
  color: var(--text-primary);
}
.qc-history {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: 4px;
  width: 320px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  z-index: 100;
  max-height: 300px;
  overflow-y: auto;
}
.qc-history-header {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-muted);
  border-bottom: 1px solid var(--border);
  font-weight: 600;
}
.qc-history-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  cursor: pointer;
  color: var(--text-secondary);
}
.qc-history-item:hover {
  background: var(--bg-hover);
}
.qc-history-proto {
  font-size: var(--text-xs);
  color: var(--accent, #E8912D);
  width: 50px;
}
.qc-history-host {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.qc-history-time {
  font-size: var(--text-xs);
}
.muted {
  color: var(--text-muted);
}
.mono {
  font-family: var(--font-mono);
}
</style>
