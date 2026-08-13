<script setup lang="ts">
import { computed, inject } from 'vue'
import { PANE_CTX, type PaneCtx } from './paneContext'
import { useI18n } from 'vue-i18n'
import WorkspaceTerminal from '@/features/terminal/WorkspaceTerminal.vue'
import FilesDrawer from '@/features/files/FilesDrawer.vue'
import { defineAsyncComponent } from 'vue'

const SqlPage = defineAsyncComponent(() => import('@/features/sql/SqlPage.vue'))
const RedisPage = defineAsyncComponent(() => import('@/features/redis/RedisPage.vue'))
const FilesPage = defineAsyncComponent(() => import('@/features/files/FilesPage.vue'))

const props = defineProps<{ leafId: string }>()
const ctx = inject<PaneCtx>(PANE_CTX)!
const { t } = useI18n()

const leaf = computed(() => ctx.allLeaves.value.find((l) => l.id === props.leafId) ?? null)
const tabInfo = computed(() => (leaf.value?.tabId ? ctx.findTab(leaf.value.tabId) ?? null : null))

const statusMap = (s: string) =>
  s === 'online' ? 'connected' : s === 'connecting' ? 'connecting' : s === 'error' ? 'error' : 'disconnected'

function onStatus(tabId: string, status: string) {
  ctx.onTabStatusChange(tabId, statusMap(status))
}
</script>

<template>
  <div
    class="ws-pane"
    :class="{ 'ws-pane--active': leaf?.id === ctx.activePaneId.value, 'ws-pane--drag-over': ctx.dragOverPane.value === leaf?.id }"
    :title="t('workspace.dragHint')"
    @click="ctx.activePaneId.value = leaf?.id || ctx.activePaneId.value"
    @contextmenu="leaf?.id && ctx.onPaneContextMenu($event, leaf.id)"
    @dragover.prevent="leaf?.id && ctx.onPaneDragEnter(leaf.id)"
    @dragleave="leaf?.id && ctx.onPaneDragLeave(leaf.id)"
    @drop="leaf?.id && ctx.onPaneDrop($event, leaf.id)"
  >
    <div class="ws-pane-header mono">
      <span>{{ tabInfo?.label || t('workspace.noTabOpen') }}</span>
      <div class="ws-pane-actions">
        <button class="ws-pane-btn" :title="t('workspace.splitH')" @click.stop="leaf?.id && ctx.splitHorizontal(leaf.id)">⬌</button>
        <button class="ws-pane-btn" :title="t('workspace.splitV')" @click.stop="leaf?.id && ctx.splitVertical(leaf.id)">⬍</button>
        <button v-if="ctx.allLeaves.value.length > 1" class="ws-pane-btn" :title="t('workspace.closePane')" @click.stop="leaf?.id && ctx.closePane(leaf.id)">×</button>
      </div>
    </div>

    <!-- Terminal (SSH) + SFTP Drawer -->
    <div v-if="tabInfo?.protocol === 'ssh'" class="ws-ssh-area">
      <KeepAlive>
        <WorkspaceTerminal
          :key="tabInfo.id || ''"
          :tab-id="tabInfo.id || ''"
          :resource-id="tabInfo.resourceId || ''"
          :name="tabInfo.label || ''"
          :protocol="tabInfo.protocol"
          :theme="tabInfo.theme"
          :font-size="tabInfo.fontSize"
          :opacity="tabInfo.opacity"
          :cursor-style="tabInfo.cursorStyle"
          :cursor-blink="tabInfo.cursorBlink"
          :background-image="tabInfo.backgroundImage"
          @update:status="(s: string) => tabInfo?.id && onStatus(tabInfo.id, s)"
          @terminal-resize="ctx.onTerminalResize"
          @encoding-change="ctx.onEncodingChange"
          @toggle-sftp="ctx.toggleSftpDrawer"
        />
      </KeepAlive>
      <div v-if="ctx.showSftpDrawer.value" class="ws-sftp-drawer" :style="{ height: ctx.sftpDrawerHeight.value + 'px' }">
        <div class="ws-sftp-drag-handle" @mousedown.prevent="ctx.startSftpDrag" />
        <FilesDrawer :resource-id="tabInfo.resourceId" />
      </div>
    </div>

    <!-- SQL (MySQL / PostgreSQL / SQLite) -->
    <SqlPage
      v-else-if="['mysql', 'postgresql', 'sqlite'].includes(tabInfo?.protocol || '')"
      :key="tabInfo?.id || ''"
      :resource-id="tabInfo?.resourceId"
      :db-type="tabInfo?.protocol"
      @update:status="(s: string) => tabInfo?.id && onStatus(tabInfo.id, s)"
    />

    <!-- Redis -->
    <RedisPage
      v-else-if="tabInfo?.protocol === 'redis'"
      :key="tabInfo?.id || ''"
      :resource-id="tabInfo?.resourceId"
      @update:status="(s: string) => tabInfo?.id && onStatus(tabInfo.id, s)"
    />

    <!-- Files (SFTP / S3) -->
    <FilesPage
      v-else-if="['sftp', 's3'].includes(tabInfo?.protocol || '')"
      :key="tabInfo?.id || ''"
      :resource-id="tabInfo?.resourceId"
      :protocol="tabInfo?.protocol === 's3' ? 's3' : 'sftp'"
      @update:status="(s: string) => tabInfo?.id && onStatus(tabInfo.id, s)"
    />

    <!-- Empty state -->
    <div v-else class="ws-component-placeholder">
      <div class="ws-placeholder-text muted">
        {{ t('workspace.noConnectionDesc') }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.ws-pane {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-deep);
  overflow: hidden;
}
.ws-pane--active {
  outline: 1px solid var(--accent);
  outline-offset: -1px;
}
.ws-pane--drag-over {
  outline: 2px dashed var(--accent);
  outline-offset: -2px;
}
.ws-pane-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--statusbar-height);
  padding: 0 var(--space-2);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  cursor: pointer;
  user-select: none;
}
.ws-pane-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity var(--transition);
}
.ws-pane:hover .ws-pane-actions {
  opacity: 1;
}
.ws-pane-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  padding: 2px 4px;
  border-radius: var(--radius-sm);
}
.ws-pane-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.ws-ssh-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  position: relative;
}
.ws-sftp-drawer {
  border-top: 1px solid var(--border);
  background: var(--bg-page);
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.ws-sftp-drag-handle {
  height: 4px;
  cursor: ns-resize;
  background: transparent;
}
.ws-sftp-drag-handle:hover {
  background: var(--accent);
}
.ws-component-placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.ws-placeholder-text {
  font-size: var(--text-sm);
}
</style>
