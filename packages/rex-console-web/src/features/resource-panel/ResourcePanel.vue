<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { onClickOutside } from '@vueuse/core'
import { useEnvironmentsStore } from '@/stores/environments'
import type { Resource } from '@/api/resources'
import { PROTOCOL_ICONS, PROTOCOL_COLORS } from '@/features/resource/protocols'
import WizardModal from '@/features/resource/WizardModal.vue'
import { useWorkspaceStore } from '@/stores/workspace'
import { useFavoritesStore } from '@/stores/favorites'

const emit = defineEmits<{
  resourceProperties: [resource: Resource]
}>()
const { t } = useI18n()
const store = useEnvironmentsStore()
const router = useRouter()
const wsStore = useWorkspaceStore()
const favStore = useFavoritesStore()

/* ---- types ---- */
type TabKey = 'connections' | 'favorites' | 'recent'

/* ---- tabs ---- */
const activeTab = ref<TabKey>('connections')

/* ---- global search ---- */
const globalSearch = ref('')
const globalSearchInput = ref<HTMLInputElement | null>(null)
const showGlobalResults = ref(false)
let debounceTimer: ReturnType<typeof setTimeout> | null = null

watch(globalSearch, (val) => {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    showGlobalResults.value = val.trim().length > 0
  }, 200)
})

function clearGlobalSearch() {
  globalSearch.value = ''
  showGlobalResults.value = false
}

function onGlobalSearchKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    clearGlobalSearch()
    globalSearchInput.value?.blur()
  } else if (e.key === 'Enter') {
    const first = globalSearchResults.value[0]?.resource
    if (first) handleResourceClick(first)
  }
}

const globalSearchResults = computed(() => {
  const q = globalSearch.value.trim().toLowerCase()
  if (!q) return []
  const results: Array<{ envName: string; resource: Resource; nameHtml: string; descHtml: string }> = []
  for (const env of store.environments) {
    const resources = store.envResources.get(env.id) || []
    for (const r of resources) {
      const nameMatch = r.name.toLowerCase().includes(q)
      if (nameMatch) {
        results.push({
          envName: env.name,
          resource: r,
          nameHtml: highlightMatch(r.name, q),
          descHtml: '',
        })
      }
    }
  }
  return results
})

const globalSearchResultsByEnv = computed(() => {
  const grouped = new Map<string, Array<{ resource: Resource; nameHtml: string }>>()
  for (const item of globalSearchResults.value) {
    const existing = grouped.get(item.envName) || []
    existing.push({ resource: item.resource, nameHtml: item.nameHtml })
    grouped.set(item.envName, existing)
  }
  return grouped
})

function highlightMatch(text: string, query: string): string {
  if (!query) return escapeHtml(text)
  const idx = text.toLowerCase().indexOf(query)
  if (idx < 0) return escapeHtml(text)
  const before = text.slice(0, idx)
  const match = text.slice(idx, idx + query.length)
  const after = text.slice(idx + query.length)
  return `${escapeHtml(before)}<mark>${escapeHtml(match)}</mark>${escapeHtml(after)}`
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

/* ---- resource click (shared) ---- */
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
    subtype: res.subtype ?? undefined,
  })
  favStore.addRecent({ id: res.id, name: res.name, protocol: res.protocol })
  router.push({ name: 'workspace' })
}

function openRecentItem(item: { id: string; name: string; protocol: string }) {
  /* Find the full resource from store by id */
  for (const env of store.environments) {
    const resources = store.envResources.get(env.id) || []
    const match = resources.find(r => r.id === item.id)
    if (match) {
      handleResourceClick(match)
      return
    }
  }
}

/* ---- connections tab (existing logic) ---- */
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

// Listen for env changes from other pages
import { onBeforeUnmount } from 'vue'

async function onEnvChanged() {
  await store.fetchEnvironments()
  for (const env of store.environments) {
    try {
      await store.fetchResources(env.id)
    } catch {
      // ignore
    }
  }
}

