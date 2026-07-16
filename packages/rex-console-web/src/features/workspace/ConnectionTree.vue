<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import StatusDot from '@/components/ui/StatusDot.vue'
import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'
import { useEnvironmentsStore } from '@/stores/environments'
import { resourcesApi, type Resource } from '@/api/resources'
import { PROTOCOL_ICONS, PROTOCOL_COLORS } from '@/features/resource/protocols'

interface TreeNode {
  id: string
  type: 'group' | 'resource'
  name: string
  protocol?: 'ssh' | 'mysql' | 'redis' | 'postgresql' | 'sftp' | 'sqlite' | 's3'
  host?: string
  port?: number
  username?: string
  environmentId?: string
  status?: StatusDotStatus
  color?: string
  children?: TreeNode[]
}

const emit = defineEmits<{ openResource: [node: TreeNode] }>()

const store = useEnvironmentsStore()
const searchQuery = ref('')
const collapsedGroups = ref(new Set<string>())
const envResources = ref<Map<string, Resource[]>>(new Map())

onMounted(async () => {
  await store.fetchEnvironments()
  for (const env of store.environments) {
    try {
      const resources = await resourcesApi.listByEnv(env.id)
      envResources.value.set(env.id, resources)
    } catch {
      // ignore
    }
  }
})

const treeData = computed<TreeNode[]>(() => {
  return store.environments.map(env => ({
    id: env.id,
    type: 'group' as const,
    name: env.name,
    children: (envResources.value.get(env.id) || []).map(res => ({
      id: res.id,
      type: 'resource' as const,
      name: res.name,
      protocol: res.protocol as TreeNode['protocol'],
      host: res.host,
      port: res.port || undefined,
      username: res.username || undefined,
      environmentId: env.id,
      status: 'offline' as StatusDotStatus,
      color: res.color || undefined,
    })),
  }))
})

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
</script>

<template>
  <div class="conn-tree">
    <div class="ct-search">
      <input
        v-model="searchQuery"
        type="text"
        class="ct-search-input mono"
        placeholder="Search..."
      />
    </div>
    <div class="ct-content">
      <div v-if="filteredTree.length === 0 && !store.loading" class="ct-empty muted">
        No environments
      </div>
      <template v-for="group in filteredTree" :key="group.id">
        <div class="ct-group" @click="toggleGroup(group.id)">
          <span class="ct-chevron" :class="{ 'ct-collapsed': collapsedGroups.has(group.id) }">▸</span>
          <span class="ct-group-name mono">{{ group.name }}</span>
          <span class="ct-group-count muted">{{ group.children?.length || 0 }}</span>
        </div>
        <div v-if="!collapsedGroups.has(group.id)">
          <div
            v-for="item in group.children"
            :key="item.id"
            class="ct-item"
            @dblclick="emit('openResource', item)"
          >
            <span class="ct-item-icon" :style="{ color: item.color || PROTOCOL_COLORS[item.protocol || ''] || 'var(--text-secondary)' }">
              {{ PROTOCOL_ICONS[item.protocol || ''] || '?' }}
            </span>
            <span class="ct-item-name">{{ item.name }}</span>
            <span class="ct-item-host mono muted">{{ item.host }}</span>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.conn-tree {
  display: flex;
  flex-direction: column;
  height: 100%;
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
.ct-empty {
  padding: var(--space-4);
  text-align: center;
  font-size: var(--text-sm);
}
.ct-group {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-2);
  font-size: var(--text-xs);
  color: var(--text-muted);
  cursor: pointer;
  user-select: none;
}
.ct-group:hover {
  color: var(--text-secondary);
}
.ct-chevron {
  font-size: 10px;
  transition: transform var(--transition);
}
.ct-collapsed {
  transform: rotate(0deg);
}
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
.ct-item:hover {
  background: var(--bg-hover);
}
.ct-item-icon {
  font-family: var(--font-mono);
  font-size: 12px;
  width: 16px;
  text-align: center;
}
.ct-item-name {
  color: var(--text-primary);
}
.ct-item-host {
  font-size: var(--text-xs);
  margin-left: auto;
}
.muted {
  color: var(--text-muted);
}
.mono {
  font-family: var(--font-mono);
}
</style>
