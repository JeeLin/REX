<script setup lang="ts">
// 2.0 占位页 — 工作空间外壳（后续 M2 实现 Xshell 模式）
</script>

<template>
  <div class="workspace-stub">
    <div class="ws-bar mono">Workspace</div>
    <div class="ws-body">
      <p class="muted">连接树 · Tab · 分屏 · 状态栏（M2）</p>
    </div>
  </div>
</template>

<style scoped>
.workspace-stub {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-page);
}
.ws-bar {
  height: var(--tabbar-height);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 var(--space-4);
  color: var(--text-secondary);
  background: var(--bg-surface);
}
.ws-body {
  flex: 1;
  display: grid;
  place-items: center;
}
</style>
