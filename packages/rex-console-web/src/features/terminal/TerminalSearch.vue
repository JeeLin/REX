<script setup lang="ts">
import { ref, computed, watch, nextTick, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SearchAddon } from '@xterm/addon-search'

const props = defineProps<{
  visible: boolean
  searchAddon: SearchAddon | null
}>()

const emit = defineEmits<{
  close: []
}>()

const { t } = useI18n()

const searchInputRef = ref<HTMLInputElement | null>(null)
const searchInput = ref('')
const caseSensitive = ref(false)
const wholeWord = ref(false)
const regex = ref(false)

// ── 匹配计数 ────────────────────────────────────────────────────────
// SearchAddon 的 decorations 默认开启（highlights all instances），因此
// onDidChangeResults 会随搜索结果变化触发，可直接拿来做「第 n/N 个」提示。
const resultCount = ref(0)
const resultIndex = ref(-1)

const matchLabel = computed(() => {
  if (!searchInput.value) return ''
  if (resultCount.value === 0) return t('terminal.search.noResult')
  // resultIndex 为 -1 表示匹配数超过 highlightLimit（1000），无法定位当前项
  if (resultIndex.value < 0) return `${resultCount.value}+`
  return `${resultIndex.value + 1}/${resultCount.value}`
})

function resetResults() {
  resultIndex.value = -1
  resultCount.value = 0
}

function doSearch(forward = true) {
  if (!props.searchAddon || !searchInput.value) return
  const opts = {
    caseSensitive: caseSensitive.value,
    wholeWord: wholeWord.value,
    regex: regex.value,
  }
  if (forward) {
    props.searchAddon.findNext(searchInput.value, opts)
  } else {
    props.searchAddon.findPrevious(searchInput.value, opts)
  }
}

function findNext() {
  doSearch(true)
}

function findPrev() {
  doSearch(false)
}

function closeSearch() {
  props.searchAddon?.clearDecorations()
  searchInput.value = ''
  resetResults()
  emit('close')
}

// 键盘事件绑在 input 上：input 获焦时事件冒泡到外层容器才拿得到，
// 绑在外层 div 上依赖焦点恰好落在子树内，不可靠。
function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    closeSearch()
  } else if (e.key === 'Enter') {
    // 终端里回车是发送命令，这里必须吃掉，否则会穿透到 PTY
    e.preventDefault()
    if (e.shiftKey) {
      findPrev()
    } else {
      findNext()
    }
  }
}

watch(searchInput, () => {
  if (searchInput.value) {
    doSearch()
  } else {
    props.searchAddon?.clearDecorations()
    resetResults()
  }
})

// 选项变化后用当前词重搜一次，保证高亮与开关状态一致
watch([caseSensitive, wholeWord, regex], () => {
  if (searchInput.value) doSearch()
})

watch(
  () => props.visible,
  async (v) => {
    if (!v) return
    // nextTick 等 DOM 落地后聚焦「本组件自己的」input。
    // 旧实现是 setTimeout(50ms) + document.querySelector('.ts-input')：
    // WorkspaceTerminal 经 PaneNode → PaneLeaf 可多开同时挂载，
    // 全局选择器会命中 DOM 里第一个搜索框，焦点跳到别的窗格；
    // 首次打开时若尚无 .ts-input，则静默不聚焦。
    await nextTick()
    searchInputRef.value?.focus()
    searchInputRef.value?.select()
  }
)

let disposeResults: { dispose(): void } | null = null

watch(
  () => props.searchAddon,
  (addon) => {
    disposeResults?.dispose()
    disposeResults = null
    if (addon && typeof addon.onDidChangeResults === 'function') {
      disposeResults = addon.onDidChangeResults((e) => {
        resultIndex.value = e.resultIndex
        resultCount.value = e.resultCount
      })
    }
    // 打桩/旧版 addon 可能没有 onDidChangeResults：无计数可用，搜索跳转
    // （findNext/findPrevious）仍可正常工作。
  },
  { immediate: true }
)

onBeforeUnmount(() => {
  disposeResults?.dispose()
  disposeResults = null
})
</script>

<template>
  <Transition name="search">
    <div v-if="visible" class="terminal-search">
      <input
        ref="searchInputRef"
        v-model="searchInput"
        class="ts-input mono"
        :placeholder="t('terminal.search.placeholder')"
        @keydown="onKeydown"
      />
      <span v-if="matchLabel" class="ts-count">{{ matchLabel }}</span>
      <div class="ts-actions">
        <button class="ts-btn" :title="t('terminal.search.prev')" @click="findPrev">↑</button>
        <button class="ts-btn" :title="t('terminal.search.next')" @click="findNext">↓</button>
        <button
          class="ts-btn"
          :class="{ 'ts-btn--active': caseSensitive }"
          :title="t('terminal.search.caseSensitive')"
          @click="caseSensitive = !caseSensitive"
        >
          Aa
        </button>
        <button
          class="ts-btn"
          :class="{ 'ts-btn--active': wholeWord }"
          :title="t('terminal.search.wholeWord')"
          @click="wholeWord = !wholeWord"
        >
          W
        </button>
        <button
          class="ts-btn"
          :class="{ 'ts-btn--active': regex }"
          :title="t('terminal.search.regex')"
          @click="regex = !regex"
        >
          .*
        </button>
        <button class="ts-btn ts-close" :title="t('terminal.search.close')" @click="closeSearch">
          ×
        </button>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.terminal-search {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-2);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-top: none;
  border-radius: 0 0 var(--radius) var(--radius);
  box-shadow: var(--shadow);
  margin: 0 var(--space-3);
}

.ts-input {
  width: 180px;
  padding: var(--space-1) var(--space-2);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  outline: none;
}

.ts-input:focus {
  border-color: var(--accent);
}

.ts-count {
  min-width: 48px;
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-family: var(--font-mono);
  text-align: center;
  white-space: nowrap;
}

.ts-actions {
  display: flex;
  gap: 2px;
}

.ts-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  background: none;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-family: var(--font-mono);
  cursor: pointer;
  transition: color var(--transition), background var(--transition);
}

.ts-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.ts-btn--active {
  color: var(--accent);
  background: rgba(232, 145, 45, 0.15);
}

.ts-close {
  margin-left: var(--space-1);
}

.search-enter-active,
.search-leave-active {
  transition: opacity var(--transition-fast), transform var(--transition-fast);
}

.search-enter-from,
.search-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
