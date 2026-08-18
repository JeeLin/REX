<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { cdrApi, type CdrRecord, type CdrDirection, type CdrState } from '@/api/sip'
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'
import Button from '@/components/ui/Button.vue'
import Select from '@/components/ui/Select.vue'
import EmptyState from '@/components/ui/EmptyState.vue'
import ResponsiveTable from '@/components/ResponsiveTable.vue'

const { t } = useI18n()

const records = ref<CdrRecord[]>([])
const loading = ref(true)
const totalCount = ref(0)
const currentPage = ref(1)
const pageSize = ref(50)
const pageSizeOptions = [
  { label: '20', value: 20 },
  { label: '50', value: 50 },
  { label: '100', value: 100 },
]

// 详情抽屉
const detail = ref<CdrRecord | null>(null)

// 过滤
const directionFilter = ref<CdrDirection | ''>('')
const stateFilter = ref<CdrState | ''>('')

const directionOptions = [
  { label: t('cdr.allDirections'), value: '' },
  { label: t('cdr.out'), value: 'out' },
  { label: t('cdr.in'), value: 'in' },
]
const stateOptions = [
  { label: t('cdr.allStates'), value: '' },
  { label: t('cdr.ringing'), value: 'ringing' },
  { label: t('cdr.active'), value: 'active' },
  { label: t('cdr.held'), value: 'held' },
  { label: t('cdr.ended'), value: 'ended' },
  { label: t('cdr.missed'), value: 'missed' },
]

async function fetchRecords() {
  loading.value = true
  try {
    const offset = (currentPage.value - 1) * pageSize.value
    const res = await cdrApi.list({
      direction: directionFilter.value || undefined,
      state: stateFilter.value || undefined,
      limit: pageSize.value,
      offset,
    })
    records.value = res.records
    totalCount.value = res.total
  } catch {
    records.value = []
    totalCount.value = 0
  } finally {
    loading.value = false
  }
}

function refreshAll() {
  fetchRecords()
}

function openDetail(rec: CdrRecord) {
  detail.value = rec
}

function closeDetail() {
  detail.value = null
}

function stateBadgeTone(state: CdrState): 'info' | 'success' | 'warning' | 'danger' | 'neutral' {
  switch (state) {
    case 'active': return 'success'
    case 'ringing': return 'warning'
    case 'held': return 'info'
    case 'ended': return 'neutral'
    case 'missed': return 'danger'
  }
}

function dirBadgeTone(dir: CdrDirection): 'info' | 'success' {
  return dir === 'out' ? 'info' : 'success'
}

function formatDuration(sec: number): string {
  if (!sec || sec <= 0) return '—'
  const m = Math.floor(sec / 60)
  const s = sec % 60
  return m > 0 ? `${m}m ${s}s` : `${s}s`
}

function shortPeer(peer: string): string {
  if (!peer) return '—'
  return peer
}

function formatTime(time: string): string {
  if (!time) return '—'
  try {
    return new Date(time).toLocaleString()
  } catch {
    return time
  }
}

const totalPages = computed(() => Math.max(1, Math.ceil(totalCount.value / pageSize.value)))
const gotoPage = ref(1)

watch([directionFilter, stateFilter], () => {
  currentPage.value = 1
  gotoPage.value = 1
  refreshAll()
})

watch(pageSize, () => {
  currentPage.value = 1
  gotoPage.value = 1
  refreshAll()
})

function applyGoto() {
  const target = Math.min(Math.max(1, Math.floor(gotoPage.value || 1)), totalPages.value)
  currentPage.value = target
}

watch(currentPage, () => {
  gotoPage.value = currentPage.value
  fetchRecords()
})

onMounted(() => {
  refreshAll()
})
</script>

