<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { getAllShortcuts } from '@/utils/shortcuts'
import { formatShortcut } from '@/utils/platform'

const { t } = useI18n()
const router = useRouter()

// ── 状态 ──────────────────────────────────────────────────
const visible = ref(false)
const query = ref('')
const inputRef = ref<HTMLInputElement>()
const selectedIndex = ref(0)

// ── 搜索结果类型 ──────────────────────────────────────────
interface SearchResult {
  id: string
  type: 'connection' | 'page' | 'command' | 'shortcut'
  label: string
  description?: string
  icon?: string
  action: () => void
}

// ── 模拟数据（实际项目从 stores 获取）──────────────────────
const allItems = computed<SearchResult[]>(() => {
  const items: SearchResult[] = []

  // 页面导航
  const pages = [
    { path: '/workspace/terminal', label: 'SSH Terminal', icon: '💻' },
    { path: '/workspace/sql', label: 'SQL Console', icon: '🗃️' },
    { path: '/workspace/redis', label: 'Redis Console', icon: '🔴' },
    { path: '/workspace/files', label: 'File Manager', icon: '📁' },
    { path: '/agents', label: 'Agent Management', icon: '🤖' },
    { path: '/settings', label: 'Settings', icon: '⚙️' },
  ]
  pages.forEach(p => {
    items.push({
      id: `page:${p.path}`,
      type: 'page',
      label: p.label,
      icon: p.icon,
      action: () => router.push(p.path),
    })
  })

  // 快捷键
  getAllShortcuts().forEach(s => {
    items.push({
      id: `shortcut:${s.id}`,
      type: 'shortcut',
      label: s.label,
      description: formatShortcut(s.keys),
      icon: '⌨️',
      action: s.action,
    })
  })

  return items
})

// ── 模糊匹配 ─────────────────────────────────────────────
interface HighlightSegment {
  text: string
  highlight: boolean
}

/**
 * Fuzzy match: each char of query must appear in order in target.
 * e.g. "sq" matches "SQL Editor" (S…Q…).
 */
function fuzzyMatch(query: string, target: string): boolean {
  if (!query) return true
  const q = query.toLowerCase()
  const t = target.toLowerCase()
  let qi = 0
  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) qi++
  }
  return qi === q.length
}

/** Return the indices in `text` that were matched by the fuzzy query. */
function fuzzyMatchIndices(query: string, text: string): number[] {
  if (!query) return []
  const q = query.toLowerCase()
  const t = text.toLowerCase()
  const indices: number[] = []
  let qi = 0
  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) {
      indices.push(ti)
      qi++
    }
  }
  return indices
}

/** Build segments for template rendering with <mark> highlighting. */
function highlightSegments(query: string, text: string): HighlightSegment[] {
  if (!query) return [{ text, highlight: false }]
  const matched = new Set(fuzzyMatchIndices(query, text))
  const segments: HighlightSegment[] = []
  let buf = ''
  let bufHL = false
  for (let i = 0; i < text.length; i++) {
    const hl = matched.has(i)
    if (hl !== bufHL) {
      if (buf) segments.push({ text: buf, highlight: bufHL })
      buf = text.charAt(i)
      bufHL = hl
    } else {
      buf += text.charAt(i)
    }
  }
  if (buf) segments.push({ text: buf, highlight: bufHL })
  return segments
}

// ── 过滤结果 ──────────────────────────────────────────────
const results = computed(() => {
  if (!query.value.trim()) return allItems.value.slice(0, 10)

  const q = query.value
  return allItems.value.filter(item =>
    fuzzyMatch(q, item.label) ||
    (item.description ? fuzzyMatch(q, item.description) : false)
  ).slice(0, 20)
})
// ── 分组 ──────────────────────────────────────────────────
const groupedResults = computed(() => {
  const groups = new Map<string, SearchResult[]>()
  for (const item of results.value) {
    const group = item.type === 'page' ? 'Pages' :
      item.type === 'shortcut' ? 'Shortcuts' :
      item.type === 'connection' ? 'Connections' : 'Commands'
    if (!groups.has(group)) groups.set(group, [])
    groups.get(group)!.push(item)
  }
  return groups
})

// ── 操作 ──────────────────────────────────────────────────
function open() {
  visible.value = true
  query.value = ''
  selectedIndex.value = 0
  nextTick(() => inputRef.value?.focus())
}

function close() {
  visible.value = false
  query.value = ''
  selectedIndex.value = 0
}

function selectItem(item: SearchResult) {
  item.action()
  close()
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectedIndex.value = Math.min(selectedIndex.value + 1, results.value.length - 1)
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (results.value[selectedIndex.value]) {
      selectItem(results.value[selectedIndex.value]!)
    }
  } else if (e.key === 'Escape') {
    close()
  }
}

