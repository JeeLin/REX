<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useEnvironmentsStore } from '@/stores/environments'

const { t } = useI18n()
const router = useRouter()
const store = useEnvironmentsStore()

// ── Props / Emits ──────────────────────────────────────────
const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

// ── State ──────────────────────────────────────────────────
const searchInput = ref('')
const inputRef = ref<HTMLInputElement>()
const selectedIndex = ref(0)

// ── Search Items ───────────────────────────────────────────
interface SearchItem {
  id: string
  title: string
  description: string
  icon: string
  action: () => void
  category: string
}

const searchItems = computed<SearchItem[]>(() => {
  const items: SearchItem[] = []

  // Navigation items
  items.push(
    { id: 'nav-workspace', title: t('nav.workspace'), description: 'Workspace', icon: '🖥️', action: () => router.push('/workspace'), category: 'Navigation' },
    { id: 'nav-dashboard', title: t('nav.dashboard'), description: 'Dashboard', icon: '📊', action: () => router.push('/dashboard'), category: 'Navigation' },
    { id: 'nav-environments', title: t('nav.environments'), description: 'Environments', icon: '📋', action: () => router.push('/environments'), category: 'Navigation' },
    { id: 'nav-audit', title: t('nav.auditLog'), description: 'Audit Log', icon: '📝', action: () => router.push('/audit-log'), category: 'Navigation' },
    { id: 'nav-settings', title: t('nav.settings'), description: 'Settings', icon: '⚙️', action: () => router.push('/settings'), category: 'Navigation' },
  )

  // Environment items
  for (const env of store.environments) {
    items.push({
      id: `env-${env.id}`,
      title: env.name,
      description: env.description || 'Environment',
      icon: env.connection_mode === 'agent' ? '🔗' : '🌐',
      action: () => router.push(`/environments/${env.id}`),
      category: 'Environments',
    })
  }

  // Resource items
  for (const env of store.environments) {
    const resources = store.envResources.get(env.id) || []
    for (const res of resources) {
      items.push({
        id: `res-${res.id}`,
        title: res.name,
        description: `${res.protocol.toUpperCase()} - ${env.name}`,
        icon: res.protocol === 'ssh' ? '💻' : res.protocol === 'mysql' ? '🗄️' : res.protocol === 'redis' ? '📦' : '📁',
        action: () => {
          router.push('/workspace')
          // TODO: Open specific resource in workspace
        },
        category: 'Resources',
      })
    }
  }

  return items
})

// ── Filtered Results ───────────────────────────────────────
const results = computed(() => {
  const q = searchInput.value.toLowerCase()
  if (!q) return searchItems.value.slice(0, 10) // Show top 10 items when empty

  return searchItems.value.filter(item =>
    item.title.toLowerCase().includes(q) ||
    item.description.toLowerCase().includes(q) ||
    item.category.toLowerCase().includes(q)
  ).slice(0, 20)
})

// ── Navigation ─────────────────────────────────────────────
function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectedIndex.value = Math.min(selectedIndex.value + 1, results.value.length - 1)
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    executeItem(results.value[selectedIndex.value])
  } else if (e.key === 'Escape') {
    emit('close')
  }
}

function executeItem(item: SearchItem | undefined) {
  if (item) {
    item.action()
    emit('close')
  }
}

// ── Watchers ───────────────────────────────────────────────
watch(() => props.visible, (val) => {
  if (val) {
    searchInput.value = ''
    selectedIndex.value = 0
    nextTick(() => inputRef.value?.focus())
  }
})

// Reset index when results change
watch(results, () => {
  selectedIndex.value = 0
})
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="visible" class="command-palette-overlay" @click.self="emit('close')">
        <div class="command-palette" @keydown="handleKeydown">
          <div class="command-palette-input-wrap">
            <span class="command-palette-icon">🔍</span>
            <input
              ref="inputRef"
              v-model="searchInput"
              class="command-palette-input"
              :placeholder="t('commandPalette.placeholder', 'Search resources, settings...')"
              autocomplete="off"
              spellcheck="false"
            />
            <kbd class="command-palette-kbd">ESC</kbd>
          </div>

          <div class="command-palette-results" v-if="results.length">
            <div
              v-for="(item, index) in results"
              :key="item.id"
              class="command-palette-item"
              :class="{ 'command-palette-item--selected': index === selectedIndex }"
              @click="executeItem(item)"
              @mouseenter="selectedIndex = index"
            >
              <span class="command-palette-item-icon">{{ item.icon }}</span>
              <div class="command-palette-item-text">
                <span class="command-palette-item-title">{{ item.title }}</span>
                <span class="command-palette-item-desc">{{ item.description }}</span>
              </div>
              <span class="command-palette-item-category">{{ item.category }}</span>
            </div>
          </div>

          <div v-else class="command-palette-empty">
            {{ t('commandPalette.noResults', 'No results found') }}
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.command-palette-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 20vh;
  z-index: 1000;
}

.command-palette {
  width: 560px;
  max-width: 90vw;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-xl);
  overflow: hidden;
}

.command-palette-input-wrap {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}

.command-palette-icon {
  font-size: 16px;
  opacity: 0.5;
}

.command-palette-input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  font-size: 15px;
  color: var(--text-primary);
  font-family: var(--font-sans);
}

.command-palette-input::placeholder {
  color: var(--text-muted);
}

.command-palette-kbd {
  font-size: 11px;
  padding: 2px 6px;
  background: var(--bg-page);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.command-palette-results {
  max-height: 360px;
  overflow-y: auto;
  padding: 8px;
}

.command-palette-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: var(--radius);
  cursor: pointer;
  transition: background 0.1s;
}

.command-palette-item:hover,
.command-palette-item--selected {
  background: var(--bg-hover);
}

.command-palette-item-icon {
  font-size: 16px;
  width: 24px;
  text-align: center;
}

.command-palette-item-text {
  flex: 1;
  min-width: 0;
}

.command-palette-item-title {
  display: block;
  font-size: 14px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.command-palette-item-desc {
  display: block;
  font-size: 12px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.command-palette-item-category {
  font-size: 11px;
  color: var(--text-muted);
  opacity: 0.6;
}

.command-palette-empty {
  padding: 24px;
  text-align: center;
  color: var(--text-muted);
  font-size: 14px;
}

/* Transitions */
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.15s;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
</style>