<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="slash-menu-overlay"
      @click="$emit('close')"
    />
  </Teleport>
  <div
    v-if="visible"
    ref="menuEl"
    class="slash-menu"
    :style="menuStyle"
  >
    <div class="slash-menu-header">{{ t('notebooks.editor.slashMenu.title') }}</div>
    <button
      v-for="(item, index) in filteredItems"
      :key="item.type"
      class="slash-menu-item"
      :class="{ active: index === activeIndex }"
      @click="$emit('select', item.type)"
      @mouseenter="activeIndex = index"
    >
      <span class="slash-item-icon">{{ item.icon }}</span>
      <div class="slash-item-text">
        <span class="slash-item-name">{{ item.label }}</span>
        <span class="slash-item-desc">{{ item.description }}</span>
      </div>
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  visible: boolean
  position: { top: number; left: number }
  filter: string
}>()

const emit = defineEmits<{
  select: [type: string]
  close: []
}>()

const { t } = useI18n()
const menuEl = ref<HTMLElement>()
const activeIndex = ref(0)

const items = computed(() => [
  {
    type: 'paragraph',
    label: t('notebooks.editor.blockType.paragraph'),
    description: t('notebooks.editor.blockType.paragraphDesc'),
    icon: '¶',
  },
  {
    type: 'heading',
    label: t('notebooks.editor.blockType.heading'),
    description: t('notebooks.editor.blockType.headingDesc'),
    icon: 'H',
  },
  {
    type: 'code',
    label: t('notebooks.editor.blockType.code'),
    description: t('notebooks.editor.blockType.codeDesc'),
    icon: '⟨⟩',
  },
  {
    type: 'command',
    label: t('notebooks.editor.blockType.command'),
    description: t('notebooks.editor.blockType.commandDesc'),
    icon: '⚡',
  },
])

const filteredItems = computed(() => {
  const q = props.filter.toLowerCase()
  return items.value.filter(
    (item) =>
      item.type.includes(q) ||
      item.label.toLowerCase().includes(q)
  )
})

const menuStyle = computed(() => ({
  top: `${props.position.top}px`,
  left: `${props.position.left}px`,
}))

watch(
  () => props.visible,
  (val) => {
    if (val) {
      activeIndex.value = 0
    }
  }
)

watch(activeIndex, () => {
  if (!menuEl.value) return
  const active = menuEl.value.querySelector('.slash-menu-item.active')
  active?.scrollIntoView({ block: 'nearest' })
})

function onKeydown(e: KeyboardEvent) {
  const count = filteredItems.value.length
  if (count === 0) return

  if (e.key === 'ArrowDown') {
    e.preventDefault()
    activeIndex.value = (activeIndex.value + 1) % count
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    activeIndex.value = (activeIndex.value - 1 + count) % count
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (activeIndex.value >= 0 && activeIndex.value < count) {
      const item = filteredItems.value[activeIndex.value]
      if (item) {
        emit('select', item.type)
      }
    }
  } else if (e.key === 'Escape') {
    emit('close')
  }
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown)
})
</script>

<style scoped>
.slash-menu-overlay {
  position: fixed;
  inset: 0;
  z-index: 999;
}

.slash-menu {
  position: fixed;
  z-index: 1000;
  min-width: 220px;
  max-width: 320px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  padding: var(--sp-xs) 0;
  font-size: var(--fs-sm);
  user-select: none;
}

.slash-menu-header {
  padding: var(--sp-sm) var(--sp-lg);
  color: var(--text-muted);
  font-size: var(--fs-xs);
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-bottom: 1px solid var(--border);
  margin-bottom: var(--sp-xs);
}

.slash-menu-item {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
  width: 100%;
  padding: var(--sp-sm) var(--sp-lg);
  border: none;
  background: none;
  color: var(--text-primary);
  cursor: pointer;
  transition: background var(--transition-fast);
  text-align: left;
  font-family: var(--font-body);
  font-size: var(--fs-sm);
}

.slash-menu-item:hover,
.slash-menu-item.active {
  background: var(--bg-hover);
}

.slash-item-icon {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  font-size: var(--fs-base);
  font-weight: 600;
  flex-shrink: 0;
  color: var(--text-secondary);
}

.slash-item-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.slash-item-name {
  font-weight: 500;
  color: var(--text-primary);
}

.slash-item-desc {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
