<template>
  <div class="workspace-shell">
    <!-- 标签栏 -->
    <TabBar :panel-count="panelCount" :drag-id="dragId" @update:drag-id="dragId = $event" @newConnection="showConnMenu = true" @dblclick="handleTabDblclick">
      <template #right>
        <div class="layout-indicator" @click="cycleLayout" :title="`${t('ws.layout.switch')} (Alt+1~5)`">
          <span>{{ layoutIcon }}</span>
          <span class="layout-label">{{ layoutLabel }}</span>
        </div>
        <button class="ws-tab-add" @click="toggleFullscreen" title="全屏 (F11)" style="font-size: 13px">⛶</button>
        <button class="ws-tab-add" @click="showShortcutsPanel = true" title="快捷键 (F1)" style="font-size: 12px">⌨</button>
      </template>
    </TabBar>

    <!-- 内容区 -->
    <div class="ws-content" :class="layoutClass">
      <!-- 空状态 -->
      <div v-if="tabs.length === 0" class="ws-empty">
        <div class="ws-empty-icon">⊞</div>
        <div class="ws-empty-text">{{ t('ws.empty.noSessions') }}</div>
        <div class="ws-empty-hint">
          从侧边栏选择资源开始连接<br />
          按 <kbd>Ctrl</kbd>+<kbd>N</kbd> 新建连接 · <kbd>Alt</kbd>+<kbd>1~5</kbd> 切换布局 · <kbd>F1</kbd> 快捷键
        </div>
      </div>

      <!-- 面板区域 -->
      <template v-else>
        <div
          v-for="i in panelCount"
          :key="i"
          class="ws-panel"
          :class="{
            active: isPanelActive(i - 1),
            'layout-drop-zone': dragId && dragOverPanel === i - 1 && currentLayout !== 'single'
          }"
          @dragover="onPanelDragOver($event, i - 1)"
          @dragleave="onPanelDragLeave"
          @drop="onPanelDrop($event, i - 1)"
        >
          <template v-if="getPanelTab(i - 1)">
            <WorkspaceTerminal
              v-if="getPanelTab(i - 1)!.component === 'terminal'"
              :key="getPanelTab(i - 1)!.id"
              :resource-id="getPanelTab(i - 1)!.resourceId"
              :resource-name="getPanelTab(i - 1)!.name"
              :connection-mode="getConnectionMode(getPanelTab(i - 1)!.resourceId)"
              @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
              @error="(msg: string) => onPanelError(getPanelTab(i - 1)!.id, msg)"
            />
            <WorkspaceSql
              v-else-if="getPanelTab(i - 1)!.component === 'sql'"
              :key="getPanelTab(i - 1)!.id"
              :resource-id="getPanelTab(i - 1)!.resourceId"
              :resource-name="getPanelTab(i - 1)!.name"
              :protocol="getPanelTab(i - 1)!.proto"
              @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
              @error="(msg: string) => onPanelError(getPanelTab(i - 1)!.id, msg)"
            />
            <WorkspaceFiles
              v-else-if="getPanelTab(i - 1)!.component === 'files'"
              :key="getPanelTab(i - 1)!.id"
              :resource-id="getPanelTab(i - 1)!.resourceId"
              :resource-name="getPanelTab(i - 1)!.name"
              @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
              @error="(msg: string) => onPanelError(getPanelTab(i - 1)!.id, msg)"
            />
            <div v-else class="panel-unsupported">
              <span :style="{ color: getProtocolIcon(getPanelTab(i - 1)!.proto).color }">{{ getProtocolIcon(getPanelTab(i - 1)!.proto).icon }}</span>
              <div class="panel-unsupported-text">{{ getPanelTab(i - 1)!.name }}</div>
              <div class="panel-unsupported-hint">暂不支持 {{ getPanelTab(i - 1)!.proto.toUpperCase() }} 协议</div>
            </div>
          </template>
          <div v-else class="panel-empty">
            <span class="panel-empty-text">面板 {{ i }}</span>
          </div>
        </div>
      </template>
    </div>

    <!-- 连接菜单 -->
    <div v-if="showConnMenu" class="conn-menu-overlay" @click="showConnMenu = false">
      <div class="conn-menu" @click.stop>
        <div class="conn-menu-search">
          <span class="search-icon">⌕</span>
          <input
            ref="connSearchInput"
            v-model="connSearchQuery"
            type="text"
            :placeholder="t('ws.conn.searchPlaceholder')"
            @keydown.esc="showConnMenu = false"
            @keydown.up.prevent="selectPrevResource"
            @keydown.down.prevent="selectNextResource"
            @keydown.enter="connectSelected"
          />
          <span class="search-shortcut">Esc</span>
        </div>
        <div class="conn-menu-list">
          <template v-for="(items, env) in groupedResources" :key="env">
            <div class="conn-menu-group-label">{{ env }}</div>
            <div
              v-for="(res, ri) in items"
              :key="res.id"
              class="conn-menu-item"
              :class="{ selected: selectedResourceIdx === getGlobalIndex(env as string, ri) }"
              @click="connectToResource(res)"
              @mouseenter="selectedResourceIdx = getGlobalIndex(env as string, ri)"
            >
              <div class="cmi-icon" :style="{ background: getProtocolIcon(res.protocol).color + '15', color: getProtocolIcon(res.protocol).color }">
                {{ getProtocolIcon(res.protocol).icon }}
              </div>
              <div class="cmi-info">
                <div class="cmi-name">{{ res.name }}</div>
                <div class="cmi-meta">{{ res.address }}</div>
              </div>
              <span class="cmi-proto">{{ res.protocol.toUpperCase() }}</span>
            </div>
          </template>
          <div v-if="flatFilteredResources.length === 0" class="conn-menu-empty">没有匹配的资源</div>
        </div>
        <div class="conn-menu-footer">
          <span><kbd>↑↓</kbd> 选择 · <kbd>↵</kbd> 连接</span>
          <span><kbd>Esc</kbd> 关闭</span>
        </div>
      </div>
    </div>

    <!-- 快捷键面板 -->
    <div v-if="showShortcutsPanel" class="shortcuts-overlay" @click="showShortcutsPanel = false">
      <div class="shortcuts-card" @click.stop>
        <h2>⌨ 快捷键</h2>
        <div class="shortcut-group">
          <div class="shortcut-group-title">标签页</div>
          <div class="shortcut-row"><span class="desc">新建连接</span><span class="keys"><kbd>Ctrl</kbd><span class="key-plus">+</span><kbd>N</kbd></span></div>
          <div class="shortcut-row"><span class="desc">关闭当前标签</span><span class="keys"><kbd>Ctrl</kbd><span class="key-plus">+</span><kbd>W</kbd></span></div>
          <div class="shortcut-row"><span class="desc">切换下一个标签</span><span class="keys"><kbd>Ctrl</kbd><span class="key-plus">+</span><kbd>Tab</kbd></span></div>
          <div class="shortcut-row"><span class="desc">切换上一个标签</span><span class="keys"><kbd>Ctrl</kbd><span class="key-plus">+</span><kbd>Shift</kbd><span class="key-plus">+</span><kbd>Tab</kbd></span></div>
          <div class="shortcut-row"><span class="desc">切换到标签 1-9</span><span class="keys"><kbd>Ctrl</kbd><span class="key-plus">+</span><kbd>1~9</kbd></span></div>
        </div>
        <div class="shortcut-group">
          <div class="shortcut-group-title">布局</div>
          <div class="shortcut-row"><span class="desc">单面板</span><span class="keys"><kbd>Alt</kbd><span class="key-plus">+</span><kbd>1</kbd></span></div>
          <div class="shortcut-row"><span class="desc">左右分屏</span><span class="keys"><kbd>Alt</kbd><span class="key-plus">+</span><kbd>2</kbd></span></div>
          <div class="shortcut-row"><span class="desc">上下分屏</span><span class="keys"><kbd>Alt</kbd><span class="key-plus">+</span><kbd>3</kbd></span></div>
          <div class="shortcut-row"><span class="desc">四宫格</span><span class="keys"><kbd>Alt</kbd><span class="key-plus">+</span><kbd>4</kbd></span></div>
          <div class="shortcut-row"><span class="desc">主+侧边</span><span class="keys"><kbd>Alt</kbd><span class="key-plus">+</span><kbd>5</kbd></span></div>
        </div>
        <div class="shortcut-group">
          <div class="shortcut-group-title">其他</div>
          <div class="shortcut-row"><span class="desc">全屏</span><span class="keys"><kbd>F11</kbd></span></div>
          <div class="shortcut-row"><span class="desc">快捷键帮助</span><span class="keys"><kbd>F1</kbd></span></div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { getProtocolIcon } from '@/composables/useProtocol'
