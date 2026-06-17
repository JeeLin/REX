<template>
  <!-- 占位：统一右键菜单组件 -->
  <div v-if="visible" class="context-menu" :style="{ left: `${x}px`, top: `${y}px` }">
    <slot />
  </div>
</template>

<script setup lang="ts">
defineProps<{
  visible: boolean
  x: number
  y: number
}>()
</script>

<style scoped>
.context-menu {
  position: fixed;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  z-index: var(--z-dropdown);
  min-width: 180px;
  padding: var(--sp-xs) 0;
}
</style>
