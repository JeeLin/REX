<template>
  <div class="app-layout" :class="{ 'sidebar-collapsed': collapsed }">
    <!-- 移动端汉堡按钮 -->
    <button class="hamburger" @click="mobileOpen = !mobileOpen" v-if="!mobileOpen">
      <span></span>
      <span></span>
      <span></span>
    </button>

    <!-- 移动端遮罩 -->
    <div class="mobile-overlay" v-if="mobileOpen" @click="closeMobile"></div>

    <aside class="sidebar" :class="{ open: mobileOpen }">
      <!-- Header -->
      <div class="sidebar-header">
        <div class="sidebar-logo">R</div>
        <span class="sidebar-brand" v-show="!collapsed">REX Hub</span>
        <div class="sidebar-header-actions" v-show="!collapsed">
          <button class="sidebar-icon-btn" @click="toggleTheme" :title="themeLabel">
            {{ themeIcon }}
          </button>
          <button class="sidebar-icon-btn" @click="toggleLang" :title="lang === 'zh' ? '中文' : 'English'">
            {{ lang === 'zh' ? '中' : 'EN' }}
          </button>
        </div>
      </div>

      <!-- 搜索框 -->
      <div class="sidebar-search" v-show="!collapsed">
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t('sidebar.searchPlaceholder')"
          class="search-input"
        />
      </div>

      <!-- 导航 -->
      <nav class="sidebar-nav">
        <router-link to="/" class="nav-item" :class="{ active: route.name === 'dashboard' }" @click="closeMobile">
          <span class="nav-icon">◉</span>
          <span v-show="!collapsed">{{ t('nav.dashboard') }}</span>
        </router-link>
        <router-link to="/workspace" class="nav-item" :class="{ active: route.name === 'workspace' }" @click="closeMobile">
          <span class="nav-icon">⊞</span>
          <span v-show="!collapsed">{{ t('nav.workspace') }}</span>
        </router-link>
        <router-link to="/environments" class="nav-item" :class="{ active: isEnvPage }" @click="closeMobile">
          <span class="nav-icon">◈</span>
          <span v-show="!collapsed">{{ t('nav.environments') }}</span>
        </router-link>
        <router-link to="/agents" class="nav-item" :class="{ active: route.name === 'agents' }" @click="closeMobile">
          <span class="nav-icon">⬡</span>
          <span v-show="!collapsed">{{ t('nav.agents') }}</span>
        </router-link>
      </nav>

      <!-- 环境资源树 -->
      <div class="sidebar-tree" v-show="!collapsed">
        <div class="tree-label">{{ t('nav.environments') }}</div>
        <div v-if="loading" class="tree-loading">{{ t('common.loading') }}...</div>
        <div v-else-if="filteredEnvs.length === 0" class="tree-empty">{{ t('common.noData') }}</div>
        <template v-else>
          <div v-for="env in filteredEnvs" :key="env.id" class="env-group">
            <button class="env-group-header" @click="toggleEnvExpand(env.id)" @contextmenu.prevent="onEnvGroupCtx($event, env)">
              <span class="env-dot" :class="env.resources.length > 0 ? 'online' : 'offline'"></span>
              <span class="env-name">{{ env.name }}</span>
              <span class="env-count">[{{ env.resources.length }}]</span>
              <span class="env-arrow">{{ isEnvExpanded(env.id) ? '▾' : '▸' }}</span>
            </button>
            <div v-if="isEnvExpanded(env.id)" class="env-resources">
              <button
                v-for="res in env.resources"
                :key="res.id"
                class="resource-item"
                @click="connectToResource(res, env.name)"
                @contextmenu.prevent="onResourceItemCtx($event, res, env.name)"
              >
                <span class="res-dot" :style="{ background: getProtocolIcon(res.protocol).color }"></span>
                <span class="res-name">{{ res.name }}</span>
                <span class="res-protocol">{{ res.protocol }}</span>
              </button>
              <router-link
                :to="`/environments/${env.id}/resources/new`"
                class="resource-item add-resource"
                @click="closeMobile"
              >
                <span class="res-dot add-dot">+</span>
                <span class="res-name">{{ t('env.addResource') }}</span>
              </router-link>
            </div>
          </div>
        </template>
      </div>

      <!-- 收藏 -->
      <div class="sidebar-section" v-show="!collapsed">
        <div class="section-header">
          <span class="section-label">★ {{ t('sidebar.favorites') }}</span>
          <span class="section-count" v-if="favoriteResources.length">({{ favoriteResources.length }})</span>
        </div>
        <div v-if="favoriteResources.length === 0" class="section-empty">{{ t('sidebar.favoritesEmpty') }}</div>
        <div v-else class="section-list">
          <button
            v-for="fav in favoriteResources"
            :key="fav.id"
            class="resource-item fav-item"
            @click="connectToResource(fav, fav.envName)"
            @contextmenu.prevent="onFavItemCtx($event, fav)"
          >
            <span class="res-dot" :style="{ background: getProtocolIcon(fav.protocol).color }"></span>
            <span class="res-name">{{ fav.name }}</span>
            <span class="res-protocol">{{ fav.protocol }}</span>
            <button class="fav-remove" @click.stop="removeFavorite(fav.id)" :title="t('ctx.removeFavorite')">✕</button>
          </button>
        </div>
      </div>

      <!-- 最近使用 -->
      <div class="sidebar-section" v-show="!collapsed">
        <div class="section-header">
          <span class="section-label">⏱ {{ t('sidebar.recent') }}</span>
          <button v-if="recent.length > 0" class="section-action" @click="clearRecent" :title="t('sidebar.clearRecent')">🗑</button>
        </div>
        <div v-if="recent.length === 0" class="section-empty">{{ t('sidebar.recentEmpty') }}</div>
        <div v-else class="section-list">
          <button
            v-for="item in recent"
            :key="item.resourceId"
            class="resource-item recent-item"
            @click="connectToResource({ id: item.resourceId, name: item.name, protocol: item.protocol }, item.envName)"
          >
            <span class="res-dot" :style="{ background: getProtocolIcon(item.protocol).color }"></span>
            <span class="res-name">{{ item.name }}</span>
            <span class="res-time">{{ formatTimeAgo(item.usedAt) }}</span>
          </button>
        </div>
      </div>

      <!-- Footer -->
      <div class="sidebar-footer">
        <router-link to="/environments/new" class="nav-item" v-show="!collapsed" @click="closeMobile">
          <span class="nav-icon">+</span>
          <span>{{ t('sidebar.newEnv') }}</span>
        </router-link>
        <router-link
          v-if="auditEnabled"
          to="/audit-log"
          class="nav-item"
          :class="{ active: route.name === 'audit-log' }"
          @click="closeMobile"
        >
          <span class="nav-icon">📋</span>
          <span v-show="!collapsed">{{ t('nav.auditLog') }}</span>
        </router-link>
        <router-link to="/settings" class="nav-item" :class="{ active: route.name === 'settings' }" @click="closeMobile">
          <span class="nav-icon">⚙</span>
          <span v-show="!collapsed">{{ t('nav.settings') }}</span>
        </router-link>
        <button class="nav-item collapse-btn" @click="toggleCollapse">
          <span class="nav-icon">{{ collapsed ? '»' : '«' }}</span>
          <span v-show="!collapsed">{{ collapsed ? t('sidebar.expand') : t('sidebar.collapse') }}</span>
        </button>
      </div>
    </aside>

    <main class="main-content" :class="{ 'no-header': route.meta.noHeader }">
      <header v-if="!route.meta.noHeader" class="page-header">
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
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useUserStore, type Theme } from '@/stores/user'
import { useAuthStore } from '@/stores/auth'
import { useSidebar } from '@/composables/useSidebar'
import { getProtocolIcon } from '@/composables/useProtocol'
import { useContextMenu } from '@/composables/useContextMenu'
import { useRecent } from '@/composables/useRecent'

