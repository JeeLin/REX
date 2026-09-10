<script setup lang="ts">
import { ref, computed, watch, nextTick, onBeforeUnmount } from 'vue'

// ── Types ──────────────────────────────────────────────────
export interface SearchTab {
  id: number
  title: string
  sql?: string  // DesignerTabs don't have sql
}

interface MatchLine {
  lineNumber: number
  lineText: string
}

interface TabSearchResult {
  tabId: number
  tabTitle: string
  matches: MatchLine[]
}

// ── Props / Emits ──────────────────────────────────────────
const props = defineProps<{
  visible: boolean
  tabs: SearchTab[]
}>()

const emit = defineEmits<{
  close: []
  'navigate': [tabId: number, lineNumber: number]
}>()

// ── State ──────────────────────────────────────────────────
const searchInput = ref('')
const inputRef = ref<HTMLInputElement>()
const selectedIndex = ref(0)

// ── Search Logic ───────────────────────────────────────────
const results = computed<TabSearchResult[]>(() => {
  const q = searchInput.value.toLowerCase()
  if (!q) return []

  const allResults: TabSearchResult[] = []
  for (const tab of props.tabs) {
    if (!tab || !tab.sql) continue
    const lines = tab.sql.split('\n')
    const matches: MatchLine[] = []
    for (let i = 0; i < lines.length; i++) {
      if (lines[i]!.toLowerCase().includes(q)) {
        matches.push({ lineNumber: i + 1, lineText: lines[i]! })
      }
    }
    if (matches.length > 0) {
      allResults.push({ tabId: tab.id, tabTitle: tab.title, matches })
    }
  }
  return allResults
})

const flatResults = computed(() => {
  const flat: { tabId: number; tabTitle: string; lineNumber: number; lineText: string }[] = []
  for (const r of results.value) {
    for (const m of r.matches) {
      flat.push({ tabId: r.tabId, tabTitle: r.tabTitle, lineNumber: m.lineNumber, lineText: m.lineText })
    }
  }
  return flat
})

const totalCount = computed(() => flatResults.value.length)

