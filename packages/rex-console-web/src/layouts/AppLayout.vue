<template>
  <div class="app-layout">
    <aside class="sidebar">
      <nav><!-- 侧边栏占位 --></nav>
    </aside>
    <main class="content">
      <header class="topbar"><!-- 顶栏占位 --></header>
      <router-view />
    </main>
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  min-height: 100vh;
  background: var(--bg-deep);
}

.sidebar {
  width: 240px;
  background: var(--bg-surface);
  border-right: 1px solid var(--border);
}

.content {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.topbar {
  height: 48px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
}
</style>
