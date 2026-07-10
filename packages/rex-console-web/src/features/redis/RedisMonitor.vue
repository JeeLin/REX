<template>
  <div class="redis-monitor">
    <!-- 顶部工具栏 -->
    <div class="monitor-toolbar">
      <span class="monitor-title">{{ t('redis.monitor.title') }}</span>
      <div class="monitor-toolbar-right">
        <span v-if="lastUpdate" class="monitor-last-update">
          {{ t('redis.monitor.lastUpdate') }}: {{ formatTime(lastUpdate) }}
        </span>
        <label class="monitor-interval-label">
          {{ t('redis.monitor.refreshInterval') }}
          <select v-model.number="refreshInterval" class="monitor-interval-select">
            <option :value="3">3s</option>
            <option :value="5">5s</option>
            <option :value="10">10s</option>
            <option :value="30">30s</option>
            <option :value="60">60s</option>
          </select>
        </label>
        <button class="monitor-refresh-btn" @click="fetchInfo" :disabled="loading">
          {{ t('common.refresh') }}
        </button>
      </div>
    </div>

    <!-- 错误状态 -->
    <div v-if="error" class="monitor-error">
      <span class="monitor-error-icon">!</span>
      <span>{{ error }}</span>
      <button class="monitor-retry-btn" @click="fetchInfo">{{ t('common.retry') }}</button>
    </div>

    <!-- 未连接状态 -->
    <div v-else-if="!connected" class="monitor-empty">
      <span class="monitor-empty-icon">r</span>
      <p>{{ t('redis.monitor.notConnected') }}</p>
    </div>

    <!-- 加载状态 -->
    <div v-else-if="loading && !infoData" class="monitor-loading">
      <div class="monitor-spinner" />
      <span>{{ t('common.loading') }}</span>
    </div>

    <!-- 监控面板 -->
    <div v-else-if="infoData" class="monitor-grid">
      <!-- 内存使用卡片 -->
      <div class="monitor-card">
        <div class="monitor-card-header">
          <span class="monitor-card-icon" style="color: #ef4444">{{ t('redis.monitor.memory.icon') }}</span>
          <span class="monitor-card-title">{{ t('redis.monitor.memory.title') }}</span>
        </div>
        <div class="monitor-card-body">
          <div class="monitor-stat-main">
            <span class="monitor-stat-value">{{ formatBytes(infoData.usedMemory) }}</span>
            <span class="monitor-stat-sub">
              / {{ formatBytes(infoData.maxMemory) || t('redis.monitor.memory.unlimited') }}
            </span>
          </div>
          <div class="monitor-progress-wrapper" v-if="infoData.maxMemory > 0">
            <div class="monitor-progress-track">
              <div
                class="monitor-progress-fill"
                :style="{ width: memoryPercent + '%' }"
                :class="memoryPercentClass"
              />
            </div>
            <span class="monitor-progress-label" :class="memoryPercentClass">
              {{ memoryPercent.toFixed(1) }}%
            </span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.memory.usedPeak') }}</span>
            <span class="monitor-detail-value">{{ formatBytes(infoData.usedMemoryPeak) }}</span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.memory.rss') }}</span>
            <span class="monitor-detail-value">{{ formatBytes(infoData.usedMemoryRss) }}</span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.memory.fragmentation') }}</span>
            <span class="monitor-detail-value">{{ infoData.memFragRatio }}%</span>
          </div>
        </div>
      </div>

      <!-- 键统计卡片 -->
      <div class="monitor-card">
        <div class="monitor-card-header">
          <span class="monitor-card-icon" style="color: #f59e0b">{{ t('redis.monitor.keys.icon') }}</span>
          <span class="monitor-card-title">{{ t('redis.monitor.keys.title') }}</span>
        </div>
        <div class="monitor-card-body">
          <div class="monitor-stat-main">
            <span class="monitor-stat-value">{{ formatNumber(infoData.totalKeys) }}</span>
            <span class="monitor-stat-sub">{{ t('redis.monitor.keys.total') }}</span>
          </div>
          <div class="monitor-key-types" v-if="infoData.keyTypes.length > 0">
            <div
              v-for="kt in infoData.keyTypes"
              :key="kt.type"
              class="monitor-key-type-row"
            >
              <span class="monitor-key-type-name">{{ kt.type }}</span>
              <div class="monitor-key-type-bar-track">
                <div
                  class="monitor-key-type-bar-fill"
                  :style="{ width: (kt.count / infoData.totalKeys * 100) + '%' }"
                />
              </div>
              <span class="monitor-key-type-count">{{ formatNumber(kt.count) }}</span>
            </div>
          </div>
          <div v-else class="monitor-no-keys">
            {{ t('redis.monitor.keys.noKeyspace') }}
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.keys.expiryCount') }}</span>
            <span class="monitor-detail-value">{{ formatNumber(infoData.expires) }}</span>
          </div>
        </div>
      </div>

      <!-- 连接信息卡片 -->
      <div class="monitor-card">
        <div class="monitor-card-header">
          <span class="monitor-card-icon" style="color: #22c55e">{{ t('redis.monitor.connections.icon') }}</span>
          <span class="monitor-card-title">{{ t('redis.monitor.connections.title') }}</span>
        </div>
        <div class="monitor-card-body">
          <div class="monitor-stat-main">
            <span class="monitor-stat-value">{{ formatNumber(infoData.connectedClients) }}</span>
            <span class="monitor-stat-sub">{{ t('redis.monitor.connections.active') }}</span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.connections.blocked') }}</span>
            <span class="monitor-detail-value">{{ formatNumber(infoData.blockedClients) }}</span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.connections.tracking') }}</span>
            <span class="monitor-detail-value">{{ formatNumber(infoData.trackingClients) }}</span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.connections.maxClients') }}</span>
            <span class="monitor-detail-value">{{ formatNumber(infoData.maxClients) }}</span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.connections.rejected') }}</span>
            <span class="monitor-detail-value" :class="{ 'monitor-value-warn': infoData.rejectedConnections > 0 }">
              {{ formatNumber(infoData.rejectedConnections) }}
            </span>
          </div>
        </div>
      </div>

      <!-- 命令统计卡片 -->
      <div class="monitor-card">
        <div class="monitor-card-header">
          <span class="monitor-card-icon" style="color: #3b82f6">{{ t('redis.monitor.commands.icon') }}</span>
          <span class="monitor-card-title">{{ t('redis.monitor.commands.title') }}</span>
        </div>
        <div class="monitor-card-body">
          <div class="monitor-stat-main">
            <span class="monitor-stat-value">{{ formatNumber(infoData.opsPerSec) }}</span>
            <span class="monitor-stat-sub">{{ t('redis.monitor.commands.perSec') }}</span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.commands.totalProcessed') }}</span>
            <span class="monitor-detail-value">{{ formatNumber(infoData.totalCommandsProcessed) }}</span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.commands.instantaneousOps') }}</span>
            <span class="monitor-detail-value">{{ formatNumber(infoData.instantaneousOpsPerSec) }}</span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.commands.hitRate') }}</span>
            <span class="monitor-detail-value" :class="{ 'monitor-value-ok': hitRatePercent >= 90 }">
              {{ hitRatePercent.toFixed(1) }}%
            </span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.commands.keyspaceHits') }}</span>
            <span class="monitor-detail-value">{{ formatNumber(infoData.keyspaceHits) }}</span>
          </div>
          <div class="monitor-detail-row">
            <span class="monitor-detail-label">{{ t('redis.monitor.commands.keyspaceMisses') }}</span>
            <span class="monitor-detail-value">{{ formatNumber(infoData.keyspaceMisses) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRedisSession } from './useRedisSession'

const props = defineProps<{
  resourceId: string
}>()

const { t } = useI18n()
const session = useRedisSession(() => props.resourceId)

const connected = computed(() => session.connected.value)

// ── 状态 ──────────────────────────────────────────────
const loading = ref(false)
const error = ref<string | null>(null)
const lastUpdate = ref<number | null>(null)
const refreshInterval = ref(5)

interface KeyType {
  type: string
  count: number
}

interface InfoData {
  // Memory
  usedMemory: number
  usedMemoryPeak: number
  usedMemoryRss: number
  maxMemory: number
  memFragRatio: number
  // Keys
  totalKeys: number
  expires: number
  keyTypes: KeyType[]
  // Connections
  connectedClients: number
  blockedClients: number
  trackingClients: number
  maxClients: number
  rejectedConnections: number
  // Commands
  opsPerSec: number
  totalCommandsProcessed: number
  instantaneousOpsPerSec: number
  keyspaceHits: number
  keyspaceMisses: number
}

const infoData = ref<InfoData | null>(null)
let refreshTimer: ReturnType<typeof setInterval> | null = null

// ── 计算属性 ──────────────────────────────────────────
const memoryPercent = computed(() => {
  if (!infoData.value || infoData.value.maxMemory <= 0) return 0
  return Math.min((infoData.value.usedMemory / infoData.value.maxMemory) * 100, 100)
})

const memoryPercentClass = computed(() => {
  const pct = memoryPercent.value
  if (pct >= 90) return 'monitor-percent-danger'
  if (pct >= 70) return 'monitor-percent-warning'
  return 'monitor-percent-ok'
})

const hitRatePercent = computed(() => {
  if (!infoData.value) return 0
  const hits = infoData.value.keyspaceHits
  const misses = infoData.value.keyspaceMisses
  const total = hits + misses
  if (total === 0) return 100
  return (hits / total) * 100
})

// ── INFO 响应解析 ─────────────────────────────────────
function parseInfoResponse(raw: string): InfoData {
  const lines = raw.split(/\r?\n/)
  const sections: Record<string, Record<string, string>> = {}
  let currentSection = ''

  for (const line of lines) {
    const trimmed = line.trim()
    if (trimmed.startsWith('#')) {
      currentSection = trimmed.slice(1).trim().toLowerCase()
      sections[currentSection] = {}
      continue
    }
    if (trimmed === '') continue
    const colonIdx = trimmed.indexOf(':')
    if (colonIdx === -1) continue
    const key = trimmed.slice(0, colonIdx).trim()
    const value = trimmed.slice(colonIdx + 1).trim()
    if (!sections[currentSection]) sections[currentSection] = {}
    const section = sections[currentSection]
    if (section) section[key] = value
  }

  const mem = sections['memory'] || {}
  const cli = sections['clients'] || {}
  const stat = sections['stats'] || {}
  const ks = sections['keyspace'] || {}

  // 解析键空间类型分布
  const keyTypes: KeyType[] = []
  let totalKeys = 0
  let expires = 0
  for (const [dbName, dbInfo] of Object.entries(ks)) {
    // 格式: keys=123,expires=0,avg_ttl=0
    const pairs = dbInfo.split(',')
    const kv: Record<string, string> = {}
    for (const pair of pairs) {
      const [k, v] = pair.split('=')
      if (k && v !== undefined) kv[k.trim()] = v.trim()
    }
    const keys = parseInt(kv['keys'] || '0', 10)
    const exp = parseInt(kv['expires'] || '0', 10)
    totalKeys += keys
    expires += exp

    // 从数据库名提取类型分布 — 无法从 INFO keyspace 获得类型级别数据
    // 使用 DB 编号作为 label
    if (keys > 0) {
      keyTypes.push({ type: dbName, count: keys })
    }
  }

  // 从 INFO keyspace 无法获得 string/hash/list 等类型分布
  // 但我们有每个 DB 的 key 数量，这比类型分布更有用
  return {
    usedMemory: parseInt(mem['used_memory'] || '0', 10),
    usedMemoryPeak: parseInt(mem['used_memory_peak'] || '0', 10),
    usedMemoryRss: parseInt(mem['used_memory_rss'] || '0', 10),
    maxMemory: parseInt(mem['maxmemory'] || '0', 10),
    memFragRatio: parseFloat(mem['mem_fragmentation_ratio'] || '0'),

    totalKeys,
    expires,
    keyTypes,

    connectedClients: parseInt(cli['connected_clients'] || '0', 10),
    blockedClients: parseInt(cli['blocked_clients'] || '0', 10),
    trackingClients: parseInt(cli['tracking_clients'] || '0', 10),
    maxClients: parseInt(cli['maxclients'] || '0', 10),
    rejectedConnections: parseInt(cli['rejected_connections'] || '0', 10),

    opsPerSec: parseInt(stat['instantaneous_ops_per_sec'] || '0', 10),
    totalCommandsProcessed: parseInt(stat['total_commands_processed'] || '0', 10),
    instantaneousOpsPerSec: parseInt(stat['instantaneous_ops_per_sec'] || '0', 10),
    keyspaceHits: parseInt(stat['keyspace_hits'] || '0', 10),
    keyspaceMisses: parseInt(stat['keyspace_misses'] || '0', 10),
  }
}

// ── 数据获取 ──────────────────────────────────────────
async function fetchInfo() {
  if (!connected.value) return

  loading.value = true
  error.value = null

  try {
    const result = await session.execute('INFO')

    if (result.type === 'error') {
      error.value = result.message
      return
    }

    if (result.type === 'response' && result.value.type === 'Bulk' && result.value.value) {
      infoData.value = parseInfoResponse(result.value.value)
      lastUpdate.value = Date.now()
    } else {
      error.value = t('redis.monitor.parseError')
    }
  } catch (e: any) {
    error.value = e.message || t('redis.monitor.fetchError')
  } finally {
    loading.value = false
  }
}

// ── 自动刷新 ──────────────────────────────────────────
function startAutoRefresh() {
  stopAutoRefresh()
  if (connected.value) {
    refreshTimer = setInterval(() => {
      if (connected.value) {
        fetchInfo()
      }
    }, refreshInterval.value * 1000)
  }
}

function stopAutoRefresh() {
  if (refreshTimer) {
    clearInterval(refreshTimer)
    refreshTimer = null
  }
}

watch(connected, (val) => {
  if (val) {
    fetchInfo()
    startAutoRefresh()
  } else {
    stopAutoRefresh()
    infoData.value = null
    error.value = null
  }
}, { immediate: true })

watch(refreshInterval, () => {
  if (connected.value) {
    startAutoRefresh()
  }
})

onUnmounted(() => {
  stopAutoRefresh()
})

// ── 格式化工具 ────────────────────────────────────────
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  const idx = Math.min(i, units.length - 1)
  return (bytes / Math.pow(k, idx)).toFixed(idx === 0 ? 0 : 1) + ' ' + units[idx]
}