// ── Highlight matched text ─────────────────────────────────
function highlightText(text: string, query: string): string {
  if (!query) return escapeHtml(text)
  const escaped = escapeHtml(text)
  const qEscaped = escapeHtml(query)
  // Case-insensitive replace with <mark>
  const regex = new RegExp(`(${qEscaped.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi')
  return escaped.replace(regex, '<mark>$1</mark>')
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

// ── Keyboard navigation ────────────────────────────────────
function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    emit('close')
    return
  }
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectedIndex.value = Math.min(selectedIndex.value + 1, flatResults.value.length - 1)
    scrollToSelected()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0)
    scrollToSelected()
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (flatResults.value[selectedIndex.value]) {
      const r = flatResults.value[selectedIndex.value]!
      emit('navigate', r.tabId, r.lineNumber)
      emit('close')
    }
  }
}

function scrollToSelected() {
  nextTick(() => {
    const el = document.querySelector('.gs-modal-result-item.selected')
    el?.scrollIntoView({ block: 'nearest' })
  })
}

// ── Click result ───────────────────────────────────────────
function navigateTo(tabId: number, lineNumber: number) {
  emit('navigate', tabId, lineNumber)
  emit('close')
}

// ── Reset on open/close ────────────────────────────────────
watch(() => props.visible, (v) => {
  if (v) {
    searchInput.value = ''
    selectedIndex.value = 0
    nextTick(() => inputRef.value?.focus())
  }
})

// Reset selection on query change
watch(searchInput, () => {
  selectedIndex.value = 0
})

// ── Global Escape listener (for overlay backdrop) ───────────
function handleGlobalKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.visible) {
    e.preventDefault()
    emit('close')
  }
}

watch(() => props.visible, (v) => {
  if (v) {
    document.addEventListener('keydown', handleGlobalKeydown, true)
  } else {
    document.removeEventListener('keydown', handleGlobalKeydown, true)
  }
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleGlobalKeydown, true)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="gs-modal">
      <div v-if="visible" class="gs-modal-overlay" @click.self="$emit('close')">
        <div class="gs-modal-panel" @keydown="handleKeydown">
          <!-- Header -->
          <div class="gs-modal-header">
            <span class="gs-modal-icon">🔍</span>
            <input
              ref="inputRef"
              v-model="searchInput"
              class="gs-modal-input"
              placeholder="Search across all SQL tabs…"
              spellcheck="false"
            />
            <span v-if="searchInput" class="gs-modal-count">
              {{ totalCount }} {{ totalCount === 1 ? 'result' : 'results' }}
            </span>
            <kbd class="gs-modal-esc">Esc</kbd>
          </div>

          <!-- Results -->
          <div class="gs-modal-body" v-if="results.length > 0">
            <div v-for="group in results" :key="group.tabId" class="gs-modal-group">
              <div class="gs-modal-group-header">
                <span class="gs-modal-group-icon">📄</span>
                <span class="gs-modal-group-title">{{ group.tabTitle }}</span>
                <span class="gs-modal-group-count">{{ group.matches.length }}</span>
              </div>
              <div
                v-for="match in group.matches"
                :key="`${group.tabId}:${match.lineNumber}`"
                class="gs-modal-result-item"
                :class="{ selected: flatResults.findIndex(r => r.tabId === group.tabId && r.lineNumber === match.lineNumber) === selectedIndex }"
                @click="navigateTo(group.tabId, match.lineNumber)"
              >
                <span class="gs-modal-line-num">{{ match.lineNumber }}</span>
                <span class="gs-modal-line-text" v-html="highlightText(match.lineText, searchInput)" />
              </div>
            </div>
          </div>

          <!-- Empty state -->
          <div v-else-if="searchInput" class="gs-modal-empty">
            <div class="gs-modal-empty-icon">🔎</div>
            <div>No matches found</div>
          </div>

          <!-- Hint when no query -->
          <div v-else class="gs-modal-empty">
            <div class="gs-modal-empty-icon">⌨️</div>
            <div>Type to search across all open SQL tabs</div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.gs-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(4px);
  z-index: 9999;
  display: flex;
  justify-content: center;
  padding-top: 12vh;
}

.gs-modal-panel {
  width: 640px;
  max-height: 480px;
  background: var(--color-bg-elevated, #1e1e2e);
  border: 1px solid var(--color-border, #333);
  border-radius: 12px;
  box-shadow: 0 16px 64px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* ---- Header / search input ---- */
.gs-modal-header {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border, #333);
  gap: 8px;
}

.gs-modal-icon {
  font-size: 18px;
  opacity: 0.6;
}

.gs-modal-input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  color: var(--color-text, #e0e0e0);
  font-size: 16px;
  font-family: inherit;
}

.gs-modal-input::placeholder {
  color: var(--color-text-muted, #888);
}

.gs-modal-count {
  font-size: 12px;
  color: var(--color-text-muted, #888);
  white-space: nowrap;
}

.gs-modal-esc {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--color-bg-muted, #333);
  color: var(--color-text-muted, #888);
  font-family: monospace;
}

/* ---- Body / results ---- */
.gs-modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.gs-modal-group {
  margin-bottom: 4px;
}

.gs-modal-group-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 16px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--color-text-muted, #888);
}

.gs-modal-group-icon {
  font-size: 12px;
}

.gs-modal-group-title {
  flex: 1;
}

.gs-modal-group-count {
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 8px;
  background: var(--color-bg-muted, #333);
  color: var(--color-text-muted, #888);
}

/* ---- Result items ---- */
.gs-modal-result-item {
  display: flex;
  align-items: flex-start;
  padding: 4px 16px;
  gap: 12px;
  cursor: pointer;
  transition: background 0.1s;
  font-family: monospace;
  font-size: 13px;
}

.gs-modal-result-item:hover,
.gs-modal-result-item.selected {
  background: var(--color-bg-hover, #2a2a3a);
}

.gs-modal-line-num {
  min-width: 32px;
  text-align: right;
  color: var(--color-text-muted, #666);
  user-select: none;
  flex-shrink: 0;
}

.gs-modal-line-text {
  flex: 1;
  white-space: pre;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--color-text, #e0e0e0);
}

.gs-modal-result-item :deep(mark) {
  background: rgba(232, 145, 45, 0.35);
  color: inherit;
  border-radius: 2px;
  padding: 0 1px;
}

/* ---- Empty state ---- */
.gs-modal-empty {
  padding: 32px 16px;
  text-align: center;
  color: var(--color-text-muted, #888);
  font-size: 14px;
}

.gs-modal-empty-icon {
  font-size: 28px;
  margin-bottom: 8px;
  opacity: 0.5;
}

/* ---- Transition ---- */
.gs-modal-enter-active,
.gs-modal-leave-active {
  transition: opacity 0.15s ease;
}

.gs-modal-enter-active .gs-modal-panel,
.gs-modal-leave-active .gs-modal-panel {
  transition: transform 0.15s ease, opacity 0.15s ease;
}

.gs-modal-enter-from,
.gs-modal-leave-to {
  opacity: 0;
}

.gs-modal-enter-from .gs-modal-panel {
  transform: scale(0.95) translateY(-10px);
  opacity: 0;
}
</style>
