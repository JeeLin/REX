<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useFavoritesStore } from '@/stores/favorites'
import { PROTOCOL_ICONS } from '@/features/resource/protocols'

const { t } = useI18n()
const router = useRouter()
const favoritesStore = useFavoritesStore()

const recentItems = computed(() => favoritesStore.recent.slice(0, 5))

const shortcuts = [
  { keys: 'Ctrl+K', desc: 'Command Palette' },
  { keys: 'Ctrl+Shift+F', desc: 'Global Search' },
  { keys: 'Ctrl+T', desc: 'New Tab' },
  { keys: 'Ctrl+W', desc: 'Close Tab' },
  { keys: 'Ctrl+← / Ctrl+→', desc: 'Tab History' },
  { keys: 'Ctrl+Shift+T', desc: 'Reopen Closed Tab' },
  { keys: 'Ctrl+\\', desc: 'Split Horizontal' },
  { keys: 'Alt+1~5', desc: 'Layout Presets' },
]

function openRecent(item: { id: string; name: string; protocol: string }) {
  // Emit through workspace store
  import('@/stores/workspace').then(({ useWorkspaceStore }) => {
    const wsStore = useWorkspaceStore()
    wsStore.openResource({
      id: item.id,
      name: item.name,
      protocol: item.protocol,
    })
  })
}
</script>

<template>
  <div class="welcome-page">
    <div class="welcome-content">
      <!-- Header -->
      <div class="welcome-header">
        <div class="welcome-logo">
          <span class="welcome-logo-icon">⬡</span>
        </div>
        <h1 class="welcome-title">REX Hub</h1>
        <p class="welcome-version mono">v0.73.0</p>
      </div>

      <!-- Quick Actions -->
      <div class="welcome-section">
        <h2 class="welcome-section-title">{{ t('welcome.quickActions', 'Quick Actions') }}</h2>
        <div class="welcome-actions">
          <button class="welcome-action-btn" @click="router.push('/environments')">
            <span class="welcome-action-icon">➕</span>
            <span class="welcome-action-label">{{ t('welcome.newConnection', 'New Connection') }}</span>
          </button>
          <button class="welcome-action-btn" @click="router.push('/environments')">
            <span class="welcome-action-icon">🌳</span>
            <span class="welcome-action-label">{{ t('welcome.connectionTree', 'Connection Tree') }}</span>
          </button>
        </div>
      </div>

      <!-- Recent -->
      <div v-if="recentItems.length > 0" class="welcome-section">
        <h2 class="welcome-section-title">{{ t('welcome.recent', 'Recent') }}</h2>
        <div class="welcome-recent-list">
          <button
            v-for="item in recentItems"
            :key="item.id"
            class="welcome-recent-item"
            @click="openRecent(item)"
          >
            <span class="welcome-recent-icon">{{ PROTOCOL_ICONS[item.protocol] || '?' }}</span>
            <span class="welcome-recent-name">{{ item.name }}</span>
            <span class="welcome-recent-protocol mono">{{ item.protocol.toUpperCase() }}</span>
          </button>
        </div>
      </div>

      <!-- Keyboard Shortcuts -->
      <div class="welcome-section">
        <h2 class="welcome-section-title">{{ t('welcome.shortcuts', 'Keyboard Shortcuts') }}</h2>
        <div class="welcome-shortcuts-grid">
          <div v-for="s in shortcuts" :key="s.keys" class="welcome-shortcut-row">
            <kbd class="welcome-shortcut-keys mono">{{ s.keys }}</kbd>
            <span class="welcome-shortcut-desc">{{ s.desc }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.welcome-page {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-deep);
  z-index: 1;
  overflow-y: auto;
}

.welcome-content {
  width: 100%;
  max-width: 520px;
  padding: var(--space-6);
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
}

.welcome-header {
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
}

.welcome-logo {
  width: 64px;
  height: 64px;
  display: grid;
  place-items: center;
  border-radius: var(--radius-lg);
  background: var(--accent-soft);
  margin-bottom: var(--space-2);
}

.welcome-logo-icon {
  font-size: 32px;
  color: var(--accent);
}

.welcome-title {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.welcome-version {
  font-size: var(--text-xs);
  color: var(--text-muted);
}

.welcome-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.welcome-section-title {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0;
}

.welcome-actions {
  display: flex;
  gap: var(--space-3);
}

.welcome-action-btn {
  flex: 1;
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: background var(--transition), border-color var(--transition);
}

.welcome-action-btn:hover {
  background: var(--bg-hover);
  border-color: var(--accent);
}

.welcome-action-icon {
  font-size: 18px;
}

.welcome-action-label {
  font-weight: 500;
}

/* Recent */
.welcome-recent-list {
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: var(--border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.welcome-recent-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  background: var(--bg-elevated);
  color: var(--text-primary);
  font-size: var(--text-sm);
  cursor: pointer;
  border: none;
  text-align: left;
  transition: background var(--transition);
}

.welcome-recent-item:hover {
  background: var(--bg-hover);
}

.welcome-recent-icon {
  font-size: 14px;
  opacity: 0.7;
}

.welcome-recent-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.welcome-recent-protocol {
  font-size: var(--text-xs);
  color: var(--text-muted);
}

/* Shortcuts */
.welcome-shortcuts-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-2);
}

.welcome-shortcut-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.welcome-shortcut-keys {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 100px;
  padding: 2px 6px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  text-align: center;
}

.welcome-shortcut-desc {
  font-size: var(--text-xs);
  color: var(--text-muted);
}
</style>
