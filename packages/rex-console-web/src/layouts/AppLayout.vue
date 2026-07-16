<script setup lang="ts">
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import ResourcePanel from '@/features/resource-panel/ResourcePanel.vue'

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
const mobileMenuOpen = ref(false)

watch(collapsed, (v) => localStorage.setItem('sidebar-collapsed', String(v)))

const isWorkspace = computed(() => route.path === '/workspace')

const currentTitle = computed(() => {
  const match = [...mainNav, ...bottomNav].find((n) => route.path.startsWith(n.to))
  return match ? t(match.key) : 'REX Hub'
})
</script>

<template>
  <div class="app-layout" :class="{ 'app-layout--collapsed': collapsed, 'app-layout--fullscreen': fullscreen && isWorkspace }">
    <!-- 移动端菜单遮罩 -->
    <div v-if="mobileMenuOpen" class="mobile-overlay" @click="mobileMenuOpen = false" />

    <!-- 侧栏：工作区全屏时隐藏，移动端为抽屉 -->
    <aside v-if="!(fullscreen && isWorkspace)" class="sidebar" :class="{ 'sidebar--mobile-open': mobileMenuOpen }">
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
          @click="mobileMenuOpen = false"
        >
          <span class="nav-icon">{{ item.icon }}</span>
          <span v-if="!collapsed" class="nav-label">{{ t(item.key) }}</span>
        </RouterLink>
      </nav>

      <!-- 资源栏：嵌入侧栏，agents 和 audit-log 之间 -->
      <ResourcePanel v-if="!(fullscreen && isWorkspace)" class="sidebar-resource" />

      <nav class="sidebar-nav sidebar-bottom">
        <RouterLink
          v-for="item in bottomNav"
          :key="item.to"
          :to="item.to"
          class="nav-item"
          :title="collapsed ? t(item.key) : undefined"
          @click="mobileMenuOpen = false"
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
        <!-- 移动端汉堡按钮 -->
        <button class="hamburger-btn" @click="mobileMenuOpen = !mobileMenuOpen">☰</button>
        <span class="topbar-title mono">{{ currentTitle }}</span>
        <div class="topbar-actions">
          <button v-if="isWorkspace" class="fullscreen-btn mono" @click="fullscreen = !fullscreen" :title="fullscreen ? 'Exit fullscreen' : 'Fullscreen'">
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

    <!-- 移动端浮动快捷按钮 -->
    <div v-if="isWorkspace" class="mobile-fab">
      <button class="fab-btn" title="New tab">+</button>
      <button class="fab-btn" title="Split">⊞</button>
      <button class="fab-btn" title="Find">🔍</button>
    </div>

    <!-- 移动端底部导航 -->
    <nav class="bottom-nav">
      <RouterLink to="/dashboard" class="bottom-nav-item">
        <span class="bottom-nav-icon">◧</span>
        <span class="bottom-nav-label">{{ t('nav.dashboard') }}</span>
      </RouterLink>
      <RouterLink to="/environments" class="bottom-nav-item">
        <span class="bottom-nav-icon">⛁</span>
        <span class="bottom-nav-label">{{ t('nav.environments') }}</span>
      </RouterLink>
      <button class="bottom-nav-item bottom-nav-fab" @click="mobileMenuOpen = !mobileMenuOpen">
        <span class="bottom-nav-icon">☰</span>
      </button>
      <RouterLink to="/agents" class="bottom-nav-item">
        <span class="bottom-nav-icon">⬡</span>
        <span class="bottom-nav-label">{{ t('nav.agents') }}</span>
      </RouterLink>
      <RouterLink to="/settings" class="bottom-nav-item">
        <span class="bottom-nav-icon">⚙</span>
        <span class="bottom-nav-label">{{ t('nav.settings') }}</span>
      </RouterLink>
    </nav>
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

/* 移动端适配 */
.hamburger-btn {
  display: none;
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: var(--text-lg);
  cursor: pointer;
  padding: 0 var(--space-2) 0 0;
}
.mobile-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: 998;
}
.sidebar-resource {
  border-top: 1px solid var(--border);
  flex: 1;
  min-height: 0;
}

/* 移动端浮动快捷按钮 */
.mobile-fab {
  display: none;
  position: fixed;
  bottom: var(--space-5);
  right: var(--space-5);
  z-index: 999;
  flex-direction: column;
  gap: var(--space-2);
}
.fab-btn {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: var(--accent);
  color: var(--text-on-accent);
  border: none;
  font-size: var(--text-lg);
  cursor: pointer;
  box-shadow: var(--shadow-lg);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform var(--transition);
}
.fab-btn:active {
  transform: scale(0.92);
}

@media (max-width: 768px) {
  .sidebar {
    position: fixed;
    left: 0;
    top: 0;
    bottom: 0;
    width: var(--sidebar-width);
    z-index: 999;
    transform: translateX(-100%);
    transition: transform var(--transition);
  }
  .sidebar--mobile-open {
    transform: translateX(0);
  }
  .app-layout--collapsed .sidebar {
    width: var(--sidebar-width);
  }
  .hamburger-btn {
    display: block;
  }
  .resource-panel-desktop {
    display: none;
  }
  .app-layout--collapsed .nav-item {
    justify-content: flex-start;
    padding: var(--space-2) var(--space-3);
  }
  .content {
    padding: var(--space-3);
  }
  .mobile-fab {
    display: flex;
  }
  .bottom-nav {
    display: flex;
  }
  .content {
    padding-bottom: 64px;
  }
}

/* Bottom nav (mobile only) */
.bottom-nav {
  display: none;
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  height: 56px;
  background: var(--bg-surface);
  border-top: 1px solid var(--border);
  z-index: 997;
  justify-content: space-around;
  align-items: center;
}
.bottom-nav-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  text-decoration: none;
  color: var(--text-muted);
  font-size: var(--text-xs);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius);
  transition: color var(--transition);
  background: none;
  border: none;
  cursor: pointer;
}
.bottom-nav-item.router-link-active,
.bottom-nav-item.bottom-nav-fab {
  color: var(--accent);
}
.bottom-nav-icon {
  font-size: 18px;
  line-height: 1;
}
.bottom-nav-label {
  font-size: 10px;
  line-height: 1;
}
</style>
