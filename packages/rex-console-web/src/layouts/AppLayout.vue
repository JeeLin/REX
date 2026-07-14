<script setup lang="ts">
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const route = useRoute()

const mainNav = [
  { to: '/workspace', key: 'nav.workspace', icon: '▤' },
  { to: '/dashboard', key: 'nav.dashboard', icon: '◧' },
  { to: '/environments', key: 'nav.environments', icon: '⛁' },
  { to: '/agents', key: 'nav.agents', icon: '⬡' },
]

const bottomNav = [
  { to: '/audit-log', key: 'nav.auditLog', icon: '☰' },
  { to: '/settings', key: 'nav.settings', icon: '⚙' },
]

const currentTitle = computed(() => {
  const match = [...mainNav, ...bottomNav].find((n) => route.path.startsWith(n.to))
  return match ? t(match.key) : 'REX Hub'
})
</script>

<template>
  <div class="app-layout">
    <aside class="sidebar">
      <div class="sidebar-brand mono">REX<span class="accent">Hub</span></div>
      <nav class="sidebar-nav">
        <RouterLink v-for="item in mainNav" :key="item.to" :to="item.to" class="nav-item">
          <span class="nav-icon">{{ item.icon }}</span>
          <span class="nav-label">{{ t(item.key) }}</span>
        </RouterLink>
      </nav>
      <div class="sidebar-spacer" />
      <nav class="sidebar-nav sidebar-bottom">
        <RouterLink v-for="item in bottomNav" :key="item.to" :to="item.to" class="nav-item">
          <span class="nav-icon">{{ item.icon }}</span>
          <span class="nav-label">{{ t(item.key) }}</span>
        </RouterLink>
      </nav>
    </aside>
    <div class="main">
      <header class="topbar">
        <span class="topbar-title mono">{{ currentTitle }}</span>
      </header>
      <main class="content">
        <RouterView />
      </main>
    </div>
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  height: 100%;
}
.sidebar {
  width: var(--sidebar-width);
  flex-shrink: 0;
  background: var(--bg-surface);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
}
.sidebar-brand {
  height: var(--topbar-height);
  display: flex;
  align-items: center;
  padding: 0 var(--space-4);
  font-size: var(--text-lg);
  font-weight: 700;
  border-bottom: 1px solid var(--border);
}
.accent {
  color: var(--accent);
}
.sidebar-nav {
  padding: var(--space-3) var(--space-2);
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.sidebar-bottom {
  padding-bottom: var(--space-4);
  border-top: 1px solid var(--border);
}
.sidebar-spacer {
  flex: 1;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius);
  color: var(--text-secondary);
  font-size: var(--text-base);
  text-decoration: none;
}
.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
  text-decoration: none;
}
.nav-item.router-link-active {
  background: var(--accent-soft);
  color: var(--accent);
}
.nav-icon {
  font-size: var(--text-md);
  width: 18px;
  text-align: center;
}
.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.topbar {
  height: var(--topbar-height);
  flex-shrink: 0;
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 var(--space-4);
  background: var(--bg-page);
}
.topbar-title {
  font-size: var(--text-base);
  color: var(--text-secondary);
}
.content {
  flex: 1;
  overflow: auto;
  padding: var(--space-5);
}
</style>