import { listEnvsWithResources } from '@/api/env'
import TabBar from '@/features/workspace/TabBar.vue'
import { useTabs } from '@/features/workspace/useTabs'
import WorkspaceTerminal from '@/features/workspace/panels/WorkspaceTerminal.vue'
import WorkspaceSql from '@/features/workspace/panels/WorkspaceSql.vue'
import WorkspaceFiles from '@/features/workspace/panels/WorkspaceFiles.vue'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

// ── Tabs ──
const { tabs, activeTabId, addTab, closeTab, nextTab, prevTab, switchTabByIndex, moveTabToPanel, swapPanels } = useTabs()

// ── Layout ──
type Layout = 'single' | 'left-right' | 'top-bottom' | 'quad' | 'sidebar-main'
const LAYOUT_ORDER: Layout[] = ['single', 'left-right', 'top-bottom', 'quad', 'sidebar-main']
const LAYOUT_PANELS: Record<Layout, number> = {
  single: 1, 'left-right': 2, 'top-bottom': 2, quad: 4, 'sidebar-main': 2,
}
const LAYOUT_ICONS: Record<Layout, string> = {
  single: '☐', 'left-right': '◧', 'top-bottom': '▤', quad: '⊞', 'sidebar-main': '◫',
}
const LAYOUT_LABELS: Record<Layout, string> = {
  single: '单面板', 'left-right': '左右分屏', 'top-bottom': '上下分屏', quad: '四宫格', 'sidebar-main': '主+侧边',
}

