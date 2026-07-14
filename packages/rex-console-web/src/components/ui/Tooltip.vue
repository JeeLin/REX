<script setup lang="ts">
withDefaults(defineProps<{ text: string; placement?: 'top' | 'bottom' | 'left' | 'right' }>(), {
  placement: 'top',
})
</script>

<template>
  <span class="tip-wrap">
    <slot />
    <span class="tip" :class="`tip--${placement}`" role="tooltip">{{ text }}</span>
  </span>
</template>

<style scoped>
.tip-wrap {
  position: relative;
  display: inline-flex;
}
.tip {
  position: absolute;
  background: var(--bg-elevated);
  color: var(--text-primary);
  border: 1px solid var(--border-strong);
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  white-space: nowrap;
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--transition);
  z-index: 90;
}
.tip-wrap:hover .tip {
  opacity: 1;
}
.tip--top {
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  margin-bottom: 6px;
}
.tip--bottom {
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  margin-top: 6px;
}
.tip--left {
  right: 100%;
  top: 50%;
  transform: translateY(-50%);
  margin-right: 6px;
}
.tip--right {
  left: 100%;
  top: 50%;
  transform: translateY(-50%);
  margin-left: 6px;
}
</style>
