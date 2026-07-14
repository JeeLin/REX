<script setup lang="ts">
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { computed, ref, watch } from 'vue'
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

const collapsed = ref(localStorage.getItem('sidebar-collapsed') === 'true')
const fullscreen = ref(false)

watch(collapsed, (v) => localStorage.setItem('sidebar-collapsed', String(v)))

const isWorkspace = computed(() => route.path === '/workspace')

const currentTitle = computed(() => {
  const match = [...mainNav, ...bottomNav].find((n) => route.path.startsWith(n.to))
  return match ? t(match.key) : 'REX Hub'
})
</script>

<template>
  <div class="app-layout" :class="{ 'app-layout--collapsed': collapsed, 'app-layout--fullscreen': fullscreen && isWorkspace }">
    <!-- 侧栏：工作区全屏时隐藏 -->
    <aside v-if="!(fullscreen && isWorkspace)" class="sidebar">
      <div class="sidebar-brand mono">
        <span v-if="!collapsed">REX<span class="accent">Hub</span></span>
        <span v-else class="brand-mini">R</span>
      </div>
      <nav class="sidebar-nav">
        <RouterLink
          v-for="item in mainNav"
          :key="item.to"
          :to="item.to"
          class="nav-item"
          :title="collapsed ? t(item.key) : undefined"
        >
          <span class="nav-icon">{{ item.icon }}</span>
          <span v-if="!collapsed" class="nav-label">{{ t(item.key) }}</span>
        </RouterLink>
      </nav>
      <div class="sidebar-spacer" />
      <nav class="sidebar-nav sidebar-bottom">
        <RouterLink
          v-for="item in bottomNav"
          :key="item.to"
          :to="item.to"
          class="nav-item"
          :title="collapsed ? t(item.key) : undefined"
        >
          <span class="nav-icon">{{ item.icon }}</span>
          <span v-if="!collapsed" class="nav-label">{{ t(item.key) }}</span>
        </RouterLink>
        <button class="nav-item nav-toggle" @click="collapsed = !collapsed" :title="collapsed ? 'Expand sidebar' : 'Collapse sidebar'">
          <span class="nav-icon">{{ collapsed ? '»' : '«' }}</span>
          <span v-if="!collapsed" class="nav-label">Collapse</span>
        </button>
      </nav>
    </aside>
    <div class="main">
      <header v-if="!(fullscreen && isWorkspace)" class="topbar">
        <span class="topbar-title mono">{{ currentTitle }}</span>
        <div v-if="isWorkspace" class="topbar-actions">
          <button class="fullscreen-btn mono" @click="fullscreen = !fullscreen" :title="fullscreen ? 'Exit fullscreen' : 'Fullscreen'">
            {{ fullscreen ? '⊟' : '⊞' }}
          </button>
        </div>
      </header>
      <!-- 全屏时工作区的退出按钮 -->
      <button
        v-if="fullscreen && isWorkspace"
        class="exit-fullscreen-btn mono"
        @click="fullscreen = false"
        title="Exit fullscreen (Esc)"
      >
        ⊟
      </button>
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
.app-layout--fullscreen {
  position: relative;
}

/* Sidebar */
.sidebar {
  width: var(--sidebar-width);
  flex-shrink: 0;
  background: var(--bg-surface);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  transition: width var(--transition);
}
.app-layout--collapsed .sidebar {
  width: 56px;
}
.sidebar-brand {
  height: var(--topbar-height);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 var(--space-4);
  font-size: var(--text-lg);
  font-weight: 700;
  border-bottom: 1px solid var(--border);
  overflow: hidden;
  white-space: nowrap;
}
.brand-mini {
  color: var(--accent);
  font-size: var(--text-xl);
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
  border: none;
  background: none;
  width: 100%;
  cursor: pointer;
  transition: background var(--transition), color var(--transition);
}
.app-layout--collapsed .nav-item {
  justify-content: center;
  padding: var(--space-2);
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
  flex-shrink: 0;
}
.nav-toggle {
  color: var(--text-muted);
  font-size: var(--text-sm);
}
.nav-toggle:hover {
  color: var(--text-primary);
}

/* Main */
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
  justify-content: space-between;
  padding: 0 var(--space-4);
  background: var(--bg-page);
}
.topbar-title {
  font-size: var(--text-base);
  color: var(--text-secondary);
}
.topbar-actions {
  display: flex;
  gap: var(--space-2);
}
.fullscreen-btn {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-secondary);
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-size: var(--text-sm);
  transition: color var(--transition);
}
.fullscreen-btn:hover {
  color: var(--accent);
}
.exit-fullscreen-btn {
  position: absolute;
  top: var(--space-2);
  right: var(--space-2);
  z-index: 100;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-secondary);
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-size: var(--text-sm);
}
.exit-fullscreen-btn:hover {
  color: var(--accent);
}
.content {
  flex: 1;
  overflow: auto;
  padding: var(--space-5);
}
</style>
