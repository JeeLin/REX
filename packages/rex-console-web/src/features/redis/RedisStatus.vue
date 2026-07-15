<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { getInfo, type RedisInfo } from '@/api/redis'

const props = defineProps<{ sessionId: string }>()

const info = ref<RedisInfo | null>(null)
const loading = ref(false)
let timer: ReturnType<typeof setInterval> | null = null

async function refresh() {
  if (!props.sessionId) return
  loading.value = true
  try {
    info.value = await getInfo(props.sessionId)
  } catch { /* ignore */ }
  finally { loading.value = false }
}

onMounted(() => {
  refresh()
  timer = setInterval(refresh, 5000)
})

onBeforeUnmount(() => {
  if (timer) clearInterval(timer)
})
</script>

<template>
  <div class="redis-status">
    <div v-if="!info" class="status-loading">{{ loading ? 'Loading...' : 'No data' }}</div>
    <template v-else>
      <div class="status-cards">
        <div class="status-card">
          <div class="card-title">Server</div>
          <div class="card-row"><span class="muted">Version</span><span>{{ info.redis_version }}</span></div>
          <div class="card-row"><span class="muted">OS</span><span>{{ info.os }}</span></div>
          <div class="card-row"><span class="muted">PID</span><span>{{ info.process_id }}</span></div>
        </div>
        <div class="status-card">
          <div class="card-title">Memory</div>
          <div class="card-row"><span class="muted">Used</span><span>{{ info.used_memory }}</span></div>
          <div class="card-row"><span class="muted">Peak</span><span>{{ info.used_memory_peak }}</span></div>
        </div>
        <div class="status-card">
          <div class="card-title">Stats</div>
          <div class="card-row"><span class="muted">Clients</span><span>{{ info.connected_clients }}</span></div>
          <div class="card-row"><span class="muted">Commands</span><span>{{ info.total_commands_processed }}</span></div>
        </div>
      </div>
      <div v-if="info.keyspace.length" class="status-keyspace">
        <div class="ks-title">Keyspace</div>
        <table class="ks-table">
          <thead><tr><th>DB</th><th>Keys</th><th>Expires</th></tr></thead>
          <tbody>
            <tr v-for="ks in info.keyspace" :key="ks.db">
              <td>{{ ks.db }}</td>
              <td>{{ ks.keys }}</td>
              <td>{{ ks.expires }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </div>
</template>

<style scoped>
.redis-status {
  padding: var(--space-4);
  overflow-y: auto;
  height: 100%;
}

.status-loading {
  text-align: center;
  color: var(--text-muted);
  padding: var(--space-8);
}

.status-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: var(--space-3);
  margin-bottom: var(--space-4);
}

.status-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--space-3);
}

.card-title {
  font-size: var(--text-xs);
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: var(--space-2);
}

.card-row {
  display: flex;
  justify-content: space-between;
  font-size: var(--text-sm);
  padding: var(--space-1) 0;
}

.muted { color: var(--text-muted); }

.ks-title {
  font-size: var(--text-xs);
  color: var(--text-muted);
  text-transform: uppercase;
  margin-bottom: var(--space-2);
}

.ks-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-sm);
}

.ks-table th,
.ks-table td {
  padding: var(--space-1) var(--space-3);
  text-align: left;
  border-bottom: 1px solid var(--border);
}

.ks-table thead {
  background: var(--bg-surface);
}
</style>
