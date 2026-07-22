<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useEnvironmentsStore } from '@/stores/environments'
import type { Environment } from '@/api/environments'
import type { Resource } from '@/api/resources'
import { resourcesApi } from '@/api/resources'
import { PROTOCOL_ICONS, PROTOCOL_COLORS } from '@/features/resource/protocols'
import WizardModal from '@/features/resource/WizardModal.vue'

const store = useEnvironmentsStore()

const searchQuery = ref('')
const collapsedEnvs = ref(new Set<string>())
const envResources = ref<Map<string, Resource[]>>(new Map())
const expandedEnvIds = ref(new Set<string>())
const wizardEnvId = ref('')

onMounted(async () => {
  await store.fetchEnvironments()
  // 预加载每个环境的资源
  for (const env of store.environments) {
    try {
      const resources = await resourcesApi.listByEnv(env.id)
      envResources.value.set(env.id, resources)
    } catch {
      // ignore
    }
  }
})

const filteredEnvs = computed(() => {
  let envs = store.environments
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    envs = envs.filter(e =>
      e.name.toLowerCase().includes(q) ||
      (envResources.value.get(e.id) || []).some(r =>
        r.name.toLowerCase().includes(q) || r.host.toLowerCase().includes(q)
      )
    )
  }
  return envs
})

function toggleEnv(envId: string) {
  if (collapsedEnvs.value.has(envId)) {
    collapsedEnvs.value.delete(envId)
  } else {
    collapsedEnvs.value.add(envId)
  }
}

function getResources(envId: string): Resource[] {
  return envResources.value.get(envId) || []
}

async function refreshEnv(envId: string) {
  try {
    const resources = await resourcesApi.listByEnv(envId)
    envResources.value.set(envId, resources)
  } catch { /* ignore */ }
}

function openWizard(envId: string, e: MouseEvent) {
  e.stopPropagation()
  wizardEnvId.value = envId
}

function onWizardCreated() {
  const envId = wizardEnvId.value
  wizardEnvId.value = ''
  if (envId) refreshEnv(envId)
}

defineExpose({ envResources })
</script>

<template>
  <div class="resource-panel">
    <!-- Header -->
    <div class="rp-header">
      <div class="rp-tabs">
        <button class="rp-tab mono rp-tab--active">Connections</button>
      </div>
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
      <div v-if="filteredEnvs.length === 0 && !store.loading" class="rp-empty muted">
        No environments yet
      </div>

      <template v-for="env in filteredEnvs" :key="env.id">
        <!-- Environment group -->
        <div class="rp-group" @click="toggleEnv(env.id)">
          <span class="rp-chevron" :class="{ 'rp-collapsed': collapsedEnvs.has(env.id) }">▸</span>
          <span class="rp-group-name mono">{{ env.name }}</span>
          <span class="rp-group-count muted">{{ env.resource_count }}</span>
          <button class="rp-add-btn" title="Add resource" @click="openWizard(env.id, $event)">+</button>
        </div>

        <!-- Resources under this environment -->
        <div v-if="!collapsedEnvs.has(env.id)">
          <div
            v-for="res in getResources(env.id)"
            :key="res.id"
            class="rp-item"
          >
            <span class="rp-item-icon" :style="{ color: res.color || PROTOCOL_COLORS[res.protocol] || 'var(--text-secondary)' }">
              {{ PROTOCOL_ICONS[res.protocol] || '?' }}
            </span>
            <span class="rp-item-name">{{ res.name }}</span>
            <span class="rp-item-host mono muted">{{ res.host }}</span>
          </div>
          <div v-if="getResources(env.id).length === 0" class="rp-item rp-empty-item muted">
            No resources
          </div>
        </div>
      </template>
    </div>

    <WizardModal
      v-if="wizardEnvId"
      :visible="true"
      :environment-id="wizardEnvId"
      @close="wizardEnvId = ''"
      @created="onWizardCreated"
    />
  </div>
</template>

<style scoped>
.resource-panel {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
}
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
}
.rp-tab--active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}
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
.rp-group-count {
  margin-left: auto;
  font-size: 10px;
}
.rp-add-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 14px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
  opacity: 0;
  transition: opacity var(--transition);
}
.rp-group:hover .rp-add-btn {
  opacity: 1;
}
.rp-add-btn:hover {
  color: var(--accent);
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
.rp-item-icon {
  font-family: var(--font-mono);
  font-size: 12px;
  width: 16px;
  text-align: center;
}
.rp-item-name {
  color: var(--text-primary);
}
.rp-item-host {
  font-size: var(--text-xs);
  margin-left: auto;
}
.rp-empty-item {
  cursor: default;
  font-size: var(--text-xs);
  font-style: italic;
}
.muted {
  color: var(--text-muted);
}
.mono {
  font-family: var(--font-mono);
}
</style>