function formatNumber(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString()
}
</script>

<style scoped>
.redis-monitor {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow-y: auto;
}

/* ── 工具栏 ──────────────────────────────────── */
.monitor-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-sm) var(--sp-lg);
  border-bottom: 1px solid var(--border);
  background: var(--bg-surface);
  flex-shrink: 0;
}

.monitor-title {
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  font-weight: 600;
  color: var(--text-primary);
}

.monitor-toolbar-right {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
}

.monitor-last-update {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.monitor-interval-label {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  font-size: var(--fs-xs);
  color: var(--text-secondary);
}

.monitor-interval-select {
  padding: 2px 6px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-deep);
  color: var(--text-primary);
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  outline: none;
  cursor: pointer;
}

.monitor-interval-select:focus {
  border-color: var(--border-focus);
}

.monitor-refresh-btn {
  padding: 3px var(--sp-sm);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
  font-size: var(--fs-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.monitor-refresh-btn:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
  border-color: var(--accent);
}

.monitor-refresh-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* ── 状态 ────────────────────────────────────── */
.monitor-error {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-md) var(--sp-lg);
  margin: var(--sp-lg);
  background: rgba(248, 81, 73, 0.08);
  border: 1px solid rgba(248, 81, 73, 0.2);
  border-radius: var(--radius-md);
  color: var(--danger);
  font-size: var(--fs-sm);
}