const { recent, clearRecent } = useRecent()

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const userStore = useUserStore()
const authStore = useAuthStore()
const { show: showMenu } = useContextMenu()

const lang = computed(() => userStore.lang)
const auditEnabled = ref(localStorage.getItem('rex-audit-enabled') !== 'false')

const {
  collapsed,
  searchQuery,
  filteredEnvs,
  loading,
  mobileOpen,
  favoriteResources,
  toggleCollapse,
  toggleEnvExpand,
  isEnvExpanded,
  fetchEnvs,
  connectToResource,
  addFavorite,
  removeFavorite,
  isFavorite,
  closeMobile,
} = useSidebar()

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

function onResourceItemCtx(e: MouseEvent, res: { id: string; name: string; protocol: string }, envName: string) {
  showMenu(e, [
    { label: t('ctx.connect'), action: () => connectToResource(res, envName) },
    { label: t('ctx.connectNewTab'), action: () => connectToResource(res, envName) },
    { separator: true },
    { label: t('ctx.editResource') },
    { label: t('ctx.deleteResource'), danger: true },
    { separator: true },
    { label: t('ctx.copyAddress'), action: () => navigator.clipboard?.writeText(res.name) },
    { label: isFavorite(res.id) ? t('ctx.removeFavorite') : t('ctx.addFavorite'), action: () => isFavorite(res.id) ? removeFavorite(res.id) : addFavorite(res.id) },
  ])
}

