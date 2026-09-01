<script setup lang="ts">
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { computed, ref, onMounted, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import ResourcePanel from '@/features/resource-panel/ResourcePanel.vue'
import { useSessionTimeout } from '@/composables/useSessionTimeout'
import { useAuthStore } from '@/stores/auth'
import Modal from '@/components/ui/Modal.vue'
import Button from '@/components/ui/Button.vue'
import type { Resource } from '@/api/resources'
import { useRouter } from 'vue-router'
import { useSwipeGesture } from '@/composables/useSwipeGesture'
import { useVirtualKeyboard } from '@/composables/useVirtualKeyboard'

const { t, locale } = useI18n()
const { showWarning, remainingSeconds, extendSession } = useSessionTimeout()
const authStore = useAuthStore()
const router = useRouter()

function onResourceProperties(res: Resource) {
  router.push({ name: 'environment-detail', params: { id: res.environment_id } })
}

function sessionLogout() {
  authStore.logout()
  window.location.href = '/login'
}
const route = useRoute()

const mainNav = [
  { to: '/workspace', key: 'nav.workspace', icon: 'grid' },
  { to: '/dashboard', key: 'nav.dashboard', icon: 'chart' },
  { to: '/environments', key: 'nav.environments', icon: 'list' },
  { to: '/audit-log', key: 'nav.auditLog', icon: 'bolt' },
]

const bottomNav = [
  { to: '/settings', key: 'nav.settings', icon: 'gear' },
]

const fullscreen = ref(false)
const mobileMenuOpen = ref(false)
const searchQuery = ref('')


const isWorkspace = computed(() => route.path === '/workspace')

// 移动端滑动手势
const mainRef = ref<HTMLElement | null>(null)
useSwipeGesture(mainRef, {
  onSwipeRight: () => { if (window.innerWidth < 768) mobileMenuOpen.value = true },
  onSwipeLeft: () => { mobileMenuOpen.value = false },
})

// 移动端虚拟键盘检测
const { isKeyboardVisible } = useVirtualKeyboard()

// F11 全屏切换
function handleGlobalKeydown(e: KeyboardEvent) {
  if (e.key === 'F11') {
    e.preventDefault()
    fullscreen.value = !fullscreen.value
  }
}
onMounted(() => document.addEventListener('keydown', handleGlobalKeydown))
onBeforeUnmount(() => document.removeEventListener('keydown', handleGlobalKeydown))

const currentTitle = computed(() => {
  const match = [...mainNav, ...bottomNav].find((n) => route.path.startsWith(n.to))
  return match ? t(match.key) : 'REX'
})
const currentUser = computed(() => 'R')

function toggleTheme() {
  const current = localStorage.getItem('rex-theme') || 'dark'
  const next = current === 'dark' ? 'light' : 'dark'
  localStorage.setItem('rex-theme', next)
  document.documentElement.dataset.theme = next === 'dark' ? undefined : next
}
function toggleLanguage() {
  locale.value = locale.value === 'zh' ? 'en' : 'zh'
  localStorage.setItem('rex-lang', locale.value)
}

function openQuickConnect() {
  router.push('/workspace')
}
function refreshAllData() {
  window.location.reload()
}

function openUserMenu() {
  router.push('/settings')
}

</script>

<template>
  <div class="app-layout" :class="{ 'app-layout--fullscreen': fullscreen && isWorkspace }">
    <!-- 移动端菜单遮罩 -->
    <div v-if="mobileMenuOpen" class="mobile-overlay" @click="mobileMenuOpen = false" />

    <!-- 侧栏：工作区全屏时隐藏，移动端为抽屉 -->
    <aside v-if="!(fullscreen && isWorkspace)" class="sidebar" :class="{ 'sidebar--mobile-open': mobileMenuOpen }">
      <!-- Brand 区 -->
      <div class="sidebar-brand">
        <div class="brand-left">
          <div class="brand-glyph">R</div>
          <span class="brand-name mono">RE<span class="brand-accent">X</span></span>
        </div>
        <div class="brand-actions">
          <button class="brand-btn" :aria-label="t('theme.toggle', 'Toggle theme')" @click="toggleTheme">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8z"/>
            </svg>
          </button>
          <button class="brand-btn" :aria-label="t('language.toggle', 'Language')" @click="toggleLanguage">
            {{ locale === 'zh' ? '中' : 'EN' }}
          </button>
        </div>
      </div>

      <!-- 搜索框 -->
      <div class="sidebar-search">
        <div class="search-box" @click="($refs.searchInput as HTMLInputElement)?.focus()">
          <svg class="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>
          </svg>
          <input
            ref="searchInput"
            v-model="searchQuery"
            type="text"
            class="search-input mono"
            :aria-label="t('common.search', 'Search')"
            :placeholder="t('sidebar.searchPlaceholder', 'Search resources…')"
          />
          <kbd class="search-kbd">Ctrl K</kbd>
        </div>
      </div>

      <!-- 主导航 -->
      <nav class="sidebar-nav">
        <RouterLink
          v-for="item in mainNav"
          :key="item.to"
          :to="item.to"
          class="nav-item"
          @click="mobileMenuOpen = false"
        >
          <!-- Grid icon (Workspace) -->
          <svg v-if="item.icon === 'grid'" class="nav-svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/>
          </svg>
          <!-- Chart icon (Dashboard) -->
          <svg v-else-if="item.icon === 'chart'" class="nav-svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/>
          </svg>
          <!-- List icon (Environments) -->
          <svg v-else-if="item.icon === 'list'" class="nav-svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/>
          </svg>
          <!-- Bolt icon (Audit log) -->
          <svg v-else-if="item.icon === 'bolt'" class="nav-svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
          </svg>
          <!-- Grid icon (Design) -->
          <svg v-else-if="item.icon === 'design'" class="nav-svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>
          </svg>
          <span class="nav-label">{{ t(item.key) }}</span>
        </RouterLink>
      </nav>

      <!-- 资源栏 -->
      <ResourcePanel v-if="!(fullscreen && isWorkspace)" class="sidebar-resource" @resource-properties="onResourceProperties" />

      <!-- 底部按钮 -->
      <div class="sidebar-bottom">
        <div class="sidebar-bottom-buttons">
          <button class="sidebar-action-btn" @click="router.push('/environments?create=true')">
            <span class="sidebar-action-plus">+</span>
            {{ t('sidebar.newEnvironment', 'New env') }}
          </button>
          <RouterLink to="/settings" class="sidebar-action-btn sidebar-action-settings" @click="mobileMenuOpen = false">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
            </svg>
            {{ t('nav.settings', 'Settings') }}
          </RouterLink>
        </div>
      </div>
    </aside>

    <div ref="mainRef" class="main">
      <!-- 顶栏 -->
      <header v-if="!(fullscreen && isWorkspace)" class="topbar">
        <!-- 移动端汉堡按钮 -->
        <button class="hamburger-btn" :aria-label="t('common.menu', 'Menu')" @click="mobileMenuOpen = !mobileMenuOpen">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="18" x2="21" y2="18"/>
          </svg>
        </button>

        <!-- 面包屑 -->
        <div class="topbar-breadcrumb">
          <span class="breadcrumb-root">REX</span>
          <span class="breadcrumb-sep">/</span>
          <span class="breadcrumb-current">{{ currentTitle }}</span>
        </div>

        <div class="topbar-spacer" />

        <!-- Quick connect (topbar search placeholder until v0.72.0 command palette) -->
        <div class="topbar-search-inline" @click="openQuickConnect">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="9 10 4 15 9 20"/><path d="M20 4v7H4"/>
          </svg>
          <span>{{ t('topbar.quickConnect', 'Go to workspace…') }}</span>
          <kbd class="topbar-search-kbd">Ctrl K</kbd>
        </div>

        <!-- 右侧按钮 + 头像 -->
        <div class="topbar-actions">
          <button class="topbar-icon-btn" :aria-label="t('topbar.notifications', 'Notifications')" :title="t('topbar.notifications', 'Notifications')">
            <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.7 21a2 2 0 0 1-3.4 0"/>
            </svg>
          </button>
          <button class="topbar-icon-btn" :aria-label="t('topbar.refresh', 'Refresh data')" :title="t('topbar.refresh', 'Refresh data (R)')" @click="refreshAllData">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 12a9 9 0 1 1-3-6.7L21 8"/><path d="M21 3v5h-5"/>
            </svg>
          </button>
          <button v-if="isWorkspace" class="topbar-icon-btn" :aria-label="fullscreen ? t('common.exitFullscreen') : t('common.fullscreen')" :title="fullscreen ? t('common.exitFullscreen') : t('common.fullscreen')" @click="fullscreen = !fullscreen">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline v-if="!fullscreen" points="15 3 21 3 21 9"/><polyline v-if="!fullscreen" points="9 21 3 21 3 15"/><line v-if="!fullscreen" x1="21" y1="3" x2="14" y2="10"/><line v-if="!fullscreen" x1="3" y1="21" x2="10" y2="14"/>
              <polyline v-if="fullscreen" points="14 14 4 14 4 4"/><polyline v-if="fullscreen" points="10 10 20 10 20 20"/><line v-if="fullscreen" x1="4" y1="14" x2="10" y2="10"/><line v-if="fullscreen" x1="20" y1="10" x2="14" y2="14"/>
            </svg>
          </button>
          <div class="topbar-avatar" :title="currentUser" @click="openUserMenu" style="cursor:pointer">
            {{ currentUser.charAt(0).toUpperCase() }}
          </div>
          <button class="topbar-icon-btn" :aria-label="t('session.logout', 'Logout')" :title="t('session.logout', 'Logout')" @click="sessionLogout" style="color:var(--danger)">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/>
            </svg>
          </button>
        </div>
      </header>
      <!-- 全屏时工作区的退出按钮 -->
      <button
        v-if="fullscreen && isWorkspace"
        class="exit-fullscreen-btn mono"
        :aria-label="t('common.exitFullscreen')"
        :title="t('common.exitFullscreen') + ' (Esc)'"
        @click="fullscreen = false"
      >
        ⊟
      </button>
      <main class="content" :class="{ 'content--keyboard-open': isKeyboardVisible }">
        <RouterView v-slot="{ Component }">
          <KeepAlive :exclude="['login', 'setup', 'EnvironmentDetailPage']">
            <component :is="Component" />
          </KeepAlive>
        </RouterView>
      </main>
    </div>

    <!-- 移动端浮动快捷按钮 -->
    <div v-if="isWorkspace" class="mobile-fab">
      <button class="fab-btn" :aria-label="t('workspace.newTab')" :title="t('workspace.newTab')">+</button>
      <button class="fab-btn" :aria-label="t('workspace.split')" :title="t('workspace.split')">⊞</button>
      <button class="fab-btn" :aria-label="t('common.find')" :title="t('common.find')">🔍</button>
    </div>

    <!-- 移动端底部导航（键盘弹出时隐藏） -->
    <nav v-show="!isKeyboardVisible" class="bottom-nav">
      <RouterLink to="/dashboard" class="bottom-nav-item">
        <svg class="bottom-nav-svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/>
        </svg>
        <span class="bottom-nav-label">{{ t('nav.dashboard') }}</span>
      </RouterLink>
      <RouterLink to="/environments" class="bottom-nav-item">
        <svg class="bottom-nav-svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/>
        </svg>
        <span class="bottom-nav-label">{{ t('nav.environments') }}</span>
      </RouterLink>
      <button class="bottom-nav-item bottom-nav-fab" @click="mobileMenuOpen = !mobileMenuOpen">
        <svg class="bottom-nav-svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="18" x2="21" y2="18"/>
        </svg>
      </button>
      <RouterLink to="/audit-log" class="bottom-nav-item">
        <svg class="bottom-nav-svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
        </svg>
        <span class="bottom-nav-label">{{ t('nav.auditLog') }}</span>
      </RouterLink>
      <RouterLink to="/settings" class="bottom-nav-item">
        <svg class="bottom-nav-svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
        </svg>
        <span class="bottom-nav-label">{{ t('nav.settings') }}</span>
      </RouterLink>
    </nav>

    <!-- Session timeout warning dialog -->
    <Modal :model-value="showWarning" :title="t('session.warningTitle')">
      <p class="session-warning-message">
        {{ t('session.warningMessage', { countdown: remainingSeconds }) }}
      </p>
      <template #footer>
        <div class="session-warning-actions">
          <Button variant="primary" @click="extendSession">
            {{ t('session.extend') }}
          </Button>
          <Button variant="danger" @click="$router.push('/login')">
            {{ t('session.logout') }}
          </Button>
        </div>
      </template>
    </Modal>
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

/* ═══════════ Sidebar ═══════════ */
.sidebar {
  width: var(--sidebar-width);
  flex-shrink: 0;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
}

/* ── Brand ── */
.sidebar-brand {
  height: var(--topbar-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--space-3) 0 var(--space-4);
  border-bottom: 1px solid var(--border);
}
.brand-left {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.brand-glyph {
  width: 28px;
  height: 28px;
  border-radius: 7px;
  background: linear-gradient(140deg, var(--accent), var(--brand-deep));
  color: var(--on-brand);
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-mono);
  font-weight: 800;
  font-size: 15px;
  flex-shrink: 0;
  box-shadow: 0 0 0 1px rgba(232,145,45,.4), 0 4px 14px rgba(232,145,45,.25);
}
.brand-name {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: 0.5px;
}
.brand-accent {
  color: var(--accent);
}
.brand-actions {
  display: flex;
  gap: var(--space-1);
}
.brand-btn {
  width: 30px;
  height: 30px;
  border-radius: 7px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: color var(--transition), background var(--transition);
}
.brand-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

/* ── Search ── */
.sidebar-search {
  padding: var(--space-2) var(--space-3);
}
.search-box {
  display: flex;
  align-items: center;
  height: 34px;
  padding: 0 var(--space-2) 0 var(--space-3);
  background: var(--bg-page);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  cursor: text;
  transition: border-color var(--transition);
}
.search-box:focus-within {
  border-color: var(--accent);
}
.search-icon {
  color: var(--text-muted);
  flex-shrink: 0;
  margin-right: var(--space-2);
}
.search-input {
  flex: 1;
  height: 100%;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-primary);
  font-size: 12.5px;
}
.search-input::placeholder {
  color: var(--text-muted);
}
.search-kbd {
  font-size: 10px;
  font-family: var(--font-mono);
  color: var(--text-muted);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 1px 5px;
  line-height: 1.4;
  flex-shrink: 0;
  pointer-events: none;
}