.monitor-error-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--danger);
  color: #fff;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}

.monitor-retry-btn {
  margin-left: auto;
  padding: 2px var(--sp-sm);
  border: 1px solid var(--danger);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--danger);
  font-size: var(--fs-xs);
  cursor: pointer;
}

.monitor-retry-btn:hover {
  background: rgba(248, 81, 73, 0.1);
}

.monitor-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-md);
  padding: var(--sp-3xl);
  color: var(--text-muted);
}

.monitor-empty-icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--bg-elevated);
  font-family: var(--font-mono);
  font-size: 20px;
  font-weight: 700;
  color: var(--text-muted);
}

.monitor-empty p {
  font-size: var(--fs-sm);
}

.monitor-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-md);
  padding: var(--sp-3xl);
  color: var(--text-secondary);
  font-size: var(--fs-sm);
}

.monitor-spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: monitor-spin 0.8s linear infinite;
}

@keyframes monitor-spin {
  to { transform: rotate(360deg); }
}

/* ── 网格布局 ────────────────────────────────── */
.monitor-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--sp-lg);
  padding: var(--sp-lg);
  flex: 1;
}

@media (max-width: 900px) {
  .monitor-grid {
    grid-template-columns: 1fr;
  }
}

/* ── 卡片 ────────────────────────────────────── */
.monitor-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
  transition: border-color var(--transition-base), box-shadow var(--transition-base);
}

