<script setup lang="ts">
// 连接状态点：online 发光、connecting 脉冲、offline 灰、error 红
export type StatusDotStatus = 'online' | 'offline' | 'connecting' | 'error'
withDefaults(
  defineProps<{ status?: 'online' | 'offline' | 'connecting' | 'error' }>(),
  { status: 'offline' },
)
</script>

<template>
  <span class="dot" :class="`dot--${status}`" />
</template>

<style scoped>
.dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.dot--online {
  background: var(--success);
  box-shadow: 0 0 6px var(--success);
}
.dot--offline {
  background: var(--text-muted);
}
.dot--connecting {
  background: var(--warning);
  animation: pulse 1.2s ease-in-out infinite;
}
.dot--error {
  background: var(--danger);
  box-shadow: 0 0 6px var(--danger);
}
@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.3;
  }
}
</style>
