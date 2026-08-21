<script setup lang="ts">
withDefaults(defineProps<{
  height?: string
  horizontal?: boolean
}>(), { height: '', horizontal: false })
</script>

<template>
  <div
    class="scrollbar"
    :class="{ 'scrollbar--horizontal': horizontal }"
    :style="height ? { height } : {}"
  >
    <slot />
  </div>
</template>

<style scoped>
.scrollbar {
  overflow-y: auto;
  overflow-x: hidden;
}
.scrollbar--horizontal {
  overflow-x: auto;
  overflow-y: hidden;
}
.scrollbar::-webkit-scrollbar {
  width: var(--scrollbar-width);
  height: var(--scrollbar-width);
}
.scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.scrollbar::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: var(--radius-pill);
}
.scrollbar::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
}
/* Firefox */
.scrollbar {
  scrollbar-width: thin;
  scrollbar-color: var(--scrollbar-thumb) transparent;
}
</style>