const currentLayout = ref<Layout>('single')
const panelCount = computed(() => LAYOUT_PANELS[currentLayout.value])
const layoutIcon = computed(() => LAYOUT_ICONS[currentLayout.value])
const layoutLabel = computed(() => LAYOUT_LABELS[currentLayout.value])
const layoutClass = computed(() => {
  if (currentLayout.value === 'single') return 'layout-single'
  return `layout-split layout-${currentLayout.value}`
})

function setLayout(layout: Layout) {
  currentLayout.value = layout
}
function cycleLayout() {
  const idx = LAYOUT_ORDER.indexOf(currentLayout.value)
  currentLayout.value = LAYOUT_ORDER[(idx + 1) % LAYOUT_ORDER.length]
}
function isPanelActive(panelIndex: number): boolean {
  if (currentLayout.value === 'single') {
    return tabs.value.some((t) => t.id === activeTabId.value)
  }
  return tabs.value.some((t) => t.panelIndex === panelIndex)
}
function getPanelTab(panelIndex: number) {
  if (currentLayout.value === 'single') {
    return tabs.value.find((t) => t.id === activeTabId.value) ?? null
  }
  return tabs.value.find((t) => t.panelIndex === panelIndex) ?? null
}

// ── Panel Drag-and-Drop ──
const dragId = ref<string | null>(null)
const dragOverPanel = ref<number | null>(null)

function onPanelDragOver(e: DragEvent, panelIndex: number) {
  if (currentLayout.value === 'single' || !dragId.value) return
  e.preventDefault()
  dragOverPanel.value = panelIndex
}

function onPanelDragLeave() {
  dragOverPanel.value = null
}

function onPanelDrop(e: DragEvent, targetPanelIndex: number) {
  e.preventDefault()
  dragOverPanel.value = null
  if (!dragId.value || currentLayout.value === 'single') return
  const draggedTab = tabs.value.find((t) => t.id === dragId.value)
  if (!draggedTab) return
  const existingTab = tabs.value.find((t) => t.panelIndex === targetPanelIndex && t.id !== dragId.value)
  if (existingTab) {
    swapPanels(dragId.value, existingTab.id)
  } else {
    moveTabToPanel(dragId.value, targetPanelIndex)
  }
}

// ── Double-click split ──
function handleTabDblclick(tabId: string) {
  if (currentLayout.value !== 'single') return
  currentLayout.value = 'left-right'
  moveTabToPanel(tabId, 0)
  const currentIdx = tabs.value.findIndex((t) => t.id === tabId)
  const candidate = tabs.value.find((t, i) => i !== currentIdx && t.panelIndex !== 0)
  if (candidate) {
    moveTabToPanel(candidate.id, 1)
  } else if (tabs.value.length > 1) {
    const fallback = tabs.value[(currentIdx + 1) % tabs.value.length]
    if (fallback.id !== tabId) moveTabToPanel(fallback.id, 1)
  }
}

