<template>
  <div class="docker-console">
    <!-- 顶部状态栏 -->
    <div class="docker-topbar">
      <span class="docker-status-dot" :class="{ connected: session.connected.value }" />
      <span class="docker-topbar-label">Docker</span>
      <span class="docker-topbar-name">{{ resourceName }}</span>
      <span class="docker-topbar-state">
        {{ serverVersion }}
      </span>
      <div class="docker-topbar-spacer" />
      <button
        v-if="!session.connected.value"
        class="docker-btn docker-btn-connect"
        :disabled="connecting"
        @click="handleConnect"
      >
        {{ connecting ? t('docker.connecting') : t('docker.connect') }}
      </button>
      <button
        v-else
        class="docker-btn docker-btn-disconnect"
        @click="handleDisconnect"
      >
        {{ t('docker.disconnect') }}
      </button>
    </div>

    <!-- 错误提示 -->
    <div v-if="session.error.value" class="docker-error-banner">
      {{ session.error.value }}
    </div>

    <!-- 未连接欢迎页 -->
    <div v-if="!session.connected.value && !session.error.value" class="docker-welcome">
      <p>{{ t('docker.welcome') }}</p>
    </div>

    <!-- 主区域：容器列表 + 日志面板 -->
    <template v-else>
      <!-- 日志面板 -->
      <ContainerLogs
        v-if="logsContainer"
        :container-name="logsContainer.name"
        :logs="logsContent"
        :loading="logsLoading"
        :error-msg="logsError"
        @close="closeLogs"
      />

      <!-- 容器列表 -->
      <ContainerList
        v-else
        :containers="session.containers.value"
        :loading="containersLoading"
        :selected-id="selectedContainerId"
        @select="handleSelectContainer"
        @contextmenu="showContextMenu"
        @refresh="refreshContainers"
      />
    </template>

    <!-- 右键菜单 -->
    <Teleport to="body">
      <div
        v-if="contextMenu.visible"
        class="docker-ctx-menu"
        :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }"
      >
        <button class="docker-ctx-item" @click="ctxAction('start')">
          {{ t('docker.ctx.start') }}
        </button>
        <button class="docker-ctx-item" @click="ctxAction('stop')">
          {{ t('docker.ctx.stop') }}
        </button>
        <button class="docker-ctx-item" @click="ctxAction('restart')">
          {{ t('docker.ctx.restart') }}
        </button>
        <div class="docker-ctx-divider" />
        <button class="docker-ctx-item" @click="ctxAction('logs')">
          {{ t('docker.ctx.logs') }}
        </button>
        <button class="docker-ctx-item" @click="ctxAction('inspect')">
          {{ t('docker.ctx.inspect') }}
        </button>
        <div class="docker-ctx-divider" />
        <button class="docker-ctx-item docker-ctx-danger" @click="ctxAction('remove')">
          {{ t('docker.ctx.remove') }}
        </button>
      </div>
    </Teleport>

    <!-- Inspect 弹窗 -->
    <Teleport to="body">
      <div v-if="inspectData" class="docker-inspect-overlay" @click.self="inspectData = null">
        <div class="docker-inspect-modal">
          <div class="docker-inspect-header">
            <span>{{ t('docker.ctx.inspect') }} — {{ inspectContainerName }}</span>
            <button class="docker-btn docker-btn-sm" @click="inspectData = null">✕</button>
          </div>
          <pre class="docker-inspect-body">{{ inspectData }}</pre>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDockerSession } from './useDockerSession'
import ContainerList from './ContainerList.vue'
import ContainerLogs from './ContainerLogs.vue'
import type { DockerContainerInfo } from '@/api/docker'

const props = defineProps<{
  resourceId: string
  resourceName: string
}>()

const { t } = useI18n()

const session = useDockerSession(() => props.resourceId)

const connecting = ref(false)
const containersLoading = ref(false)
const selectedContainerId = ref<string | null>(null)

// ── 服务器版本 ────────────────────────────────────────────
const serverVersion = computed(() => {
  const info = session.serverInfo.value
  if (!info) return ''
  const ver = info['ServerVersion']
  return ver ? `Docker ${ver}` : ''
})

// ── 连接/断开 ────────────────────────────────────────────
async function handleConnect() {
  connecting.value = true
  try {
    await session.connect()
    await refreshContainers()
  } catch {
    // error is set in session.error
  } finally {
    connecting.value = false
  }
}

function handleDisconnect() {
  session.disconnect()
  selectedContainerId.value = null
  logsContainer.value = null
}

// ── 容器列表 ──────────────────────────────────────────────
async function refreshContainers() {
  if (!session.connected.value) return
  containersLoading.value = true
  try {
    await session.listContainers(true)
  } catch {
    // error surfaced via session.error
  } finally {
    containersLoading.value = false
  }
}

function handleSelectContainer(c: DockerContainerInfo) {
  selectedContainerId.value = selectedContainerId.value === c.id ? null : c.id
}

// ── 右键菜单 ──────────────────────────────────────────────
const contextMenu = reactive({
  visible: false,
  x: 0,
  y: 0,
  container: null as DockerContainerInfo | null,
})