.monitor-card:hover {
  border-color: rgba(232, 145, 45, 0.15);
  box-shadow: var(--phosphor-glow);
}

.monitor-card-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-md) var(--sp-lg);
  border-bottom: 1px solid var(--border);
  background: var(--bg-elevated);
}

.monitor-card-icon {
  font-family: var(--font-mono);
  font-size: var(--fs-md);
  font-weight: 700;
}

.monitor-card-title {
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  font-weight: 600;
  color: var(--text-primary);
}

.monitor-card-body {
  padding: var(--sp-lg);
  display: flex;
  flex-direction: column;
  gap: var(--sp-md);
}

/* ── 主统计值 ────────────────────────────────── */
.monitor-stat-main {
  display: flex;
  align-items: baseline;
  gap: var(--sp-sm);
}

.monitor-stat-value {
  font-family: var(--font-mono);
  font-size: var(--fs-2xl);
  font-weight: 700;
  line-height: 1;
  color: var(--text-primary);
}

.monitor-stat-sub {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

/* ── 进度条 ──────────────────────────────────── */
.monitor-progress-wrapper {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.monitor-progress-track {
  flex: 1;
  height: 8px;
  background: var(--bg-deep);
  border-radius: 4px;
  overflow: hidden;
}

.monitor-progress-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.4s ease, background 0.4s ease;
}

.monitor-progress-fill.monitor-percent-ok {
  background: var(--success);
}

.monitor-progress-fill.monitor-percent-warning {
  background: var(--warning);
}

.monitor-progress-fill.monitor-percent-danger {
  background: var(--danger);
}

.monitor-progress-label {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  font-weight: 600;
  min-width: 48px;
  text-align: right;
}

.monitor-progress-label.monitor-percent-ok {
  color: var(--success);
}

.monitor-progress-label.monitor-percent-warning {
  color: var(--warning);
}

.monitor-progress-label.monitor-percent-danger {
  color: var(--danger);
}

/* ── 详情行 ──────────────────────────────────── */
.monitor-detail-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-xs) 0;
  border-top: 1px solid var(--border);
}

.monitor-detail-label {
  font-size: var(--fs-xs);
  color: var(--text-secondary);
}

.monitor-detail-value {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--text-primary);
}

.monitor-value-warn {
  color: var(--warning);
}

.monitor-value-ok {
  color: var(--success);
}

/* ── 键类型分布 ──────────────────────────────── */
.monitor-key-types {
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
}

.monitor-key-type-row {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.monitor-key-type-name {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--text-secondary);
  min-width: 32px;
}

.monitor-key-type-bar-track {
  flex: 1;
  height: 6px;
  background: var(--bg-deep);
  border-radius: 3px;
  overflow: hidden;
}

.monitor-key-type-bar-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 3px;
  transition: width 0.4s ease;
}

.monitor-key-type-count {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  min-width: 36px;
  text-align: right;
}

.monitor-no-keys {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  text-align: center;
  padding: var(--sp-sm) 0;
}
</style>