function onFavItemCtx(e: MouseEvent, fav: { id: string; name: string; protocol: string; envName: string }) {
  showMenu(e, [
    { label: t('ctx.connect'), action: () => connectToResource(fav, fav.envName) },
    { separator: true },
    { label: t('ctx.removeFavorite'), action: () => removeFavorite(fav.id) },
    { label: t('ctx.copyAddress'), action: () => navigator.clipboard?.writeText(fav.name) },
  ])
}

function formatTimeAgo(timestamp: number): string {
  const now = Date.now()
  const diff = now - timestamp
  const minutes = Math.floor(diff / 60000)
  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes}分钟前`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}小时前`
  const days = Math.floor(hours / 24)
  if (days < 30) return `${days}天前`
  return new Date(timestamp).toLocaleDateString()
}

function onEnvGroupCtx(e: MouseEvent, env: { id: string; name: string }) {
  showMenu(e, [
    { label: t('ctx.openDetail'), action: () => router.push(`/environments/${env.id}`) },
    { label: t('ctx.openAllWorkspace') },
    { separator: true },
    { label: t('ctx.newResource'), action: () => router.push(`/environments/${env.id}/resources/new`) },
    { label: t('ctx.editEnv') },
    { label: t('ctx.deleteEnv'), danger: true },
  ])
}

onMounted(() => {
  fetchEnvs()
  window.addEventListener('audit-toggle', ((e: CustomEvent) => {
    auditEnabled.value = e.detail.enabled
  }) as EventListener)
})
</script>

<style scoped>
.app-layout {
  display: flex;
  min-height: 100vh;
  background: var(--bg-deep);
}

/* ── 侧边栏 ─────────────────────────────── */
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
  transition: width var(--transition-normal);
  overflow-y: auto;
  overflow-x: hidden;
}

.sidebar-collapsed .sidebar {
  width: 56px;
}

.sidebar-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-lg);
  border-bottom: 1px solid var(--border);
  min-height: var(--header-height);
  flex-shrink: 0;
}

.sidebar-logo {
  width: 28px;
  height: 28px;
  min-width: 28px;
  background: var(--accent);
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: var(--fs-md);
  color: #000;
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

/* ── 搜索框 ─────────────────────────────── */
.sidebar-search {
  padding: var(--sp-sm) var(--sp-md);
  flex-shrink: 0;
}

.search-input {
  width: 100%;
  padding: var(--sp-xs) var(--sp-sm);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-deep);
  color: var(--text-primary);
  font-size: var(--fs-xs);
  outline: none;
  transition: border-color var(--transition-fast);
}

.search-input:focus {
  border-color: var(--accent);
}

.search-input::placeholder {
  color: var(--text-muted);
}

/* ── 导航 ─────────────────────────────── */
.sidebar-nav {
  padding: var(--sp-sm);
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex-shrink: 0;
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
  border: none;
  background: none;
  cursor: pointer;
  width: 100%;
  text-align: left;
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
  min-width: 20px;
  text-align: center;
  flex-shrink: 0;
}

/* ── 环境资源树 ─────────────────────────── */
.sidebar-tree {
  flex: 1;
  overflow-y: auto;
  padding: var(--sp-sm) var(--sp-md);
}

.tree-label {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: var(--sp-xs);
  padding: 0 var(--sp-sm);
}

.tree-loading,
.tree-empty {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  padding: var(--sp-sm);
  text-align: center;
}

.env-group {
  margin-bottom: var(--sp-xs);
}

