<script setup lang="ts">
import { ref, computed } from 'vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'

interface TreeNode {
  id: string
  type: 'group' | 'resource'
  name: string
  protocol?: 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3'
  host?: string
  status?: StatusDotStatus
  color?: string
  children?: TreeNode[]
}

const emit = defineEmits<{ openResource: [node: TreeNode] }>()

const searchQuery = ref('')
const collapsedGroups = ref(new Set<string>())

const treeData = ref<TreeNode[]>([
  {
    id: 'prod', type: 'group', name: 'Production', children: [
      { id: 'r1', type: 'resource', name: 'Web Server', protocol: 'ssh', host: '10.0.1.5', status: 'online', color: '#f85149' },
      { id: 'r2', type: 'resource', name: 'DB Primary', protocol: 'mysql', host: 'db.internal', status: 'online' },
      { id: 'r3', type: 'resource', name: 'Cache', protocol: 'redis', host: 'cache.local', status: 'offline' },
    ],
  },
  {
    id: 'staging', type: 'group', name: 'Staging', children: [
      { id: 'r4', type: 'resource', name: 'Analytics', protocol: 'postgresql', host: 'analytics.db', status: 'connecting' },
    ],
  },
  {
    id: 'dev', type: 'group', name: 'Development', children: [
      { id: 'r5', type: 'resource', name: 'Dev API', protocol: 'ssh', host: 'localhost:3000', status: 'online' },
    ],
  },
])

const filteredTree = computed(() => {
  if (!searchQuery.value) return treeData.value
  const q = searchQuery.value.toLowerCase()
  return treeData.value
    .map(group => ({
      ...group,
      children: group.children?.filter(r =>
        r.name.toLowerCase().includes(q) || r.host?.toLowerCase().includes(q)
      ),
    }))
    .filter(group => (group.children?.length ?? 0) > 0)
})

function toggleGroup(id: string) {
  if (collapsedGroups.value.has(id)) {
    collapsedGroups.value.delete(id)
  } else {
    collapsedGroups.value.add(id)
  }
}

const protoColor = (proto?: TreeNode['protocol']) => proto ? `var(--proto-${proto})` : 'var(--text-muted)'

defineExpose({ treeData })
</script>

<template>
  <div class="connection-tree">
    <div class="ct-search">
      <input
        v-model="searchQuery"
        type="text"
        class="ct-search-input mono"
        placeholder="Search..."
      />
    </div>
    <div class="ct-content">
      <template v-for="group in filteredTree" :key="group.id">
        <div class="ct-group" @click="toggleGroup(group.id)">
          <span class="ct-chevron" :class="{ 'ct-collapsed': collapsedGroups.has(group.id) }">▸</span>
          <span class="ct-group-name mono">{{ group.name }}</span>
          <span class="ct-group-count muted">{{ group.children?.length }}</span>
        </div>
        <div v-if="!collapsedGroups.has(group.id)">
          <div
            v-for="item in group.children"
            :key="item.id"
            class="ct-item"
            @dblclick="emit('openResource', item)"
          >
            <span class="ct-color-dot" v-if="item.color" :style="{ background: item.color }" />
            <span class="ct-proto mono" :style="{ color: protoColor(item.protocol) }">
              {{ item.protocol?.toUpperCase() }}
            </span>
            <span class="ct-name">{{ item.name }}</span>
            <span class="ct-host mono muted">{{ item.host }}</span>
            <StatusDot v-if="item.status" :status="item.status" />
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.connection-tree {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.ct-search {
  padding: var(--space-2);
  border-bottom: 1px solid var(--border);
}
.ct-search-input {
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
.ct-search-input:focus {
  border-color: var(--accent);
}
.ct-content {
  flex: 1;
  overflow-y: auto;
}
.ct-group {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-2) var(--space-2);
  font-size: var(--text-xs);
  color: var(--text-muted);
  cursor: pointer;
  user-select: none;
}
.ct-group:hover { color: var(--text-secondary); }
.ct-chevron {
  font-size: 10px;
  transition: transform var(--transition);
}
.ct-collapsed { transform: rotate(0deg); }
.ct-group-name {
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}
.ct-group-count {
  margin-left: auto;
  font-size: 10px;
}
.ct-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-2);
  padding-left: var(--space-4);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  cursor: pointer;
  transition: background var(--transition);
}
.ct-item:hover { background: var(--bg-hover); }
.ct-color-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}
.ct-proto {
  font-size: var(--text-xs);
  font-weight: 600;
  width: 40px;
  flex-shrink: 0;
}
.ct-name { color: var(--text-primary); }
.ct-host {
  font-size: var(--text-xs);
  margin-left: auto;
}
</style>
