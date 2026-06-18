<template>
  <div class="app-layout">
    <aside class="sidebar">
      <div class="sidebar-header">
        <div class="sidebar-logo">R</div>
        <span class="sidebar-brand">REX Hub</span>
        <div class="sidebar-header-actions">
          <button class="sidebar-icon-btn" @click="toggleTheme" :title="themeLabel">
            {{ themeIcon }}
          </button>
          <button class="sidebar-icon-btn" @click="toggleLang" :title="lang === 'zh' ? '中文' : 'English'">
            {{ lang === 'zh' ? '中' : 'EN' }}
          </button>
        </div>
      </div>

      <nav class="sidebar-nav">
        <router-link to="/" class="nav-item" :class="{ active: route.name === 'dashboard' }">
          <span class="nav-icon">◉</span>
          <span>{{ t('nav.dashboard') }}</span>
        </router-link>
        <router-link to="/environments" class="nav-item" :class="{ active: isEnvPage }">
          <span class="nav-icon">◈</span>
          <span>{{ t('nav.environments') }}</span>
        </router-link>
        <router-link to="/agents" class="nav-item" :class="{ active: route.name === 'agents' }">
          <span class="nav-icon">⬡</span>
          <span>{{ t('nav.agents') }}</span>
        </router-link>
        <router-link to="/audit-log" class="nav-item" :class="{ active: route.name === 'audit-log' }">
          <span class="nav-icon">📋</span>
          <span>{{ t('nav.auditLog') }}</span>
        </router-link>
      </nav>

      <div class="sidebar-footer">
        <router-link to="/environments/new" class="nav-item">
          <span class="nav-icon">+</span>
          <span>{{ t('env.create') }}</span>
        </router-link>
      </div>
    </aside>

    <main class="main-content">
      <header class="page-header">
        <h1 class="page-title">{{ pageTitle }}</h1>
        <div class="header-actions">
          <button class="btn btn-ghost btn-sm" @click="handleLogout">
            {{ t('auth.logout') }}
          </button>
        </div>
      </header>
      <div class="page-body">
        <router-view />
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useUserStore, type Theme } from '@/stores/user'
import { useAuthStore } from '@/stores/auth'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const userStore = useUserStore()
const authStore = useAuthStore()

const lang = computed(() => userStore.lang)

const isEnvPage = computed(() => {
  const name = route.name as string
  return name?.startsWith('environment')
})

const pageTitle = computed(() => {
  const name = route.name as string
  const map: Record<string, string> = {
    dashboard: t('nav.dashboard'),
    environments: t('nav.environments'),
    'environment-new': t('env.create'),
    'environment-detail': t('nav.environments'),
    'resource-new': t('resource.create'),
    agents: t('nav.agents'),
    'audit-log': t('nav.auditLog'),
    settings: t('nav.settings'),
  }
  return map[name] || ''
})

const themeIcon = computed(() => {
  const icons: Record<string, string> = { dark: '☀', light: '☾', system: '◐' }
  return icons[userStore.theme] || '☀'
})

const themeLabel = computed(() => {
  const labels: Record<string, string> = { dark: 'Dark', light: 'Light', system: 'Auto' }
  return labels[userStore.theme] || ''
})

function toggleTheme() {
  const cycle: Record<Theme, Theme> = { dark: 'light', light: 'system', system: 'dark' }
  userStore.setTheme(cycle[userStore.theme])
}

function toggleLang() {
  userStore.setLang(userStore.lang === 'zh' ? 'en' : 'zh')
  location.reload()
}

function handleLogout() {
  authStore.logout()
  router.push('/login')
}
</script>

<style scoped>
.app-layout {
  display: flex;
  min-height: 100vh;
  background: var(--bg-deep);
}

.sidebar {
  width: var(--sidebar-width);
  background: var(--bg-surface);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  position: fixed;
  top: 0;
  left: 0;
  bottom: 0;
  z-index: var(--z-sticky);
}

.sidebar-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-lg);
  border-bottom: 1px solid var(--border);
  min-height: var(--header-height);
}

.sidebar-logo {
  width: 28px;
  height: 28px;
  background: var(--accent);
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: var(--fs-md);
  color: #000;
  flex-shrink: 0;
}

.sidebar-brand {
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: var(--fs-md);
  flex: 1;
}

.sidebar-header-actions {
  display: flex;
  gap: var(--sp-xs);
}

.sidebar-icon-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: var(--fs-sm);
  transition: all var(--transition-fast);
}

.sidebar-icon-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.sidebar-nav {
  flex: 1;
  padding: var(--sp-sm);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-sm) var(--sp-md);
  border-radius: var(--radius-md);
  color: var(--text-secondary);
  font-size: var(--fs-sm);
  text-decoration: none;
  transition: all var(--transition-fast);
}

.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
  text-decoration: none;
}

.nav-item.active {
  background: var(--bg-hover);
  color: var(--accent);
}

.nav-icon {
  width: 20px;
  text-align: center;
  flex-shrink: 0;
}

.sidebar-footer {
  padding: var(--sp-sm);
  border-top: 1px solid var(--border);
}

.main-content {
  flex: 1;
  margin-left: var(--sidebar-width);
  display: flex;
  flex-direction: column;
  min-height: 100vh;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-lg) var(--sp-xl);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  min-height: var(--header-height);
}

.page-title {
  font-family: var(--font-mono);
  font-size: var(--fs-md);
  font-weight: 600;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.page-body {
  padding: var(--sp-xl);
  overflow-y: auto;
  flex: 1;
}

@media (max-width: 767px) {
  .sidebar {
    display: none;
  }
  .main-content {
    margin-left: 0;
  }
}
</style>