<template>
  <div class="page-container cdr-page">
    <header class="page-header">
      <div class="page-header-left">
        <h1 class="page-title mono">{{ t('cdr.title') }}</h1>
        <span class="page-subtitle">{{ t('cdr.subtitle') }}</span>
      </div>
      <div class="page-header-actions">
        <Button variant="ghost" size="sm" @click="refreshAll">↻ {{ t('common.refresh') }}</Button>
      </div>
    </header>

    <div class="filters">
      <Select v-model="directionFilter" :options="directionOptions" size="sm" />
      <Select v-model="stateFilter" :options="stateOptions" size="sm" />
    </div>

    <EmptyState
      v-if="!loading && records.length === 0"
      icon="📞"
      :title="t('cdr.noRecords')"
      :description="t('cdr.emptyDesc')"
    />

    <Card v-else class="cdr-card">
      <div v-if="loading" class="loading muted">{{ t('common.loadingEllipsis') }}</div>
      <ResponsiveTable v-else>
        <table class="cdr-table">
          <thead>
            <tr>
              <th>{{ t('cdr.peer') }}</th>
              <th>{{ t('cdr.direction') }}</th>
              <th>{{ t('cdr.start') }}</th>
              <th>{{ t('cdr.duration') }}</th>
              <th>{{ t('cdr.state') }}</th>
              <th>{{ t('cdr.media') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="rec in records"
              :key="rec.id"
              class="cdr-row"
              @click="openDetail(rec)"
            >
              <td class="mono">{{ shortPeer(rec.peer) }}</td>
              <td><Badge :tone="dirBadgeTone(rec.direction)">{{ rec.direction === 'out' ? t('cdr.out') : t('cdr.in') }}</Badge></td>
              <td class="mono">{{ formatTime(rec.start_time) }}</td>
              <td class="mono">{{ formatDuration(rec.duration_sec) }}</td>
              <td><Badge :tone="stateBadgeTone(rec.state)">{{ rec.state }}</Badge></td>
              <td>
                <span v-if="rec.recording_url" class="media-tag">🎙 {{ t('cdr.rec') }}</span>
                <span v-if="rec.pcap_url" class="media-tag">📡 {{ t('cdr.pcap') }}</span>
                <span v-if="!rec.recording_url && !rec.pcap_url" class="muted">—</span>
              </td>
            </tr>
          </tbody>
        </table>
      </ResponsiveTable>
    </Card>

    <div class="pagination">
      <span class="page-total muted">{{ t('cdr.totalCount', { n: totalCount }) }}</span>
      <Select v-model="pageSize" :options="pageSizeOptions" size="sm" />
      <button class="page-btn" :disabled="currentPage <= 1" @click="currentPage--">← {{ t('common.prev', 'Prev') }}</button>
      <span class="page-info mono">{{ currentPage }} / {{ totalPages }}</span>
      <button class="page-btn" :disabled="currentPage >= totalPages" @click="currentPage++">{{ t('common.next', 'Next') }} →</button>
      <span class="page-goto">
        <span class="muted">{{ t('cdr.gotoPage') }}</span>
        <input
          v-model.number="gotoPage"
          class="page-goto-input mono"
          type="number"
          min="1"
          :max="totalPages"
          @keyup.enter="applyGoto"
        />
      </span>
    </div>

    <!-- 详情抽屉 -->
    <div v-if="detail" class="drawer-overlay" @click.self="closeDetail">
      <aside class="drawer">
        <header class="drawer-header">
          <h2 class="drawer-title mono">{{ shortPeer(detail.peer) }}</h2>
          <button class="drawer-close" @click="closeDetail">✕</button>
        </header>
        <div class="drawer-body">
          <div class="detail-field">
            <span class="detail-label muted">{{ t('cdr.id') }}</span>
            <span class="mono">{{ detail.id }}</span>
          </div>
          <div class="detail-field">
            <span class="detail-label muted">{{ t('cdr.callId') }}</span>
            <span class="mono">{{ detail.call_id }}</span>
          </div>
          <div class="detail-field">
            <span class="detail-label muted">{{ t('cdr.direction') }}</span>
            <Badge :tone="dirBadgeTone(detail.direction)">{{ detail.direction === 'out' ? t('cdr.out') : t('cdr.in') }}</Badge>
          </div>
          <div class="detail-field">
            <span class="detail-label muted">{{ t('cdr.start') }}</span>
            <span class="mono">{{ formatTime(detail.start_time) }}</span>
          </div>
          <div class="detail-field">
            <span class="detail-label muted">{{ t('cdr.end') }}</span>
            <span class="mono">{{ formatTime(detail.end_time || '') }}</span>
          </div>
          <div class="detail-field">
            <span class="detail-label muted">{{ t('cdr.duration') }}</span>
            <span class="mono">{{ formatDuration(detail.duration_sec) }}</span>
          </div>
          <div class="detail-field">
            <span class="detail-label muted">{{ t('cdr.state') }}</span>
            <Badge :tone="stateBadgeTone(detail.state)">{{ detail.state }}</Badge>
          </div>

          <div v-if="detail.recording_url || detail.pcap_url" class="detail-section">
            <h3 class="section-title">{{ t('cdr.assets') }}</h3>
            <div v-if="detail.recording_url" class="asset-block">
              <audio class="player" :src="detail.recording_url" controls></audio>
              <a class="download-link" :href="detail.recording_url" download>{{ t('cdr.downloadRec') }}</a>
            </div>
            <div v-if="detail.pcap_url" class="asset-block">
              <a class="download-link" :href="detail.pcap_url" download>{{ t('cdr.downloadPcap') }}</a>
            </div>
          </div>
        </div>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.cdr-page { height: 100%; overflow-y: auto; }
.page-header-actions { display: flex; gap: var(--space-2); }
.filters { display: flex; gap: var(--space-2); align-items: center; margin-bottom: var(--space-4); flex-wrap: wrap; }
.cdr-card { overflow-x: auto; }
.loading { padding: var(--space-6); text-align: center; }
.cdr-table { width: 100%; border-collapse: collapse; font-size: var(--text-sm); }
.cdr-table th { text-align: left; padding: var(--space-2) var(--space-3); color: var(--text-muted); font-weight: 500; border-bottom: 1px solid var(--border); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.5px; }
.cdr-table td { padding: var(--space-2) var(--space-3); border-bottom: 1px solid var(--border); color: var(--text-secondary); }
.cdr-row { cursor: pointer; }
.cdr-row:hover td { background: var(--bg-hover); }
.media-tag { font-size: var(--text-xs); margin-right: var(--space-1); }
.muted { color: var(--text-muted); }
.mono { font-family: var(--font-mono); }

/* Pagination */
.pagination { display: flex; align-items: center; justify-content: center; flex-wrap: wrap; gap: var(--space-3); padding: var(--space-4) 0; }
.page-total { font-size: var(--text-xs); }
.page-btn { padding: var(--space-1) var(--space-3); background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); color: var(--text-secondary); font-size: var(--text-sm); cursor: pointer; transition: border-color var(--transition), color var(--transition); }
.page-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--text-primary); }
.page-btn:disabled { opacity: var(--disabled-opacity); cursor: not-allowed; }
.page-info { font-size: var(--text-xs); color: var(--text-muted); }
.page-goto { display: flex; align-items: center; gap: var(--space-1); font-size: var(--text-xs); }
.page-goto-input { width: 56px; background: var(--bg-deep); border: 1px solid var(--border); border-radius: var(--radius); padding: 4px 8px; color: var(--text-primary); font-size: var(--text-sm); outline: none; }
.page-goto-input:focus { border-color: var(--accent); }

