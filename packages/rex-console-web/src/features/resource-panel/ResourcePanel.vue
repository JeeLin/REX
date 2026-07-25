import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { onClickOutside } from '@vueuse/core'
import { useEnvironmentsStore } from '@/stores/environments'
import type { Resource } from '@/api/resources'
import { PROTOCOL_ICONS, PROTOCOL_COLORS } from '@/features/resource/protocols'
import WizardModal from '@/features/resource/WizardModal.vue'
import { useWorkspaceStore } from '@/stores/workspace'
const emit = defineEmits<{
  resourceProperties: [resource: Resource]
}>()
const { t } = useI18n()
const store = useEnvironmentsStore()
const router = useRouter()
const wsStore = useWorkspaceStore()


function handleResourceClick(res: Resource) {
  wsStore.openResource({
    id: res.id,
    name: res.name,
    protocol: res.protocol,
    host: res.host,
    port: res.port ?? undefined,
    username: res.username,
    environmentId: res.environment_id,
    color: res.color ?? undefined,
  })
  router.push({ name: 'workspace' })
}


const searchQuery = ref('')
const collapsedEnvs = ref(new Set<string>())
const wizardEnvId = ref('')

onMounted(async () => {
  await store.fetchEnvironments()
  for (const env of store.environments) {
    try {
      await store.fetchResources(env.id)
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
      (store.envResources.get(e.id) || []).some(r =>
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
  return store.envResources.get(envId) || []
}

function openWizard(envId: string, e: MouseEvent) {
  e.stopPropagation()
  wizardEnvId.value = envId
}

function onWizardCreated() {
  wizardEnvId.value = ''
}
/* ---- context menu ---- */
const ctxMenu = ref<{ show: boolean; x: number; y: number; resource: Resource | null }>({
  show: false, x: 0, y: 0, resource: null,
})
const ctxMenuRef = ref<HTMLElement | null>(null)
onClickOutside(ctxMenuRef, () => { ctxMenu.value.show = false })

function onContextMenu(e: MouseEvent, res: Resource) {
  e.preventDefault()
  e.stopPropagation()
  ctxMenu.value = { show: true, x: e.clientX, y: e.clientY, resource: res }
}

function ctxOpen() {
  const res = ctxMenu.value.resource
  if (!res) return
  handleResourceClick(res)
  ctxMenu.value.show = false
}

function ctxProperties() {
  const res = ctxMenu.value.resource
  if (!res) return
  ctxMenu.value.show = false
  emit('resourceProperties', res)
}

function ctxDelete() {
  const res = ctxMenu.value.resource
  if (!res) return
  if (!confirm(t('common.confirm') + '?')) return
  ctxMenu.value.show = false
  store.deleteResource(res.environment_id, res.id)
}

</script>

<template>
  <div class="resource-panel">
    <!-- Header -->
    <div class="rp-header">
      <div class="rp-tabs">
        <button class="rp-tab mono rp-tab--active">{{ t('resourcePanel.connections') }}</button>
      </div>
    </div>

    <!-- Search -->
    <div class="rp-search">
      <input
        v-model="searchQuery"
        type="text"
        class="rp-search-input mono"
        :placeholder="t('resourcePanel.searchPlaceholder')"
      />
    </div>

    <!-- Content -->
    <div class="rp-content">
      <div v-if="filteredEnvs.length === 0 && !store.loading" class="rp-empty muted">
        {{ t('resourcePanel.noEnvironments') }}
      </div>

      <template v-for="env in filteredEnvs" :key="env.id">
        <!-- Environment group -->
        <div class="rp-group" @click="toggleEnv(env.id)">
          <span class="rp-chevron" :class="{ 'rp-collapsed': collapsedEnvs.has(env.id) }">▸</span>
          <span class="rp-group-name mono">{{ env.name }}</span>
          <span class="rp-group-count muted">{{ env.resource_count }}</span>
          <button class="rp-add-btn" :title="t('resourcePanel.addResource')" @click="openWizard(env.id, $event)">+</button>
        </div>

        <!-- Resources under this environment -->
        <div v-if="!collapsedEnvs.has(env.id)">
          <div
            v-for="res in getResources(env.id)"
            :key="res.id"
            class="rp-item"
            @click="handleResourceClick(res)"
            @contextmenu.prevent="onContextMenu($event, res)"
          >
            <span class="rp-item-icon" :style="{ color: res.color || PROTOCOL_COLORS[res.protocol] || 'var(--text-secondary)' }">
              {{ PROTOCOL_ICONS[res.protocol] || '?' }}
            </span>
            <span class="rp-item-name">{{ res.name }}</span>
            <span class="rp-item-host mono muted">{{ res.host }}</span>
          </div>
          <div v-if="getResources(env.id).length === 0" class="rp-item rp-empty-item muted">
            {{ t('resourcePanel.noResources') }}
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
    <!-- Resource context menu -->
    <Teleport to="body">
      <div v-if="ctxMenu.show" class="rp-ctx-overlay" @click="ctxMenu.show = false" @contextmenu.prevent="ctxMenu.show = false" />
      <div v-if="ctxMenu.show" ref="ctxMenuRef" class="rp-ctx-menu" :style="{ top: ctxMenu.y + 'px', left: ctxMenu.x + 'px' }">
        <div class="rp-ctx-item" @click="ctxOpen">🚀 {{ t('sidebar.openResource') }}</div>
        <div class="rp-ctx-item" @click="ctxProperties">✏️ {{ t('sidebar.properties') }}</div>
        <div class="rp-ctx-item rp-ctx-item--danger" @click="ctxDelete">🗑 {{ t('sidebar.delete') }}</div>
      </div>
    </Teleport>
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
.rp-ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 999;
}
.rp-ctx-menu {
  position: fixed;
  z-index: 1000;
  min-width: 160px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow-md);
  padding: var(--space-1) 0;
}
.rp-ctx-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-primary);
  cursor: pointer;
  white-space: nowrap;
}
.rp-ctx-item:hover {
  background: var(--bg-hover);
}
.rp-ctx-item--danger {
  color: var(--danger);
}
.rp-ctx-item--danger:hover {
  background: var(--bg-hover);
}
</style>
