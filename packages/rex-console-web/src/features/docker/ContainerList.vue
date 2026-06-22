<template>
  <div class="docker-container-list">
    <div class="docker-filter-bar">
      <input
        v-model="searchQuery"
        class="docker-search"
        :placeholder="t('docker.filter.searchPlaceholder')"
      />
      <label class="docker-filter-check">
        <input v-model="showAll" type="checkbox" />
        {{ t('docker.filter.showStopped') }}
      </label>
      <button class="docker-btn docker-btn-sm" :disabled="loading" @click="$emit('refresh')">
        {{ t('common.refresh') }}
      </button>
    </div>

    <div class="docker-table-wrap">
      <table class="docker-table">
        <thead>
          <tr>
            <th></th>
            <th>{{ t('docker.table.name') }}</th>
            <th>{{ t('docker.table.image') }}</th>
            <th>{{ t('docker.table.state') }}</th>
            <th>{{ t('docker.table.ports') }}</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="c in filteredContainers"
            :key="c.id"
            class="docker-row"
            :class="{ selected: selectedId === c.id }"
            @click="$emit('select', c)"
            @contextmenu.prevent="$emit('contextmenu', $event, c)"
          >
            <td>
              <span class="docker-state-dot" :class="stateClass(c.state)" />
            </td>
            <td class="docker-cell-name">{{ c.name }}</td>
            <td class="docker-cell-image">{{ c.image }}</td>
            <td>
              <span class="docker-state-badge" :class="stateClass(c.state)">
                {{ c.state }}
              </span>
            </td>
            <td class="docker-cell-ports">
              <template v-for="(p, i) in c.ports" :key="i">
                <span class="docker-port">{{ p.public ? `${p.public}→` : '' }}{{ p.private }}/{{ p.protocol }}</span>
              </template>
              <span v-if="c.ports.length === 0" class="docker-no-ports">—</span>
            </td>
            <td>
              <button class="docker-action-btn" @click.stop="$emit('contextmenu', $event, c)">
                ▾
              </button>
            </td>
          </tr>
          <tr v-if="filteredContainers.length === 0">
            <td colspan="6" class="docker-empty">
              {{ t('docker.table.noContainers') }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="docker-stats">
      {{ t('docker.stats.total', { total: containers.length }) }}
      · {{ t('docker.stats.running', { count: runningCount }) }}
      · {{ t('docker.stats.stopped', { count: stoppedCount }) }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { DockerContainerInfo, DockerContainerState } from '@/api/docker'

const props = defineProps<{
  containers: DockerContainerInfo[]
  loading: boolean
  selectedId: string | null
}>()

defineEmits<{
  select: [container: DockerContainerInfo]
  contextmenu: [event: MouseEvent, container: DockerContainerInfo]
  refresh: []
}>()

const { t } = useI18n()

const searchQuery = ref('')
const showAll = ref(true)

const filteredContainers = computed(() => {
  let list = props.containers
  if (!showAll.value) {
    list = list.filter(c => c.state === 'Running' || c.state === 'Paused')
  }
  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase()
    list = list.filter(c =>
      c.name.toLowerCase().includes(q) ||
      c.image.toLowerCase().includes(q)
    )
  }
  return list
})

const runningCount = computed(() => props.containers.filter(c => c.state === 'Running').length)
const stoppedCount = computed(() => props.containers.filter(c => c.state !== 'Running').length)

function stateClass(state: DockerContainerState): string {
  switch (state) {
    case 'Running': return 'state-running'
    case 'Paused': return 'state-paused'
    case 'Stopped': return 'state-stopped'
    case 'Created': return 'state-created'
    case 'Dead': return 'state-dead'
    default: return ''
  }
}
</script>

<style scoped>
.docker-filter-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  flex-shrink: 0;
}

.docker-search {
  flex: 1;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-primary);
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  outline: none;
}

.docker-search::placeholder { color: var(--text-secondary); }
.docker-search:focus { border-color: var(--accent); }

.docker-filter-check {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--text-secondary);
  font-size: 12px;
  white-space: nowrap;
  cursor: pointer;
}

.docker-filter-check input { cursor: pointer; }

.docker-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-primary);
  padding: 3px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  white-space: nowrap;
}

.docker-btn:hover:not(:disabled) { background: var(--bg-hover); }
.docker-btn:disabled { opacity: 0.5; cursor: not-allowed; }

/* Table */
.docker-table-wrap {
  flex: 1;
  overflow-y: auto;
}

.docker-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.docker-table th {
  text-align: left;
  padding: 6px 12px;
  color: var(--text-secondary);
  font-weight: 500;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--border-primary);
  position: sticky;
  top: 0;
  background: var(--bg-primary);
}

.docker-table td {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-primary);
}

.docker-row {
  cursor: pointer;
  transition: background 0.15s;
}

.docker-row:hover { background: var(--bg-hover); }
.docker-row.selected { background: var(--bg-active); }

/* State dots */
.docker-state-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.state-running { background: #3fb950; }
.state-paused { background: #d29922; }
.state-stopped { background: #6e7681; }
.state-created { background: #8b949e; }
.state-dead { background: #f85149; }

/* State badge */
.docker-state-badge {
  display: inline-block;
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 11px;
  font-weight: 500;
}

.docker-state-badge.state-running { background: #3fb95022; color: #3fb950; }
.docker-state-badge.state-paused { background: #d2992222; color: #d29922; }
.docker-state-badge.state-stopped { background: #6e768122; color: #8b949e; }
.docker-state-badge.state-created { background: #8b949e22; color: #8b949e; }
.docker-state-badge.state-dead { background: #f8514922; color: #f85149; }

/* Cells */
.docker-cell-name {
  font-weight: 500;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
}

.docker-cell-image {
  color: var(--text-secondary);
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 12px;
}

.docker-cell-ports {
  font-size: 11px;
  color: var(--text-secondary);
}

.docker-port {
  margin-right: 4px;
}

.docker-no-ports {
  color: var(--text-secondary);
}

.docker-empty {
  text-align: center;
  color: var(--text-secondary);
  padding: 24px 12px;
}

.docker-action-btn {
  background: none;
  border: 1px solid transparent;
  color: var(--text-secondary);
  padding: 2px 6px;
  border-radius: 3px;
  cursor: pointer;
  font-size: 12px;
}

.docker-action-btn:hover {
  background: var(--bg-tertiary);
  border-color: var(--border-primary);
  color: var(--text-primary);
}

/* Stats */
.docker-stats {
  padding: 6px 12px;
  border-top: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  font-size: 11px;
  color: var(--text-secondary);
  flex-shrink: 0;
}
</style>