/* ── Nav ── */
.sidebar-nav {
  padding: var(--space-2) var(--space-3);
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius);
  color: var(--text-secondary);
  font-size: 13.5px;
  text-decoration: none;
  border: none;
  background: none;
  width: 100%;
  cursor: pointer;
  transition: background var(--transition), color var(--transition);
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
.nav-svg {
  flex-shrink: 0;
}

/* ── New resource button ── */
.sidebar-resource {
  border-top: 1px solid var(--border);
  flex: 1;
  min-height: 0;
}

/* ── Bottom buttons ── */
.sidebar-bottom {
  padding: var(--space-3);
  border-top: 1px solid var(--border);
}
.sidebar-bottom-buttons {
  display: flex;
  gap: var(--space-2);
}
.sidebar-action-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-1);
  height: 30px;
  font-size: 12.5px;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  cursor: pointer;
  text-decoration: none;
  transition: color var(--transition), background var(--transition), border-color var(--transition);
}
.sidebar-action-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
  border-color: var(--text-muted);
  text-decoration: none;
}
.sidebar-action-plus {
  font-weight: 700;
  font-size: 14px;
}

/* ═══════════ Main ═══════════ */
.main {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

/* ── Topbar ── */
.topbar {
  height: var(--topbar-height);
  flex-shrink: 0;
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 var(--space-4);
  background: var(--bg-page);
}
.topbar-breadcrumb {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-sm);
}
.breadcrumb-root {
  color: var(--text-muted);
}
.breadcrumb-sep {
  color: var(--text-muted);
  opacity: 0.5;
}
.breadcrumb-current {
  color: var(--text-primary);
  font-weight: 600;
}
.topbar-spacer {
  flex: 1;
}
.topbar-search-inline {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 34px;
  width: 260px;
  padding: 0 12px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-surface);
  color: var(--text-dim);
  font-size: 12.5px;
  cursor: pointer;
  transition: border-color var(--transition);
}
.topbar-search-inline:hover {
  border-color: var(--border-strong);
}
.topbar-search-kbd {
  font-size: 10px;
  font-family: var(--font-mono);
  color: var(--text-muted);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 1px 5px;
  line-height: 1.4;
  margin-left: auto;
  pointer-events: none;
}
.topbar-actions {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}
.topbar-icon-btn {
  width: 32px;
  height: 32px;
  border-radius: var(--radius);
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: color var(--transition), background var(--transition);
}
.topbar-icon-btn:hover {
  color: var(--accent);
  background: var(--bg-hover);
}
.topbar-avatar {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  background: linear-gradient(135deg, #3b82f6, #1d4ed8);
  color: #ffffff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  flex-shrink: 0;
}

.content {
  width: 100%;
  flex: 1;
  min-height: 0;
}

/* ── Hamburger (mobile) ── */
.hamburger-btn {
  display: none;
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  padding: var(--space-1) var(--space-2) var(--space-1) 0;
  margin-right: var(--space-2);
}
.mobile-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: 998;
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

/* ═══════════ Mobile FAB ═══════════ */
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

/* ═══════════ Bottom nav (mobile) ═══════════ */
.bottom-nav {
  display: none;
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  height: 56px;
  padding-bottom: env(safe-area-inset-bottom);
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
  font-size: 10px;
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
.bottom-nav-svg {
  width: 18px;
  height: 18px;
}
.bottom-nav-label {
  font-size: 10px;
  line-height: 1;
}

/* ═══════════ Session warning ═══════════ */
.session-warning-message {
  margin: 0;
  font-size: var(--text-sm);
  color: var(--text-secondary);
  line-height: 1.5;
}
.session-warning-actions {
  display: flex;
  gap: var(--space-3);
  justify-content: flex-end;
  width: 100%;
}

/* ═══════════ Responsive ═══════════ */
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
  .hamburger-btn {
    display: block;
  }
  .content {
    padding: var(--space-3);
    padding-bottom: 64px;
  }
  .mobile-fab {
    display: flex;
  }
  .bottom-nav {
    display: flex;
  }
  .content--keyboard-open {
    padding-bottom: var(--space-3);
  }
}
</style>