// ── 全局快捷键注册 ────────────────────────────────────────
function handleGlobalKeydown(e: KeyboardEvent) {
  const mod = (e.metaKey || e.ctrlKey) && e.key === 'k'
  if (mod) {
    e.preventDefault()
    if (visible.value) {
      close()
    } else {
      open()
    }
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleGlobalKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleGlobalKeydown)
})

// ── 选中项高亮 ────────────────────────────────────────────
watch(query, () => {
  selectedIndex.value = 0
})
</script>

<template>
  <!-- Overlay -->
  <Teleport to="body">
    <Transition name="quick-open">
      <div v-if="visible" class="qo-overlay" @click.self="close">
        <div class="qo-panel">
          <!-- Search Input -->
          <div class="qo-input-wrapper">
            <span class="qo-icon">🔍</span>
            <input
              ref="inputRef"
              v-model="query"
              class="qo-input"
              :placeholder="t('quickOpen.placeholder', 'Search pages, shortcuts, connections...')"
              @keydown="handleKeydown"
            />
            <kbd class="qo-esc">Esc</kbd>
          </div>

          <!-- Results -->
          <div class="qo-results" v-if="results.length > 0">
            <template v-for="[group, items] in groupedResults" :key="group">
              <div class="qo-group-label">{{ group }}</div>
              <div
                v-for="item in items"
                :key="item.id"
                :class="{ selected: results.indexOf(item) === selectedIndex }"
                @click="selectItem(item)"
                @mouseenter="selectedIndex = results.indexOf(item)"
              >
                <span class="qo-item-icon">{{ item.icon || '📄' }}</span>
                <span class="qo-item-label"><template v-for="(seg, i) in highlightSegments(query, item.label)" :key="i"><mark v-if="seg.highlight">{{ seg.text }}</mark><template v-else>{{ seg.text }}</template></template></span>
                <span v-if="item.description" class="qo-item-desc"><template v-for="(seg, i) in highlightSegments(query, item.description ?? '')" :key="i"><mark v-if="seg.highlight">{{ seg.text }}</mark><template v-else>{{ seg.text }}</template></template></span>
              </div>
            </template>
          </div>

          <!-- Empty State -->
          <div v-else class="qo-empty">
            {{ t('quickOpen.noResults', 'No results found') }}
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.qo-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  z-index: 9999;
  display: flex;
  justify-content: center;
  padding-top: 20vh;
}

.qo-panel {
  width: 560px;
  max-height: 400px;
  background: var(--color-bg-elevated, #1e1e2e);
  border: 1px solid var(--color-border, #333);
  border-radius: 12px;
  box-shadow: 0 16px 64px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.qo-input-wrapper {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border, #333);
  gap: 8px;
}

.qo-icon {
  font-size: 18px;
  opacity: 0.6;
}

.qo-input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  color: var(--color-text, #e0e0e0);
  font-size: 16px;
  font-family: inherit;
}

.qo-input::placeholder {
  color: var(--color-text-muted, #888);
}

.qo-esc {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--color-bg-muted, #333);
  color: var(--color-text-muted, #888);
  font-family: monospace;
}

.qo-results {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.qo-group-label {
  padding: 4px 16px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--color-text-muted, #888);
}

.qo-item {
  display: flex;
  align-items: center;
  padding: 8px 16px;
  gap: 10px;
  cursor: pointer;
  transition: background 0.1s;
}

.qo-item:hover,
.qo-item.selected {
  background: var(--color-bg-hover, #2a2a3a);
}

.qo-item-icon {
  font-size: 16px;
  width: 24px;
  text-align: center;
}

.qo-item-label {
  flex: 1;
  font-size: 14px;
  color: var(--color-text, #e0e0e0);
}

.qo-item-desc {
  font-size: 12px;
  color: var(--color-text-muted, #888);
  font-family: monospace;
}

.qo-item-label mark,
.qo-item-desc mark {
  background: rgba(232, 145, 45, 0.3);
  color: inherit;
  border-radius: 2px;
  padding: 0 1px;
}

.qo-empty {
  padding: 24px 16px;
  text-align: center;
  color: var(--color-text-muted, #888);
  font-size: 14px;
}

/* Transition */
.quick-open-enter-active,
.quick-open-leave-active {
  transition: opacity 0.15s ease;
}

.quick-open-enter-active .qo-panel,
.quick-open-leave-active .qo-panel {
  transition: transform 0.15s ease, opacity 0.15s ease;
}

.quick-open-enter-from,
.quick-open-leave-to {
  opacity: 0;
}

.quick-open-enter-from .qo-panel {
  transform: scale(0.95) translateY(-10px);
  opacity: 0;
}
</style>