// ── Connection Menu ──
const showConnMenu = ref(false)
const connSearchQuery = ref('')
const connSearchInput = ref<HTMLInputElement | null>(null)
const selectedResourceIdx = ref(0)

interface Resource { id: string; name: string; address: string; protocol: string; envName: string; connectionMode: 'direct' | 'agent' }

const envsWithRes = ref<any[]>([])

const flatFilteredResources = computed<Resource[]>(() => {
  const all: Resource[] = []
  for (const env of envsWithRes.value) {
    for (const r of env.resources) {
      const q = connSearchQuery.value.toLowerCase()
      if (!q || r.name.toLowerCase().includes(q) || r.protocol.includes(q)) {
        all.push({ id: r.id, name: r.name, address: r.address || '', protocol: r.protocol, envName: env.name, connectionMode: (env.connection_mode === 'agent' ? 'agent' : 'direct') as 'direct' | 'agent' })
      }
    }
  }
  return all
})

const groupedResources = computed(() => {
  const groups: Record<string, Resource[]> = {}
  for (const r of flatFilteredResources.value) {
    if (!groups[r.envName]) groups[r.envName] = []
    groups[r.envName].push(r)
  }
  return groups
})

function getGlobalIndex(envName: string, localIndex: number): number {
  let idx = 0
  for (const [env, items] of Object.entries(groupedResources.value)) {
    if (env === envName) return idx + localIndex
    idx += items.length
  }
  return 0
}

function selectNextResource() {
  const total = flatFilteredResources.value.length
  if (total === 0) return
  selectedResourceIdx.value = (selectedResourceIdx.value + 1) % total
}
function selectPrevResource() {
  const total = flatFilteredResources.value.length
  if (total === 0) return
  selectedResourceIdx.value = (selectedResourceIdx.value - 1 + total) % total
}
function connectSelected() {
  const res = flatFilteredResources.value[selectedResourceIdx.value]
  if (res) connectToResource(res)
}
function connectToResource(res: Resource) {
  addTab(res.name, res.protocol as any, res.id)
  showConnMenu.value = false
  connSearchQuery.value = ''
  selectedResourceIdx.value = 0
}

function getConnectionMode(resourceId: string): 'direct' | 'agent' {
  for (const env of envsWithRes.value) {
    const r = env.resources?.find((x: any) => x.id === resourceId)
    if (r) return env.connection_mode === 'agent' ? 'agent' : 'direct'
  }
  return 'direct'
}

// ── Panel lifecycle ──
function onPanelDisconnect(tabId: string) {
  const tab = tabs.value.find((t) => t.id === tabId)
  if (tab) tab.status = 'offline'
}

function onPanelError(tabId: string, _msg: string) {
  const tab = tabs.value.find((t) => t.id === tabId)
  if (tab) tab.status = 'offline'
}

watch(showConnMenu, (val) => {
  if (val) {
    nextTick(() => connSearchInput.value?.focus())
  }
})

watch(connSearchQuery, () => {
  selectedResourceIdx.value = 0
})

onMounted(async () => {
  try { envsWithRes.value = await listEnvsWithResources() } catch { /* */ }
  window.addEventListener('keydown', onKeyDown)

  // 从路由 query 读取待打开的资源
  const openId = route.query.open as string
  if (openId) {
    const openName = (route.query.name as string) || openId
    const openProto = (route.query.proto as string) || 'ssh'
    addTab(openName, openProto as any, openId)
    // 清除 query 参数，避免刷新重复打开
    router.replace({ name: 'workspace' })
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKeyDown)
})

// ── Shortcuts Panel ──
const showShortcutsPanel = ref(false)

// ── Fullscreen ──
function toggleFullscreen() {
  if (document.fullscreenElement) {
    document.exitFullscreen()
  } else {
    document.documentElement.requestFullscreen()
  }
}

// ── Listen for sidebar resource clicks ──
function onOpenInWorkspace(e: Event) {
  const detail = (e as CustomEvent).detail
  if (detail?.resource) {
    addTab(detail.resource.name, detail.resource.protocol, detail.resource.id)
  }
}