onMounted(() => {
  window.addEventListener('rex:env-changed', onEnvChanged)
})
onBeforeUnmount(() => {
  window.removeEventListener('rex:env-changed', onEnvChanged)
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

/* ---- favorites tab ---- */
const favoriteResources = computed(() => {
  const allResources: Resource[] = []
  for (const env of store.environments) {
    const resources = store.envResources.get(env.id) || []
    for (const r of resources) {
      if (favStore.isFavorite(r.id)) {
        allResources.push(r)
      }
    }
  }
  return allResources
})

function relativeTime(ts: number): string {
  const diff = Date.now() - ts
  const sec = Math.floor(diff / 1000)
  if (sec < 60) return `${sec}s`
  const min = Math.floor(sec / 60)
  if (min < 60) return `${min}m`
  const hr = Math.floor(min / 60)
  if (hr < 24) return `${hr}h`
  const d = Math.floor(hr / 24)
  return `${d}d`
}

/* ---- recent tab ---- */
const recentItems = computed(() => favStore.recent)

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

function ctxToggleFavorite() {
  const res = ctxMenu.value.resource
  if (!res) return
  favStore.toggleFavorite(res.id)
  ctxMenu.value.show = false
}
</script>

<template>
  <div class="resource-panel">
    <!-- Global Search -->
    <div class="rp-search">
      <div class="rp-search-wrap">
        <span class="rp-search-icon">🔍</span>
        <input
          ref="globalSearchInput"
          v-model="globalSearch"
          type="text"
          class="rp-search-input mono"
          :placeholder="t('sidebar.search')"
          @keydown="onGlobalSearchKeydown"
        />
      </div>
    </div>

    <!-- Tabs (hidden when search active) -->
    <div v-if="!showGlobalResults" class="rp-header">
      <div class="rp-tabs">
        <button
          class="rp-tab mono"
          :class="{ 'rp-tab--active': activeTab === 'connections' }"
          @click="activeTab = 'connections'"
        >
          🔗 {{ t('sidebar.connections') }}
        </button>
        <button
          class="rp-tab mono"
          :class="{ 'rp-tab--active': activeTab === 'favorites' }"
          @click="activeTab = 'favorites'"
        >
          ⭐ {{ t('sidebar.favorites') }}
        </button>
        <button
          class="rp-tab mono"
          :class="{ 'rp-tab--active': activeTab === 'recent' }"
          @click="activeTab = 'recent'"
        >
          🕐 {{ t('sidebar.recent') }}
        </button>
      </div>
    </div>

    <!-- Global Search Results -->
    <div v-if="showGlobalResults" class="rp-content">
      <div v-if="globalSearchResults.length === 0" class="rp-empty muted">
        {{ t('sidebar.noMatch') }}
      </div>
      <template v-for="[envName, items] in globalSearchResultsByEnv" :key="envName">
        <div class="rp-group rp-group--static">
          <span class="rp-group-name mono">{{ envName }}</span>
        </div>
        <div
          v-for="item in items"
          :key="item.resource.id"
          class="rp-item"
          @click="handleResourceClick(item.resource)"
        >
          <span class="rp-item-icon" :style="{ color: item.resource.color || PROTOCOL_COLORS[item.resource.protocol] || 'var(--text-secondary)' }">
            {{ PROTOCOL_ICONS[item.resource.protocol] || '?' }}
          </span>
          <span class="rp-item-name" v-html="item.nameHtml"></span>
          <span class="rp-item-protocol mono muted">{{ item.resource.protocol }}</span>
        </div>
      </template>
    </div>

    <!-- Connections Tab -->
    <div v-else-if="activeTab === 'connections'" class="rp-content">
      <!-- Local search within connections -->
      <div class="rp-inner-search">
        <input
          v-model="searchQuery"
          type="text"
          class="rp-search-input mono"
          :placeholder="t('resourcePanel.searchPlaceholder')"
        />
      </div>

      <div v-if="filteredEnvs.length === 0 && !store.loading" class="rp-empty muted">
        {{ t('resourcePanel.noEnvironments') }}
      </div>

      <template v-for="env in filteredEnvs" :key="env.id">
        <div class="rp-group" @click="toggleEnv(env.id)">
          <span class="rp-chevron" :class="{ 'rp-collapsed': collapsedEnvs.has(env.id) }">▸</span>
          <span class="rp-group-name mono">{{ env.name }}</span>
          <span class="rp-group-count muted">{{ env.resource_count }}</span>
          <button class="rp-add-btn" :title="t('resourcePanel.addResource')" @click="openWizard(env.id, $event)">+</button>
        </div>

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
            <button
              class="rp-star-btn"
              :class="{ 'rp-star--active': favStore.isFavorite(res.id) }"
              :title="favStore.isFavorite(res.id) ? t('sidebar.unfavorite') : t('sidebar.favorites')"
              @click.stop="favStore.toggleFavorite(res.id)"
            >
              {{ favStore.isFavorite(res.id) ? '★' : '☆' }}
            </button>
          </div>
          <div v-if="getResources(env.id).length === 0" class="rp-item rp-empty-item muted">
            {{ t('resourcePanel.noResources') }}
          </div>
        </div>
      </template>
    </div>

    <!-- Favorites Tab -->
    <div v-else-if="activeTab === 'favorites'" class="rp-content">
      <div v-if="favoriteResources.length === 0" class="rp-empty muted">
        {{ t('sidebar.noFavorites') }}
      </div>
      <div
        v-for="res in favoriteResources"
        :key="res.id"
        class="rp-item"
        @click="handleResourceClick(res)"
        @contextmenu.prevent="onContextMenu($event, res)"
      >
        <span class="rp-item-icon" :style="{ color: res.color || PROTOCOL_COLORS[res.protocol] || 'var(--text-secondary)' }">
          {{ PROTOCOL_ICONS[res.protocol] || '?' }}
        </span>
        <span class="rp-item-name">{{ res.name }}</span>
        <span class="rp-item-protocol mono muted">{{ res.protocol }}</span>
        <button
          class="rp-star-btn rp-star--active"
          :title="t('sidebar.favorites')"
          @click.stop="favStore.toggleFavorite(res.id)"
        >
          ★
        </button>
      </div>
    </div>

    <!-- Recent Tab -->
    <div v-else-if="activeTab === 'recent'" class="rp-content">
      <div v-if="recentItems.length === 0" class="rp-empty muted">
        {{ t('sidebar.noRecent') }}
      </div>
      <div
        v-for="item in recentItems"
        :key="item.id"
        class="rp-item"
        @click="openRecentItem(item)"
      >
        <span class="rp-item-icon" :style="{ color: PROTOCOL_COLORS[item.protocol] || 'var(--text-secondary)' }">
          {{ PROTOCOL_ICONS[item.protocol] || '?' }}
        </span>
        <span class="rp-item-name">{{ item.name }}</span>
        <span class="rp-item-time mono muted">{{ relativeTime(item.time) }}</span>
        <span class="rp-item-protocol mono muted">{{ item.protocol }}</span>
      </div>
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
        <div class="rp-ctx-item" @click="ctxOpen"><span class="rp-ctx-icon">🚀</span> {{ t('sidebar.openResource') }}</div>
        <div class="rp-ctx-item" @click="ctxToggleFavorite">
          <span class="rp-ctx-icon">{{ ctxMenu.resource && favStore.isFavorite(ctxMenu.resource.id) ? '★' : '☆' }}</span>
          {{ ctxMenu.resource && favStore.isFavorite(ctxMenu.resource.id) ? t('sidebar.unfavorite') : t('sidebar.favorites') }}
        </div>
        <div class="rp-ctx-item" @click="ctxProperties"><span class="rp-ctx-icon">✏️</span> {{ t('sidebar.properties') }}</div>
        <div class="rp-ctx-item rp-ctx-item--danger" @click="ctxDelete"><span class="rp-ctx-icon">🗑</span> {{ t('sidebar.delete') }}</div>
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
  flex: 1;
}
.rp-tab {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-muted);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  white-space: nowrap;
}
.rp-tab:hover {
  color: var(--text-secondary);
}
.rp-tab--active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}
.rp-search {
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
}
.rp-search-wrap {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}
.rp-search-icon {
  font-size: 12px;
  flex-shrink: 0;
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
.rp-inner-search {
  padding: var(--space-1) var(--space-3);
  border-bottom: 1px solid var(--border);
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
.rp-group--static {
  cursor: default;
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
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.rp-item-name :deep(mark) {
  background: var(--accent-subtle, rgba(88, 166, 255, 0.25));
  color: inherit;
  padding: 0 1px;
  border-radius: 2px;
}
.rp-item-host {
  font-size: var(--text-xs);
  margin-left: auto;
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.rp-item-protocol {
  font-size: var(--text-xs);
}
.rp-item-time {
  font-size: var(--text-xs);
}
.rp-star-btn {
  background: none;
  border: none;
  font-size: 14px;
  cursor: pointer;
  padding: 0;
  line-height: 1;
  color: var(--text-muted);
  opacity: 0;
  transition: opacity var(--transition), color var(--transition);
  flex-shrink: 0;
}
.rp-item:hover .rp-star-btn,
.rp-star--active {
  opacity: 1;
}
.rp-star--active {
  color: var(--accent);
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
.rp-ctx-item > span:first-child,
.rp-ctx-item > em:first-child {
  font-size: 14px;
  width: 18px;
  text-align: center;
  flex-shrink: 0;
  font-style: normal;
}
.rp-ctx-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  font-size: 14px;
  flex-shrink: 0;
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
