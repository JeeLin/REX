<template>
  <div class="responsive-table-wrap">
    <slot />
  </div>
</template>

<style scoped>
.responsive-table-wrap {
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
  border: 1px solid var(--border);
  border-radius: 8px;
}
.responsive-table-wrap :deep(table) {
  width: 100%;
  min-width: 600px;
}
</style>
