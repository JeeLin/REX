<script setup lang="ts">
import { ref, watch } from 'vue'
import { SearchAddon } from '@xterm/addon-search'

const props = defineProps<{
  visible: boolean
  searchAddon: SearchAddon | null
}>()

const emit = defineEmits<{
  close: []
}>()

const searchInput = ref('')
const caseSensitive = ref(false)
const wholeWord = ref(false)
const regex = ref(false)

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
  emit('close')
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    closeSearch()
  } else if (e.key === 'Enter') {
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
  }
})

watch(() => props.visible, (v) => {
  if (v) {
    // 自动聚焦搜索输入框
    setTimeout(() => {
      const input = document.querySelector('.ts-input') as HTMLInputElement
      input?.focus()
      input?.select()
    }, 50)
  }
})
</script>

<template>
  <Transition name="search">
    <div v-if="visible" class="terminal-search" @keydown="onKeydown">
      <input
        ref="searchInputRef"
        v-model="searchInput"
        class="ts-input mono"
        placeholder="Find..."
        autofocus
      />
      <div class="ts-actions">
        <button class="ts-btn" title="Previous (Shift+Enter)" @click="findPrev">↑</button>
        <button class="ts-btn" title="Next (Enter)" @click="findNext">↓</button>
        <button
          class="ts-btn"
          :class="{ 'ts-btn--active': caseSensitive }"
          title="Case Sensitive"
          @click="caseSensitive = !caseSensitive"
        >
          Aa
        </button>
        <button
          class="ts-btn"
          :class="{ 'ts-btn--active': wholeWord }"
          title="Whole Word"
          @click="wholeWord = !wholeWord"
        >
          W
        </button>
        <button
          class="ts-btn"
          :class="{ 'ts-btn--active': regex }"
          title="Regex"
          @click="regex = !regex"
        >
          .*
        </button>
        <button class="ts-btn ts-close" title="Close (Esc)" @click="closeSearch">×</button>
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