onMounted(() => {
  window.addEventListener('open-in-workspace', onOpenInWorkspace)
})
onUnmounted(() => {
  window.removeEventListener('open-in-workspace', onOpenInWorkspace)
})

// ── Global Keyboard Shortcuts ──
function onKeyDown(e: KeyboardEvent) {
  const tag = (e.target as HTMLElement).tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA') return

  const ctrl = e.ctrlKey || e.metaKey

  if (ctrl && e.key === 'n') {
    e.preventDefault()
    showConnMenu.value = true
  } else if (ctrl && e.key === 'w') {
    e.preventDefault()
    if (activeTabId.value) {
      closeTab(activeTabId.value)
    }
  } else if (ctrl && !e.shiftKey && e.key === 'Tab') {
    e.preventDefault()
    nextTab()
  } else if (ctrl && e.shiftKey && e.key === 'Tab') {
    e.preventDefault()
    prevTab()
  } else if (ctrl && e.key >= '1' && e.key <= '9') {
    e.preventDefault()
    switchTabByIndex(parseInt(e.key) - 1)
  } else if (e.altKey && e.key >= '1' && e.key <= '5') {
    e.preventDefault()
    const layouts: Layout[] = ['single', 'left-right', 'top-bottom', 'quad', 'sidebar-main']
    setLayout(layouts[parseInt(e.key) - 1])
  } else if (e.key === 'F11') {
    e.preventDefault()
    toggleFullscreen()
  } else if (e.key === 'F1') {
    e.preventDefault()
    showShortcutsPanel.value = !showShortcutsPanel.value
  } else if (e.key === 'Escape') {
    if (showConnMenu.value) showConnMenu.value = false
    else if (showShortcutsPanel.value) showShortcutsPanel.value = false
  }
}
</script>

<style scoped>
/* ── Shell ── */
.workspace-shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

/* ── Content Area ── */
.ws-content {
  flex: 1;
  overflow: hidden;
  position: relative;
}

.ws-content.layout-single .ws-panel {
  position: absolute;
  inset: 0;
  display: none;
  flex-direction: column;
}

.ws-content.layout-single .ws-panel.active {
  display: flex;
}

.ws-content.layout-split {
  display: grid;
  gap: 1px;
  background: var(--border);
}

.ws-content.layout-split .ws-panel {
  position: relative;
  display: none;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-deep);
  min-height: 0;
}

.ws-content.layout-split .ws-panel.active {
  display: flex;
}

.ws-content.layout-split .ws-panel.layout-drop-zone {
  border: 2px dashed var(--accent);
  background: rgba(232, 145, 45, 0.06);
  box-shadow: inset 0 0 20px rgba(232, 145, 45, 0.08);
}

.ws-content.layout-split.layout-left-right {
  grid-template-columns: 1fr 1fr;
}

.ws-content.layout-split.layout-top-bottom {
  grid-template-rows: 1fr 1fr;
}

.ws-content.layout-split.layout-quad {
  grid-template-columns: 1fr 1fr;
  grid-template-rows: 1fr 1fr;
}

.ws-content.layout-split.layout-sidebar-main {
  grid-template-columns: 2fr 1fr;
}

.panel-content {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.panel-placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-sm);
  font-family: var(--font-mono);
  font-size: var(--fs-lg);
  color: var(--text-secondary);
}

.panel-status {
  font-size: var(--fs-xs);
  padding: 1px 6px;
  border-radius: 3px;
  background: var(--bg-surface);
}

.panel-status.online { color: var(--success); }
.panel-status.offline { color: var(--text-muted); }
.panel-status.connecting { color: var(--accent); }

.panel-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px dashed var(--border);
  margin: var(--sp-sm);
  border-radius: var(--radius-md);
}

.panel-empty-text {
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.panel-unsupported {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--sp-sm);
  color: var(--text-muted);
}

.panel-unsupported span:first-child {
  font-size: 32px;
  opacity: 0.4;
}

.panel-unsupported-text {
  font-family: var(--font-mono);
  font-weight: 500;
  font-size: var(--fs-sm);
  color: var(--text-secondary);
}