function showContextMenu(event: MouseEvent, container: DockerContainerInfo) {
  contextMenu.container = container
  contextMenu.x = event.clientX
  contextMenu.y = event.clientY
  contextMenu.visible = true
}

function hideContextMenu() {
  contextMenu.visible = false
  contextMenu.container = null
}

async function ctxAction(action: string) {
  const c = contextMenu.container
  hideContextMenu()
  if (!c) return

  switch (action) {
    case 'start':
      await session.startContainer(c.id)
      break
    case 'stop':
      await session.stopContainer(c.id)
      break
    case 'restart':
      await session.restartContainer(c.id)
      break
    case 'remove':
      await session.removeContainer(c.id)
      break
    case 'logs':
      openLogs(c)
      break
    case 'inspect':
      openInspect(c)
      break
  }
  // 刷新列表
  if (['start', 'stop', 'restart', 'remove'].includes(action)) {
    await refreshContainers()
  }
}

// ── 日志 ──────────────────────────────────────────────────
const logsContainer = ref<DockerContainerInfo | null>(null)
const logsContent = ref<string | null>(null)
const logsLoading = ref(false)
const logsError = ref<string | null>(null)

async function openLogs(c: DockerContainerInfo) {
  logsContainer.value = c
  logsContent.value = null
  logsLoading.value = true
  logsError.value = null
  try {
    logsContent.value = await session.getLogs(c.id, 200)
  } catch (err) {
    logsError.value = err instanceof Error ? err.message : String(err)
  } finally {
    logsLoading.value = false
  }
}

function closeLogs() {
  logsContainer.value = null
  logsContent.value = null
  logsError.value = null
}

// ── Inspect ───────────────────────────────────────────────
const inspectData = ref<string | null>(null)
const inspectContainerName = ref('')

async function openInspect(c: DockerContainerInfo) {
  inspectContainerName.value = c.name
  try {
    const data = await session.inspectContainer(c.id)
    inspectData.value = JSON.stringify(data, null, 2)
  } catch (err) {
    inspectData.value = `Error: ${err instanceof Error ? err.message : String(err)}`
  }
}

// ── 全局点击关闭菜单 ──────────────────────────────────────
function onGlobalClick() {
  if (contextMenu.visible) {
    hideContextMenu()
  }
}

document.addEventListener('click', onGlobalClick)

// ── 自动连接 ─────────────────────────────────────────────
onMounted(() => {
  if (!session.connected.value) {
    session.connect().then(() => refreshContainers()).catch(() => {})
  }
})

onUnmounted(() => {
  document.removeEventListener('click', onGlobalClick)
})
</script>

<style scoped>
.docker-console {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
}

/* 顶部状态栏 */
.docker-topbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  font-size: 13px;
  flex-shrink: 0;
}

.docker-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #f85149;
}

.docker-status-dot.connected { background: #3fb950; }

.docker-topbar-label { font-weight: 600; color: #2496ed; }
.docker-topbar-name { color: var(--text-secondary); }
.docker-topbar-state { color: var(--text-secondary); font-size: 12px; }
.docker-topbar-spacer { flex: 1; }

/* 按钮 */
.docker-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-primary);
  padding: 3px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  white-space: nowrap;
}

.docker-btn:hover:not(:disabled) { background: var(--bg-hover); }
.docker-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.docker-btn-connect { border-color: #3fb950; color: #3fb950; }
.docker-btn-disconnect { border-color: #f85149; color: #f85149; }
.docker-btn-sm { padding: 2px 8px; }

/* 错误 */
.docker-error-banner {
  color: #f85149;
  padding: 8px 12px;
  border-bottom: 1px solid #f8514933;
  background: #f8514911;
  font-size: 12px;
  flex-shrink: 0;
}

/* 欢迎 */
.docker-welcome {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  font-size: 14px;
  font-family: inherit;
}

/* 右键菜单 */
.docker-ctx-menu {
  position: fixed;
  z-index: 9999;
  background: var(--bg-surface, #1c2128);
  border: 1px solid var(--border-primary);
  border-radius: 6px;
  padding: 4px 0;
  min-width: 160px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}

.docker-ctx-item {
  display: block;
  width: 100%;
  padding: 6px 12px;
  background: none;
  border: none;
  color: var(--text-primary);
  font-size: 12px;
  text-align: left;
  cursor: pointer;
  font-family: inherit;
}

.docker-ctx-item:hover { background: var(--bg-hover); }

.docker-ctx-danger { color: #f85149; }
.docker-ctx-danger:hover { background: #f8514922; }

.docker-ctx-divider {
  height: 1px;
  background: var(--border-primary);
  margin: 4px 0;
}

/* Inspect 弹窗 */
.docker-inspect-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
}

.docker-inspect-modal {
  background: var(--bg-surface, #1c2128);
  border: 1px solid var(--border-primary);
  border-radius: 8px;
  width: 80vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.docker-inspect-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-primary);
  font-size: 13px;
  font-weight: 600;
}

.docker-inspect-body {
  flex: 1;
  overflow: auto;
  margin: 0;
  padding: 16px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