.env-group-header {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  width: 100%;
  padding: var(--sp-xs) var(--sp-sm);
  border: none;
  background: none;
  color: var(--text-secondary);
  font-size: var(--fs-xs);
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: background var(--transition-fast);
}

.env-group-header:hover {
  background: var(--bg-hover);
}

.env-dot {
  width: 6px;
  height: 6px;
  min-width: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.env-dot.online { background: #22c55e; }
.env-dot.offline { background: #555; }

.env-name {
  flex: 1;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.env-count {
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 10px;
}

.env-arrow {
  color: var(--text-muted);
  font-size: 10px;
}

.env-resources {
  padding-left: var(--sp-lg);
}

.resource-item {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  width: 100%;
  padding: 3px var(--sp-sm);
  border: none;
  background: none;
  color: var(--text-secondary);
  font-size: var(--fs-xs);
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: background var(--transition-fast);
  text-decoration: none;
}

.resource-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
  text-decoration: none;
}

.res-dot {
  width: 6px;
  height: 6px;
  min-width: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.res-dot.add-dot {
  background: var(--bg-hover);
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  width: 14px;
  height: 14px;
  min-width: 14px;
  border-radius: var(--radius-sm);
}

.res-name {
  flex: 1;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.res-protocol {
  font-family: var(--font-mono);
  font-size: 9px;
  color: var(--text-muted);
  text-transform: uppercase;
}

/* ── Sidebar Sections (Favorites, Recent) ── */
.sidebar-section {
  padding: var(--sp-sm);
  border-top: 1px solid var(--border);
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--sp-xs);
  margin-bottom: var(--sp-xs);
}

.section-label {
  font-size: var(--fs-xs);
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.section-count {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.section-action {
  border: none;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--fs-xs);
  padding: 2px 4px;
  border-radius: var(--radius-sm);
}

.section-action:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.section-empty {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  padding: var(--sp-xs) var(--sp-sm);
  text-align: center;
}

.section-list {
  display: flex;
  flex-direction: column;
}

.fav-item {
  position: relative;
}

.fav-remove {
  border: none;
  background: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 10px;
  padding: 2px 4px;
  border-radius: var(--radius-sm);
  opacity: 0;
  transition: opacity var(--transition-fast);
}

.fav-item:hover .fav-remove {
  opacity: 1;
}

.fav-remove:hover {
  color: var(--danger);
  background: var(--bg-hover);
}

.res-time {
  font-size: 9px;
  color: var(--text-muted);
  white-space: nowrap;
}

/* ── Footer ─────────────────────────────── */
.sidebar-footer {
  padding: var(--sp-sm);
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}

.collapse-btn {
  justify-content: flex-start;
}

/* ── Main Content ───────────────────────── */
.main-content {
  flex: 1;
  margin-left: var(--sidebar-width);
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  transition: margin-left var(--transition-normal);
}

.sidebar-collapsed .main-content {
  margin-left: 56px;
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

.main-content.no-header .page-body {
  padding: 0;
}

/* ── 汉堡菜单（移动端） ──────────────────── */
.hamburger {
  display: none;
  position: fixed;
  top: var(--sp-md);
  left: var(--sp-md);
  z-index: calc(var(--z-sticky) + 2);
  width: 36px;
  height: 36px;
  border: none;
  background: var(--bg-surface);
  border-radius: var(--radius-md);
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  cursor: pointer;
  box-shadow: 0 2px 8px rgba(0,0,0,0.2);
}

.hamburger span {
  display: block;
  width: 18px;
  height: 2px;
  background: var(--text-primary);
  border-radius: 1px;
  transition: all var(--transition-fast);
}

.mobile-overlay {
  display: none;
}

@media (max-width: 767px) {
  .sidebar {
    transform: translateX(-100%);
    transition: transform var(--transition-normal);
    width: var(--sidebar-width) !important;
    z-index: calc(var(--z-sticky) + 2);
    pointer-events: none;
  }

  .sidebar.open {
    transform: translateX(0);
    pointer-events: auto;
  }

  .main-content {
    margin-left: 0 !important;
  }

  .hamburger {
    display: flex;
  }

  .mobile-overlay {
    display: block;
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.5);
    z-index: calc(var(--z-sticky) + 1);
  }

  .page-header {
    padding-left: 60px;
  }
}
</style>