.panel-unsupported-hint {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

/* ── Empty State ── */
.ws-empty {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  gap: var(--sp-lg);
}

.ws-empty-icon {
  font-size: 48px;
  opacity: 0.3;
  filter: drop-shadow(0 0 12px rgba(232, 145, 45, 0.15));
}

.ws-empty-text {
  font-size: var(--fs-md);
}

.ws-empty-hint {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  max-width: 360px;
  text-align: center;
  line-height: 1.6;
}

.ws-empty kbd {
  background: var(--bg-surface);
  padding: 1px 6px;
  border-radius: 3px;
  border: 1px solid var(--border);
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
}

/* ── Layout Indicator ── */
.layout-indicator {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  background: var(--bg-elevated);
  cursor: pointer;
  transition: all var(--transition-fast);
  font-family: var(--font-mono);
}

.layout-indicator:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.layout-label {
  font-size: 11px;
}

/* ── Connection Menu ── */
.conn-menu-overlay {
  position: fixed;
  inset: 0;
  z-index: 400;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 100px;
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: blur(4px);
}

.conn-menu {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  width: 420px;
  max-width: 90vw;
  box-shadow: var(--shadow-lg), var(--phosphor-shadow);
  overflow: hidden;
  animation: modalIn 0.15s ease;
}

.conn-menu-search {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-md) var(--sp-lg);
  border-bottom: 1px solid var(--border);
}

.conn-menu-search input {
  flex: 1;
  border: none;
  background: none;
  outline: none;
  color: var(--text-primary);
  font-size: var(--fs-base);
  font-family: var(--font-body);
}

.conn-menu-search input::placeholder {
  color: var(--text-muted);
}

.search-icon {
  color: var(--text-muted);
  font-size: var(--fs-md);
}

.search-shortcut {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  padding: 2px 6px;
  border: 1px solid var(--border);
  border-radius: 3px;
  font-family: var(--font-mono);
}

.conn-menu-list {
  max-height: 320px;
  overflow-y: auto;
  padding: var(--sp-sm);
}

.conn-menu-group-label {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  padding: var(--sp-sm) var(--sp-md) var(--sp-xs);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
}

.conn-menu-item {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
  padding: var(--sp-sm) var(--sp-md);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.conn-menu-item:hover {
  background: var(--bg-hover);
}

.conn-menu-item.selected {
  background: rgba(232, 145, 45, 0.1);
}

.cmi-icon {
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: var(--fs-xs);
  flex-shrink: 0;
}

.cmi-info {
  flex: 1;
  min-width: 0;
}

.cmi-name {
  font-family: var(--font-mono);
  font-weight: 500;
  font-size: var(--fs-sm);
}

.cmi-meta {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.cmi-proto {
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  padding: 1px 6px;
  border-radius: 3px;
  background: var(--bg-surface);
  color: var(--text-muted);
}

.conn-menu-empty {
  padding: var(--sp-xl);
  text-align: center;
  color: var(--text-muted);
}

.conn-menu-footer {
  padding: var(--sp-sm) var(--sp-lg);
  border-top: 1px solid var(--border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.conn-menu-footer kbd {
  padding: 1px 5px;
  border: 1px solid var(--border);
  border-radius: 3px;
  font-family: var(--font-mono);
  font-size: 10px;
  background: var(--bg-surface);
}

/* ── Shortcuts Panel ── */
.shortcuts-overlay {
  position: fixed;
  inset: 0;
  z-index: 500;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
}

.shortcuts-card {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-2xl);
  width: 520px;
  max-width: 90vw;
  max-height: 80vh;
  overflow-y: auto;
  box-shadow: var(--shadow-lg), var(--phosphor-shadow);
  animation: modalIn 0.15s ease;
}

.shortcuts-card h2 {
  font-family: var(--font-mono);
  font-size: var(--fs-lg);
  font-weight: 600;
  margin-bottom: var(--sp-lg);
}

.shortcut-group {
  margin-bottom: var(--sp-lg);
}

.shortcut-group-title {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
  margin-bottom: var(--sp-sm);
}

.shortcut-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-xs) 0;
  font-size: var(--fs-sm);
}

.shortcut-row .desc {
  color: var(--text-secondary);
}

.shortcut-row kbd {
  padding: 2px 8px;
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  background: var(--bg-surface);
  color: var(--text-primary);
  min-width: 28px;
  text-align: center;
}

.shortcut-row .keys {
  display: flex;
  gap: 4px;
}

.shortcut-row .key-plus {
  color: var(--text-muted);
  font-size: var(--fs-xs);
}

@keyframes modalIn {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}
</style>
