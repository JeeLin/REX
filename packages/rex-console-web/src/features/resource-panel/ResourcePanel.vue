<script setup lang="ts">
import { ref, computed } from 'vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

interface Connection {
  id: string
  name: string
  host: string
  protocol: 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3'
  status: StatusDotStatus
  group: string
  favorite?: boolean
  lastUsed?: string
}

const connections = ref<Connection[]>([
  { id: '1', name: 'Web Server', host: '10.0.1.5', protocol: 'ssh', status: 'online', group: 'Production', favorite: true, lastUsed: '2m ago' },
  { id: '2', name: 'DB Primary', host: 'db.internal', protocol: 'mysql', status: 'online', group: 'Production', favorite: true, lastUsed: '5m ago' },
  { id: '3', name: 'Cache', host: 'cache.local', protocol: 'redis', status: 'offline', group: 'Production', lastUsed: '1h ago' },
  { id: '4', name: 'Analytics', host: 'analytics.db', protocol: 'postgresql', status: 'connecting', group: 'Staging', lastUsed: '3h ago' },
])

const groups = computed(() => [...new Set(connections.value.map(c => c.group))])
const favorites = computed(() => connections.value.filter(c => c.favorite))
const recent = computed(() => connections.value.filter(c => c.lastUsed).slice(0, 5))

const activeTab = ref<'all' | 'favorites' | 'recent'>('all')
const searchQuery = ref('')
const collapsedGroups = ref(new Set<string>())

const filteredConnections = computed(() => {
  let list = connections.value
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    list = list.filter(c => c.name.toLowerCase().includes(q) || c.host.toLowerCase().includes(q))
  }
  return list
})

const protoColor = (proto: Connection['protocol']) => `var(--proto-${proto})`

function toggleGroup(group: string) {
  if (collapsedGroups.value.has(group)) {
    collapsedGroups.value.delete(group)
  } else {
    collapsedGroups.value.add(group)
  }
}

function toggleFavorite(id: string) {
  const conn = connections.value.find(c => c.id === id)
  if (conn) conn.favorite = !conn.favorite
}

defineExpose({ connections })
</script>

<template>
  <div class="resource-panel">
    <!-- Header -->
    <div class="rp-header">
      <div class="rp-tabs">
        <button
          v-for="tab in [
            { key: 'all' as const, label: 'All' },
            { key: 'favorites' as const, label: '★' },
            { key: 'recent' as const, label: 'Recent' },
          ]"
          :key="tab.key"
          class="rp-tab mono"
          :class="{ 'rp-tab--active': activeTab === tab.key }"
          @click="activeTab = tab.key"
        >
          {{ tab.label }}
        </button>
      </div>
      <button class="rp-add" title="New connection">+</button>
    </div>

    <!-- Search -->
    <div class="rp-search">
      <input
        v-model="searchQuery"
        type="text"
        class="rp-search-input mono"
        placeholder="Search..."
      />
    </div>

    <!-- Content -->
    <div class="rp-content">
      <!-- Favorites tab -->
      <template v-if="activeTab === 'favorites'">
        <div v-if="favorites.length === 0" class="rp-empty muted">No favorites yet</div>
        <div v-for="conn in favorites" :key="conn.id" class="rp-item">
          <StatusDot :status="conn.status" />
          <span class="rp-item-proto mono" :style="{ color: protoColor(conn.protocol) }">
            {{ conn.protocol.toUpperCase() }}
          </span>
          <span class="rp-item-name">{{ conn.name }}</span>
          <span class="rp-item-host mono muted">{{ conn.host }}</span>
        </div>
      </template>

      <!-- Recent tab -->
      <template v-else-if="activeTab === 'recent'">
        <div v-if="recent.length === 0" class="rp-empty muted">No recent connections</div>
        <div v-for="conn in recent" :key="conn.id" class="rp-item">
          <StatusDot :status="conn.status" />
          <span class="rp-item-proto mono" :style="{ color: protoColor(conn.protocol) }">
            {{ conn.protocol.toUpperCase() }}
          </span>
          <span class="rp-item-name">{{ conn.name }}</span>
          <span class="rp-item-time muted">{{ conn.lastUsed }}</span>
        </div>
      </template>

      <!-- All connections (grouped) -->
      <template v-else>
        <template v-for="group in groups" :key="group">
          <div class="rp-group" @click="toggleGroup(group)">
            <span class="rp-chevron" :class="{ 'rp-collapsed': collapsedGroups.has(group) }">▸</span>
            <span class="rp-group-name mono">{{ group }}</span>
          </div>
          <div v-if="!collapsedGroups.has(group)">
            <div
              v-for="conn in filteredConnections.filter(c => c.group === group)"
              :key="conn.id"
              class="rp-item"
            >
              <StatusDot :status="conn.status" />
              <span class="rp-item-proto mono" :style="{ color: protoColor(conn.protocol) }">
                {{ conn.protocol.toUpperCase() }}
              </span>
              <span class="rp-item-name">{{ conn.name }}</span>
              <span class="rp-item-host mono muted">{{ conn.host }}</span>
              <button
                class="rp-fav"
                :class="{ 'rp-fav--active': conn.favorite }"
                @click.stop="toggleFavorite(conn.id)"
                title="Toggle favorite"
              >
                {{ conn.favorite ? '★' : '☆' }}
              </button>
            </div>
          </div>
        </template>
      </template>
    </div>
  </div>
</template>

<style scoped>
.resource-panel {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
}

/* Header */
.rp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border);
}
.rp-tabs {
  display: flex;
}
.rp-tab {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-muted);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  transition: color var(--transition);
}
.rp-tab:hover {
  color: var(--text-secondary);
}
.rp-tab--active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}
.rp-add {
  padding: 0 var(--space-3);
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-lg);
  cursor: pointer;
}
.rp-add:hover {
  color: var(--accent);
}

/* Search */
.rp-search {
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
}
.rp-search-input {
  width: 100%;
  height: 28px;
  padding: 0 var(--space-2);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-primary);
  font-size: var(--text-xs);
  outline: none;
}
.rp-search-input::placeholder {
  color: var(--text-muted);
}
.rp-search-input:focus {
  border-color: var(--accent);
}

/* Content */
.rp-content {
  flex: 1;
  overflow-y: auto;
}
.rp-empty {
  padding: var(--space-4);
  text-align: center;
  font-size: var(--text-sm);
}
.rp-group {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-muted);
  cursor: pointer;
  user-select: none;
}
.rp-group:hover {
  color: var(--text-secondary);
}
.rp-chevron {
  font-size: 10px;
  transition: transform var(--transition);
}
.rp-collapsed {
  transform: rotate(0deg);
}
.rp-group-name {
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}
.rp-item {
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
.rp-item:hover {
  background: var(--bg-hover);
}
.rp-item-proto {
  font-size: var(--text-xs);
  font-weight: 600;
}
.rp-item-name {
  color: var(--text-primary);
}
.rp-item-host {
  font-size: var(--text-xs);
  margin-left: auto;
}
.rp-item-time {
  font-size: var(--text-xs);
  margin-left: auto;
}
.rp-fav {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-sm);
  cursor: pointer;
  padding: 0 2px;
}
.rp-fav--active {
  color: var(--accent);
}
.rp-fav:hover {
  color: var(--accent);
}
</style>
