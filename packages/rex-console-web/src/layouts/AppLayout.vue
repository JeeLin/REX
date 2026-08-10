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

const { t } = useI18n()
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
  { to: '/workspace', key: 'nav.workspace', icon: '▤' },
  { to: '/dashboard', key: 'nav.dashboard', icon: '◧' },
  { to: '/environments', key: 'nav.environments', icon: '⛁' },
  { to: '/agents', key: 'nav.agents', icon: '⬡' },
]

const bottomNav = [
  { to: '/audit-log', key: 'nav.auditLog', icon: '☰' },
  { to: '/settings', key: 'nav.settings', icon: '⚙' },
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
  return match ? t(match.key) : 'REX Hub'
})
</script>

<template>
  <div class="app-layout" :class="{ 'app-layout--fullscreen': fullscreen && isWorkspace }">
    <!-- 移动端菜单遮罩 -->
    <div v-if="mobileMenuOpen" class="mobile-overlay" @click="mobileMenuOpen = false" />

    <!-- 侧栏：工作区全屏时隐藏，移动端为抽屉 -->
    <aside v-if="!(fullscreen && isWorkspace)" class="sidebar" :class="{ 'sidebar--mobile-open': mobileMenuOpen }">
      <div class="sidebar-brand mono">
        REX<span class="accent">Hub</span>
      </div>
      <div class="sidebar-search">
        <input
          v-model="searchQuery"
          type="text"
          class="sidebar-search-input mono"
          :aria-label="t('common.search', 'Search')"
          :placeholder="t('common.search', 'Search…')"
        />
        <span class="sidebar-search-icon">⌕</span>
      </div>
      <nav class="sidebar-nav">
        <RouterLink
          v-for="item in mainNav"
          :key="item.to"
          :to="item.to"
          class="nav-item"
          @click="mobileMenuOpen = false"
        >
          <span class="nav-icon">{{ item.icon }}</span>
          <span class="nav-label">{{ t(item.key) }}</span>
        </RouterLink>
      </nav>

      <!-- 资源栏：嵌入侧栏，agents 和 audit-log 之间 -->
      <ResourcePanel v-if="!(fullscreen && isWorkspace)" class="sidebar-resource" @resource-properties="onResourceProperties" />

      <nav class="sidebar-nav sidebar-bottom">
        <RouterLink
          v-for="item in bottomNav"
          :key="item.to"
          :to="item.to"
          class="nav-item"
          @click="mobileMenuOpen = false"
        >
          <span class="nav-icon">{{ item.icon }}</span>
          <span class="nav-label">{{ t(item.key) }}</span>
        </RouterLink>
      </nav>
    </aside>

    <div ref="mainRef" class="main">
      <header v-if="!(fullscreen && isWorkspace)" class="topbar">
        <!-- 移动端汉堡按钮 -->
        <button class="hamburger-btn" :aria-label="t('common.menu', 'Menu')" @click="mobileMenuOpen = !mobileMenuOpen">☰</button>
        <span class="topbar-title mono">{{ currentTitle }}</span>
        <div class="topbar-actions">
          <button v-if="isWorkspace" class="fullscreen-btn mono" :aria-label="fullscreen ? t('common.exitFullscreen') : t('common.fullscreen')" :title="fullscreen ? t('common.exitFullscreen') : t('common.fullscreen')" @click="fullscreen = !fullscreen">
            {{ fullscreen ? '⊟' : '⊞' }}
          </button>
          <button class="logout-btn" :aria-label="t('common.logout', 'Logout')" title="退出登录" @click="sessionLogout">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
              <polyline points="16 17 21 12 16 7" />
              <line x1="21" y1="12" x2="9" y2="12" />
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

/* Sidebar */
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
  justify-content: center;
  padding: 0 var(--space-4);
  font-size: var(--text-lg);
  font-weight: 700;
  border-bottom: 1px solid var(--border);
  overflow: hidden;
  white-space: nowrap;
}
.sidebar-search {
  position: relative;
  padding: var(--space-2) var(--space-3);
}
.sidebar-search-input {
  width: 100%;
  height: 32px;
  padding: 0 var(--space-3) 0 var(--space-6);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
  transition: border-color var(--transition);
}
.sidebar-search-input:focus {
  border-color: var(--accent);
}
.sidebar-search-input::placeholder {
  color: var(--text-muted);
}
.sidebar-search-icon {
  position: absolute;
  left: calc(var(--space-3) + var(--space-2));
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-muted);
  font-size: var(--text-sm);
  pointer-events: none;
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

/* Main */
.main {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
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
.logout-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-base);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius);
  transition: color var(--transition), background var(--transition);
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.logout-btn:hover {
  color: var(--danger);
  background: var(--bg-hover);
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
  width: 100%;
  flex: 1;
  min-height: 0;
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
  .hamburger-btn {
    display: block;
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
  .content--keyboard-open {
    padding-bottom: var(--space-3);
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

/* Session timeout warning */
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
</style>