/* Drawer */
.drawer-overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.5); display: flex; justify-content: flex-end; z-index: 50; }
.drawer { width: min(440px, 92vw); height: 100%; background: var(--bg-surface); border-left: 1px solid var(--border); display: flex; flex-direction: column; animation: slide-in var(--transition); }
@keyframes slide-in { from { transform: translateX(100%); } to { transform: translateX(0); } }
.drawer-header { display: flex; align-items: center; justify-content: space-between; padding: var(--space-4); border-bottom: 1px solid var(--border); }
.drawer-title { font-size: var(--text-base); margin: 0; }
.drawer-close { background: none; border: none; color: var(--text-muted); font-size: var(--text-lg); cursor: pointer; }
.drawer-close:hover { color: var(--text-primary); }
.drawer-body { padding: var(--space-4); display: flex; flex-direction: column; gap: var(--space-3); }
.detail-field { display: flex; gap: var(--space-2); font-size: var(--text-sm); align-items: baseline; }
.detail-label { font-size: var(--text-xs); min-width: 90px; flex-shrink: 0; }
.detail-section { margin-top: var(--space-2); border-top: 1px solid var(--border); padding-top: var(--space-3); }
.section-title { font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.5px; color: var(--text-muted); margin: 0 0 var(--space-2); }
.asset-block { display: flex; flex-direction: column; gap: var(--space-2); margin-bottom: var(--space-2); }
.player { width: 100%; }
.download-link { color: var(--accent); font-size: var(--text-sm); text-decoration: none; }
.download-link:hover { text-decoration: underline; }
</style>
