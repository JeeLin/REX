<template>
  <Teleport to="body">
    <div
      v-if="menuVisible"
      class="ctx-overlay"
      @click="menuHide"
      @contextmenu.prevent="menuHide"
    ></div>
    <div
      v-if="menuVisible"
      class="ctx-menu"
      :style="{ left: menuX + 'px', top: menuY + 'px' }"
    >
      <template v-for="(item, idx) in menuItems" :key="idx">
        <div v-if="item.separator" class="ctx-separator"></div>
        <div
          v-else
          class="ctx-item"
          :class="{
            'ctx-danger': item.danger,
            'ctx-disabled': item.disabled,
            'ctx-active': openSubIdx === idx,
          }"
          @click="handleClick(item)"
          @mouseenter="item.children ? (openSubIdx = idx) : undefined"
          @mouseleave="onItemLeave"
        >
          <span v-if="item.icon" class="ctx-icon">{{ item.icon }}</span>
          <span class="ctx-label">{{ item.label }}</span>
          <span v-if="item.children" class="ctx-arrow">▸</span>
        </div>
      </template>
    </div>
    <div
      v-if="menuVisible && openSubItem"
      class="ctx-menu ctx-submenu"
      :style="submenuStyle"
      @mouseenter="keepSub = true"
      @mouseleave="openSubIdx = null; keepSub = false"
    >
      <template v-for="(child, ci) in openSubItem.children!" :key="ci">
        <div v-if="child.separator" class="ctx-separator"></div>
        <div
          v-else
          class="ctx-item"
          :class="{
            'ctx-danger': child.danger,
            'ctx-disabled': child.disabled,
          }"
          @click="handleSubClick(child)"
        >
          <span v-if="child.icon" class="ctx-icon">{{ child.icon }}</span>
          <span class="ctx-label">{{ child.label }}</span>
        </div>
      </template>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useContextMenu, type MenuItem } from '@/composables/useContextMenu'

const { visible: menuVisible, x: menuX, y: menuY, items: menuItems, hide: menuHide } = useContextMenu()

const openSubIdx = ref<number | null>(null)
const keepSub = ref(false)

const openSubItem = computed(() => {
  if (openSubIdx.value === null) return null
  return menuItems.value[openSubIdx.value] ?? null
})

const submenuStyle = computed(() => {
  if (openSubIdx.value === null) return { display: 'none' }
  const menuWidth = 180
  return {
    left: `${menuX.value + menuWidth + 4}px`,
    top: `${menuY.value + (openSubIdx.value! * 36)}px`,
  }
})

function onItemLeave() {
  if (!keepSub.value) {
    openSubIdx.value = null
  }
}

function handleClick(item: MenuItem) {
  if (item.disabled) return
  if (item.children) {
    openSubIdx.value = null
    return
  }
  item.action?.()
  menuHide()
}

function handleSubClick(child: MenuItem) {
  if (child.disabled) return
  child.action?.()
  menuHide()
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') menuHide()
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown)
})
</script>

<style>
.ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 999;
}

.ctx-menu {
  position: fixed;
  z-index: 1000;
  min-width: 180px;
  max-width: 280px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  padding: var(--sp-xs) 0;
  font-size: var(--fs-sm);
  user-select: none;
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-sm) var(--sp-lg);
  cursor: pointer;
  color: var(--text-primary);
  transition: background var(--transition-fast);
  white-space: nowrap;
}

.ctx-item:hover {
  background: var(--bg-hover);
}

.ctx-danger {
  color: var(--danger);
}

.ctx-disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.ctx-disabled:hover {
  background: transparent;
}

.ctx-icon {
  width: 16px;
  text-align: center;
  flex-shrink: 0;
  font-size: var(--fs-xs);
}

.ctx-label {
  flex: 1;
}

.ctx-arrow {
  color: var(--text-muted);
  font-size: var(--fs-xs);
}

.ctx-separator {
  height: 1px;
  background: var(--border);
  margin: var(--sp-xs) 0;
}

.ctx-submenu {
  position: fixed;
}

.ctx-active {
  background: var(--bg-hover);
}
</style>
