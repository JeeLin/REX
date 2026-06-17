<template>
  <!-- 占位：全局 Toast 组件 -->
  <div class="toast-container">
    <slot />
  </div>
</template>

<style scoped>
.toast-container {
  position: fixed;
  top: var(--sp-lg);
  right: var(--sp-lg);
  z-index: var(--z-toast);
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
}
</style>
