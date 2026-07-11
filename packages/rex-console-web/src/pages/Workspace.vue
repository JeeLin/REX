<template>
  <div class="workspace-shell">
    <!-- 标签栏 -->
    <TabBar :panel-count="panelCount" :drag-id="dragId" @update:drag-id="dragId = $event" @new-connection="showConnMenu = true" @dblclick="handleTabDblclick">
      <template #right>
        <div class="layout-indicator" :title="`${t('ws.layout.switch')} (Alt+1~5)`" @click="cycleLayout">
          <span>{{ layoutIcon }}</span>
          <span class="layout-label">{{ layoutLabel }}</span>
        </div>
        <button class="ws-tab-add" :title="t('ws.fullscreen')" style="font-size: 13px" @click="toggleFullscreen">⛶</button>
        <button class="ws-tab-add" :title="t('ws.shortcuts.title')" style="font-size: 12px" @click="showShortcutsPanel = true">⌨</button>
      </template>
    </TabBar>

    <!-- 内容区 -->
    <div class="ws-content" :class="layoutClass" :style="layoutStyle">
      <!-- 空状态 -->
      <div v-if="tabs.length === 0" class="ws-empty">
        <div class="ws-empty-icon">💻</div>
        <div class="ws-empty-text">{{ t('ws.empty.noSessions') }}</div>
        <div class="ws-empty-hint">
          {{ t('ws.empty.hint') }}<br />
          <span v-html="t('ws.empty.shortcutsHint')"></span>
        </div>
      </div>

      <!-- 面板区域 -->
      <template v-else>
        <!-- 单面板模式：渲染所有 tab，用 v-show 切换，保持连接不中断 -->
        <div
          v-if="currentLayout === 'single'"
          class="ws-panel active layout-single"
        >
          <template v-for="tab in tabs" :key="tab.id">
            <div v-show="tab.id === activeTabId" class="ws-tab-content">
              <WorkspaceTerminal
                v-if="tab.component === 'terminal'"
                :resource-id="tab.resourceId"
                :resource-name="tab.name"
                :connection-mode="getConnectionMode(tab.resourceId)"
                @disconnect="onPanelDisconnect(tab.id)"
                @error="(msg: string) => onPanelError(tab.id, msg)"
              />
              <WorkspaceSql
                v-else-if="tab.component === 'sql'"
                :resource-id="tab.resourceId"
                :resource-name="tab.name"
                :protocol="tab.proto"
                @disconnect="onPanelDisconnect(tab.id)"
                @error="(msg: string) => onPanelError(tab.id, msg)"
              />
              <WorkspaceFiles
                v-else-if="tab.component === 'files'"
                :resource-id="tab.resourceId"
                :resource-name="tab.name"
                @disconnect="onPanelDisconnect(tab.id)"
                @error="(msg: string) => onPanelError(tab.id, msg)"
              />
              <WorkspaceRedis
                v-else-if="tab.component === 'redis'"
                :resource-id="tab.resourceId"
                :resource-name="tab.name"
                @disconnect="onPanelDisconnect(tab.id)"
                @error="(msg: string) => onPanelError(tab.id, msg)"
              />
              <WorkspaceSqlite
                v-else-if="tab.component === 'sqlite'"
                :resource-id="tab.resourceId"
                :resource-name="tab.name"
                @disconnect="onPanelDisconnect(tab.id)"
                @error="(msg: string) => onPanelError(tab.id, msg)"
              />
              <WorkspaceS3
                v-else-if="tab.component === 's3'"
                :resource-id="tab.resourceId"
                :resource-name="tab.name"
                @disconnect="onPanelDisconnect(tab.id)"
                @error="(msg: string) => onPanelError(tab.id, msg)"
              />
              <div v-else class="panel-unsupported">
                <span :style="{ color: getProtocolIcon(tab.proto).color }">{{ getProtocolIcon(tab.proto).icon }}</span>
                <div class="panel-unsupported-text">{{ tab.name }}</div>
                <div class="panel-unsupported-hint">{{ t('ws.empty.unsupported', { proto: tab.proto.toUpperCase() }) }}</div>
              </div>
            </div>
          </template>
        </div>

        <!-- 分屏模式：按面板渲染 -->
        <div
          v-for="i in panelCount"
          v-else
          :key="i"
          class="ws-panel"
          :class="{
            active: isPanelActive(i - 1),
            'layout-drop-zone': dragId && dragOverPanel === i - 1
          }"
          @dragover="onPanelDragOver($event, i - 1)"
          @dragleave="onPanelDragLeave"
          @drop="onPanelDrop($event, i - 1)"
          @mousedown="onPanelMouseDown($event, i - 1)"
        >
          <template v-if="getPanelTab(i - 1)">
            <WorkspaceTerminal
              v-if="getPanelTab(i - 1)!.component === 'terminal'"
              :key="'panel-' + (i - 1)"
              :resource-id="getPanelTab(i - 1)!.resourceId"
              :resource-name="getPanelTab(i - 1)!.name"
              :connection-mode="getConnectionMode(getPanelTab(i - 1)!.resourceId)"
              @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
              @error="(msg: string) => onPanelError(getPanelTab(i - 1)!.id, msg)"
            />
            <WorkspaceSql
              v-else-if="getPanelTab(i - 1)!.component === 'sql'"
              :key="'panel-' + (i - 1)"
              :resource-id="getPanelTab(i - 1)!.resourceId"
              :resource-name="getPanelTab(i - 1)!.name"
              :protocol="getPanelTab(i - 1)!.proto"
              @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
              @error="(msg: string) => onPanelError(getPanelTab(i - 1)!.id, msg)"
            />
            <WorkspaceFiles
              v-else-if="getPanelTab(i - 1)!.component === 'files'"
              :key="'panel-' + (i - 1)"
              :resource-id="getPanelTab(i - 1)!.resourceId"
              :resource-name="getPanelTab(i - 1)!.name"
              @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
              @error="(msg: string) => onPanelError(getPanelTab(i - 1)!.id, msg)"
            />
            <WorkspaceRedis
              v-else-if="getPanelTab(i - 1)!.component === 'redis'"
              :key="'panel-' + (i - 1)"
              :resource-id="getPanelTab(i - 1)!.resourceId"
              :resource-name="getPanelTab(i - 1)!.name"
              @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
              @error="(msg: string) => onPanelError(getPanelTab(i - 1)!.id, msg)"
            />
            <WorkspaceSqlite
              v-else-if="getPanelTab(i - 1)!.component === 'sqlite'"
              :key="'panel-' + (i - 1)"
              :resource-id="getPanelTab(i - 1)!.resourceId"
              :resource-name="getPanelTab(i - 1)!.name"
              @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
              @error="(msg: string) => onPanelError(getPanelTab(i - 1)!.id, msg)"
            />
            <WorkspaceS3
              v-else-if="getPanelTab(i - 1)!.component === 's3'"
              :key="'panel-' + (i - 1)"
              :resource-id="getPanelTab(i - 1)!.resourceId"
              :resource-name="getPanelTab(i - 1)!.name"
              @disconnect="onPanelDisconnect(getPanelTab(i - 1)!.id)"
              @error="(msg: string) => onPanelError(getPanelTab(i - 1)!.id, msg)"
            />
            <div v-else class="panel-unsupported">
              <span :style="{ color: getProtocolIcon(getPanelTab(i - 1)!.proto).color }">{{ getProtocolIcon(getPanelTab(i - 1)!.proto).icon }}</span>
              <div class="panel-unsupported-text">{{ getPanelTab(i - 1)!.name }}</div>
              <div class="panel-unsupported-hint">{{ t('ws.empty.unsupported', { proto: getPanelTab(i - 1)!.proto.toUpperCase() }) }}</div>
            </div>
          </template>
          <div v-else class="panel-empty">
            <span class="panel-empty-text">{{ t('ws.empty.panelEmpty', { n: i }) }}</span>
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
        <div v-if="allTags.length" class="conn-menu-tags">
          <button
            v-for="tag in allTags"
            :key="tag.id"
            class="cm-tag-chip"
            :class="{ active: selectedTagIds.includes(tag.id) }"
            :style="{ '--tag-color': tag.color }"
            @click="toggleTag(tag.id)"
          >
            <span class="cm-tag-dot" />
            {{ tag.name }}
          </button>
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
              <template v-if="getResourceTags(res.id).length">
                <span
                  v-for="tag in getResourceTags(res.id).slice(0, 3)"
                  :key="tag.id"
                  class="cmi-tag"
                  :style="{ background: tag.color + '18', color: tag.color }"
                >{{ tag.name }}</span>
                <span v-if="getResourceTags(res.id).length > 3" class="cmi-tag cmi-tag-more">+{{ getResourceTags(res.id).length - 3 }}</span>
              </template>
            </div>
          </template>
          <div v-if="flatFilteredResources.length === 0" class="conn-menu-empty">{{ t('ws.empty.connEmpty') }}</div>
        </div>
        <div class="conn-menu-footer">
          <span>{{ t('ws.empty.connHint') }}</span>
          <span>{{ t('ws.empty.connClose') }}</span>
        </div>
      </div>
    </div>

    <!-- 全局搜索命令面板 (Ctrl+K) -->
    <CommandPalette
      ref="cmdRef"
      :visible="showCommandPalette"
      @close="showCommandPalette = false"
      @select="handleCommandSelect"
    />

    <!-- 快捷键面板 -->
    <div v-if="showShortcutsPanel" class="shortcuts-overlay" @click="showShortcutsPanel = false">
      <div class="shortcuts-card" @click.stop>
        <h2>⌨ {{ t('ws.shortcuts.title') }}</h2>
        <div class="shortcut-group">
          <div class="shortcut-group-title">{{ t('ws.shortcuts.tab') }}</div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.newConnection') }}</span><span class="keys"><kbd>Ctrl</kbd><span class="key-plus">+</span><kbd>N</kbd></span></div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.closeTab') }}</span><span class="keys"><kbd>Ctrl</kbd><span class="key-plus">+</span><kbd>W</kbd></span></div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.nextTab') }}</span><span class="keys"><kbd>Ctrl</kbd><span class="key-plus">+</span><kbd>Tab</kbd></span></div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.prevTab') }}</span><span class="keys"><kbd>Ctrl</kbd><span class="key-plus">+</span><kbd>Shift</kbd><span class="key-plus">+</span><kbd>Tab</kbd></span></div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.tabN') }}</span><span class="keys"><kbd>Ctrl</kbd><span class="key-plus">+</span><kbd>1~9</kbd></span></div>
        </div>
        <div class="shortcut-group">
          <div class="shortcut-group-title">{{ t('ws.shortcuts.layout') }}</div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.layoutSingle') }}</span><span class="keys"><kbd>Alt</kbd><span class="key-plus">+</span><kbd>1</kbd></span></div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.layoutLeftRight') }}</span><span class="keys"><kbd>Alt</kbd><span class="key-plus">+</span><kbd>2</kbd></span></div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.layoutTopBottom') }}</span><span class="keys"><kbd>Alt</kbd><span class="key-plus">+</span><kbd>3</kbd></span></div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.layoutQuad') }}</span><span class="keys"><kbd>Alt</kbd><span class="key-plus">+</span><kbd>4</kbd></span></div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.layoutSidebarMain') }}</span><span class="keys"><kbd>Alt</kbd><span class="key-plus">+</span><kbd>5</kbd></span></div>
        </div>
        <div class="shortcut-group">
          <div class="shortcut-group-title">{{ t('ws.shortcuts.other') }}</div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.globalSearch') }}</span><span class="keys"><kbd>Ctrl</kbd><span class="key-plus">+</span><kbd>K</kbd></span></div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.toggleFullscreen') }}</span><span class="keys"><kbd>F11</kbd></span></div>
          <div class="shortcut-row"><span class="desc">{{ t('ws.shortcuts.help') }}</span><span class="keys"><kbd>F1</kbd></span></div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick, defineAsyncComponent } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { getProtocolIcon } from '@/composables/useProtocol'
import type { Protocol } from '@/composables/useProtocol'
import { listEnvsWithResources } from '@/api/env'
import type { EnvWithResources } from '@/api/env'
import { listTags, type Tag } from '@/api/tags'
import client from '@/api/client'
import CommandPalette from '@/components/CommandPalette.vue'
import type { CommandItem } from '@/components/CommandPalette.vue'
import TabBar from '@/features/workspace/TabBar.vue'
import { useTabs } from '@/features/workspace/useTabs'
import { useWorkspacePersistence } from '@/composables/useWorkspacePersistence'

// Lazy load workspace panels for better code splitting
const WorkspaceTerminal = defineAsyncComponent(() => import('@/features/workspace/panels/WorkspaceTerminal.vue'))
const WorkspaceSql = defineAsyncComponent(() => import('@/features/workspace/panels/WorkspaceSql.vue'))
const WorkspaceFiles = defineAsyncComponent(() => import('@/features/workspace/panels/WorkspaceFiles.vue'))
const WorkspaceRedis = defineAsyncComponent(() => import('@/features/workspace/panels/WorkspaceRedis.vue'))
const WorkspaceSqlite = defineAsyncComponent(() => import('@/features/workspace/panels/WorkspaceSqlite.vue'))
const WorkspaceS3 = defineAsyncComponent(() => import('@/features/workspace/panels/WorkspaceS3.vue'))

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

// ── Tabs ──
const { tabs, activeTabId, addTab, closeTab, duplicateTab, nextTab, prevTab, switchTabByIndex, moveTabToPanel, swapPanels } = useTabs()

// ── Workspace Persistence ──
const { restore } = useWorkspacePersistence()

// ── Layout ──
type Layout = 'single' | 'left-right' | 'top-bottom' | 'quad' | 'sidebar-main'
const LAYOUT_ORDER: Layout[] = ['single', 'left-right', 'top-bottom', 'quad', 'sidebar-main']
const LAYOUT_PANELS: Record<Layout, number> = {
  single: 1, 'left-right': 2, 'top-bottom': 2, quad: 4, 'sidebar-main': 2,
}
const LAYOUT_ICONS: Record<Layout, string> = {
  single: '⬜', 'left-right': '🔲', 'top-bottom': '🔳', quad: '🔲🔲', 'sidebar-main': '📐',
}
const LAYOUT_LABELS: Record<Layout, string> = {
  single: t('ws.layout.single'), 'left-right': t('ws.layout.leftRight'), 'top-bottom': t('ws.layout.topBottom'), quad: t('ws.layout.quad'), 'sidebar-main': t('ws.layout.sidebarMain'),
}

const currentLayout = ref<Layout>('single')
const panelCount = computed(() => LAYOUT_PANELS[currentLayout.value])
const layoutIcon = computed(() => LAYOUT_ICONS[currentLayout.value])
const layoutLabel = computed(() => LAYOUT_LABELS[currentLayout.value])
const layoutClass = computed(() => {
  if (currentLayout.value === 'single') return 'layout-single'
  return `layout-split layout-${currentLayout.value}`
})
const layoutStyle = computed(() => {
  if (currentLayout.value === 'single') return {}
  const s = panelSizes.value
  const hasCustom = Object.keys(s).length > 0
  if (!hasCustom) return {}
  if (currentLayout.value === 'left-right') {
    return { gridTemplateColumns: `${s[0] ?? 50}fr ${s[1] ?? 50}fr` }
  } else if (currentLayout.value === 'top-bottom') {
    return { gridTemplateRows: `${s[0] ?? 50}fr ${s[1] ?? 50}fr` }
  } else if (currentLayout.value === 'sidebar-main') {
    return { gridTemplateColumns: `${s[0] ?? 67}fr ${s[1] ?? 33}fr` }
  }
  return {}
})


function setLayout(layout: Layout) {
  currentLayout.value = layout
}
function cycleLayout() {
  const idx = LAYOUT_ORDER.indexOf(currentLayout.value)
  currentLayout.value = LAYOUT_ORDER[(idx + 1) % LAYOUT_ORDER.length]!
}
// ── Panel Resize ──
const panelSizes = ref<Record<number, number>>({})
let resizingPanel = -1
let resizingStart = 0
let resizingStartSize = 0

function startPanelResize(e: MouseEvent, panelIndex: number) {
  e.preventDefault()
  resizingPanel = panelIndex
  resizingStart = currentLayout.value.includes('top-bottom') ? e.clientY : e.clientX
  resizingStartSize = panelSizes.value[panelIndex] ?? 50
  document.addEventListener('mousemove', onPanelResize)
  document.addEventListener('mouseup', stopPanelResize)
  document.body.style.cursor = currentLayout.value.includes('top-bottom') ? 'row-resize' : 'col-resize'
  document.body.style.userSelect = 'none'
}

function onPanelResize(e: MouseEvent) {
  if (resizingPanel < 0) return
  const current = currentLayout.value.includes('top-bottom') ? e.clientY : e.clientX
  const diff = current - resizingStart
  const container = document.querySelector('.ws-content.layout-split') as HTMLElement
  if (!container) return
  const total = currentLayout.value.includes('top-bottom') ? container.clientHeight : container.clientWidth
  const deltaPercent = (diff / total) * 100
  const newSize = Math.max(20, Math.min(80, resizingStartSize + deltaPercent))
  panelSizes.value[resizingPanel] = newSize
}

function stopPanelResize() {
  resizingPanel = -1
  document.removeEventListener('mousemove', onPanelResize)
  document.removeEventListener('mouseup', stopPanelResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}
function onPanelMouseDown(e: MouseEvent, panelIndex: number) {
  if (currentLayout.value === 'single') return
  const el = e.currentTarget as HTMLElement
  const rect = el.getBoundingClientRect()
  const isHorizontal = !currentLayout.value.includes('top-bottom')
  const handleSize = 6
  let isHandle = false
  if (isHorizontal) {
    isHandle = e.clientX > rect.right - handleSize
  } else {
    isHandle = e.clientY > rect.bottom - handleSize
  }
  if (isHandle && panelIndex < panelCount.value - 1) {
    startPanelResize(e, panelIndex)
  }
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
  e.dataTransfer!.dropEffect = 'move'
  dragOverPanel.value = panelIndex
}

function onPanelDragLeave(e: DragEvent) {
  const related = e.relatedTarget as HTMLElement | null
  const current = e.currentTarget as HTMLElement
  // Don't clear if still inside the same panel (child element traversal)
  if (related && current.contains(related)) return
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
    const fallback = tabs.value[(currentIdx + 1) % tabs.value.length]!
    if (fallback.id !== tabId) moveTabToPanel(fallback.id, 1)
  }
}

// ── Connection Menu ──
const showConnMenu = ref(false)
const connSearchQuery = ref('')
const connSearchInput = ref<HTMLInputElement | null>(null)
const selectedResourceIdx = ref(0)

interface Resource { id: string; name: string; address: string; protocol: string; envName: string; connectionMode: 'direct' | 'agent' }

const envsWithRes = ref<EnvWithResources[]>([])

// ── Tag Filtering ──
const allTags = ref<Tag[]>([])
const resourceTagMap = ref<Record<string, string[]>>({}) // resource_id -> tag_ids
const selectedTagIds = ref<string[]>([])

interface ResourceTagInfo { resource_id: string; tags: Tag[] }

const flatFilteredResources = computed<Resource[]>(() => {
  const all: Resource[] = []
  const tagFilter = selectedTagIds.value
  for (const env of envsWithRes.value) {
    for (const r of env.resources) {
      const q = connSearchQuery.value.toLowerCase()
      const matchesSearch = !q || r.name.toLowerCase().includes(q) || r.protocol.includes(q)
      const matchesTags = tagFilter.length === 0 || tagFilter.every(tid => resourceTagMap.value[r.id]?.includes(tid))
      if (matchesSearch && matchesTags) {
        all.push({ id: r.id, name: r.name, address: '', protocol: r.protocol, envName: env.name, connectionMode: (env.connection_mode === 'agent' ? 'agent' : 'direct') as 'direct' | 'agent' })
      }
    }
  }
  return all
})

const groupedResources = computed(() => {
  const groups: Record<string, Resource[]> = {}
  for (const r of flatFilteredResources.value) {
    if (!groups[r.envName]) groups[r.envName] = []
    groups[r.envName]!.push(r)
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
  addTab(res.name, res.protocol as Protocol, res.id)
  showConnMenu.value = false
  connSearchQuery.value = ''
  selectedResourceIdx.value = 0
  selectedTagIds.value = []
}

function toggleTag(tagId: string) {
  const idx = selectedTagIds.value.indexOf(tagId)
  if (idx >= 0) selectedTagIds.value.splice(idx, 1)
  else selectedTagIds.value.push(tagId)
  selectedResourceIdx.value = 0
}

function getResourceTags(resourceId: string): Tag[] {
  const tagIds = resourceTagMap.value[resourceId]
  if (!tagIds?.length) return []
  return allTags.value.filter(t => tagIds.includes(t.id))
}

async function fetchResourceTags() {
  try {
    const res = await client.get<{ data: ResourceTagInfo[] }>('/resource-tags')
    const map: Record<string, string[]> = {}
    for (const item of res.data.data) {
      map[item.resource_id] = item.tags.map(t => t.id)
    }
    resourceTagMap.value = map
  } catch { /* */ }
}

async function loadTags() {
  try { allTags.value = await listTags() } catch { /* */ }
  await fetchResourceTags()
}

function getConnectionMode(resourceId: string): 'direct' | 'agent' {
  for (const env of envsWithRes.value) {
    const r = env.resources?.find(x => x.id === resourceId)
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
    loadTags()
  }
})

watch(connSearchQuery, () => {
  selectedResourceIdx.value = 0
})

onMounted(async () => {
  try { envsWithRes.value = await listEnvsWithResources() } catch { /* */ }
  window.addEventListener('keydown', onKeyDown)

  // Restore workspace state from persistence
  restore()

  // 从路由 query 读取待打开的资源
  const openId = route.query.open as string
  if (openId) {
    const openName = (route.query.name as string) || openId
    const openProto = (route.query.proto as string) || 'ssh'
    addTab(openName, openProto as Protocol, openId)
    // 清除 query 参数，避免刷新重复打开
    router.replace({ name: 'workspace' })
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKeyDown)
})

// ── Shortcuts Panel ──
const showShortcutsPanel = ref(false)

// ── Command Palette (Ctrl+K) ──
const showCommandPalette = ref(false)
const cmdRef = ref<InstanceType<typeof CommandPalette> | null>(null)

const NAV_ITEMS: CommandItem[] = [
  { id: 'nav:workspace', label: t('ws.nav.workspace'), category: 'navigation', icon: '⊞' },
  { id: 'nav:dashboard', label: t('ws.nav.dashboard'), category: 'navigation', icon: '◉' },
  { id: 'nav:environments', label: t('ws.nav.environments'), category: 'navigation', icon: '◈' },
  { id: 'nav:agents', label: t('ws.nav.agents'), category: 'navigation', icon: '⬡' },
  { id: 'nav:settings', label: t('ws.nav.settings'), category: 'navigation', icon: '⚙' },
]

const ACTION_ITEMS: CommandItem[] = [
  { id: 'action:new-connection', label: t('ws.actions.newConnection'), category: 'action', icon: '+', shortcut: 'Ctrl+N' },
  { id: 'action:layout-single', label: t('ws.actions.layoutSingle'), category: 'action', icon: '⬜', shortcut: 'Alt+1' },
  { id: 'action:layout-left-right', label: t('ws.actions.layoutLeftRight'), category: 'action', icon: '🔲', shortcut: 'Alt+2' },
  { id: 'action:layout-top-bottom', label: t('ws.actions.layoutTopBottom'), category: 'action', icon: '🔳', shortcut: 'Alt+3' },
  { id: 'action:layout-quad', label: t('ws.actions.layoutQuad'), category: 'action', icon: '🔲', shortcut: 'Alt+4' },
  { id: 'action:layout-sidebar-main', label: t('ws.actions.layoutSidebarMain'), category: 'action', icon: '📐', shortcut: 'Alt+5' },
  { id: 'action:fullscreen', label: t('ws.actions.toggleFullscreen'), category: 'action', icon: '⛶', shortcut: 'F11' },
]

function buildCommandPaletteItems(): CommandItem[] {
  const items: CommandItem[] = []
  // Resources
  for (const env of envsWithRes.value) {
    for (const r of env.resources) {
      const proto = getProtocolIcon(r.protocol)
      items.push({
        id: `res:${r.id}`,
        label: r.name,
        category: 'resource',
        icon: proto.icon,
        hint: env.name,
        color: proto.color,
      })
    }
  }
  items.push(...NAV_ITEMS)
  items.push(...ACTION_ITEMS)
  return items
}

function handleCommandSelect(item: CommandItem) {
  showCommandPalette.value = false
  if (item.id.startsWith('res:')) {
    const resourceId = item.id.slice(4)
    const resource = envsWithRes.value.flatMap(e => e.resources).find(r => r.id === resourceId)
    if (resource) addTab(resource.name, resource.protocol as Protocol, resource.id)
  } else if (item.id.startsWith('nav:')) {
    const routeMap: Record<string, string> = {
      'nav:dashboard': 'dashboard',
      'nav:environments': 'environments',
      'nav:agents': 'agents',
      'nav:settings': 'settings',
      'nav:workspace': 'workspace',
    }
    const name = routeMap[item.id]
    if (name && name !== 'workspace') router.push({ name })
  } else if (item.id === 'action:new-connection') {
    showConnMenu.value = true
  } else if (item.id.startsWith('action:layout-')) {
    const layout = item.id.replace('action:layout-', '') as Layout
    setLayout(layout)
  } else if (item.id === 'action:fullscreen') {
    toggleFullscreen()
  }
}

watch(showCommandPalette, (val) => {
  if (val) {
    nextTick(() => cmdRef.value?.setCommands(buildCommandPaletteItems()))
  }
})

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
  window.addEventListener('rex:shortcut', onShortcut as EventListener)
})
onUnmounted(() => {
  window.removeEventListener('open-in-workspace', onOpenInWorkspace)
  window.removeEventListener('rex:shortcut', onShortcut as EventListener)
})
function onShortcut(e: Event) {
  const detail = (e as CustomEvent).detail
  if (detail === 'command-palette') {
    showCommandPalette.value = !showCommandPalette.value
  } else if (detail === 'new-connection') {
    showConnMenu.value = true
  } else if (detail === 'shortcuts-panel') {
    showShortcutsPanel.value = !showShortcutsPanel.value
  }
}


// ── Global Keyboard Shortcuts ──
function onKeyDown(e: KeyboardEvent) {
  const el = e.target as HTMLElement
  const tag = el.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || el.isContentEditable) return

  // Escape always works — close overlays in priority order
  if (e.key === 'Escape') {
    if (showCommandPalette.value) { showCommandPalette.value = false; return }
    if (showConnMenu.value) { showConnMenu.value = false; return }
    if (showShortcutsPanel.value) { showShortcutsPanel.value = false; return }
    return
  }

  // Suppress workspace shortcuts when any overlay is open
  if (showCommandPalette.value || showConnMenu.value || showShortcutsPanel.value) return

  const ctrl = e.ctrlKey || e.metaKey

  if (ctrl && e.key === 'w') {
    e.preventDefault()
    if (activeTabId.value) {
      closeTab(activeTabId.value)
    }
  } else if (ctrl && e.key === 'd') {
    e.preventDefault()
    if (activeTabId.value) {
      duplicateTab(activeTabId.value)
    }
  } else if (ctrl && e.key === 'Tab') {
    e.preventDefault()
    if (e.shiftKey) {
      prevTab()
    } else {
      nextTab()
    }
  } else if (ctrl && e.key >= '1' && e.key <= '9') {
    e.preventDefault()
    switchTabByIndex(parseInt(e.key) - 1)
  } else if (e.altKey && e.key >= '1' && e.key <= '5') {
    e.preventDefault()
    setLayout(LAYOUT_ORDER[parseInt(e.key) - 1]!)
  } else if (e.key === 'F11') {
    e.preventDefault()
    toggleFullscreen()
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

.ws-tab-content {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.ws-content.layout-split {
  display: grid;
  gap: 1px;
  background: var(--border);
}

.ws-content.layout-split .ws-panel {
  position: relative;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-deep);
  min-height: 0;
}
.ws-content.layout-split .ws-panel::after {
  content: '';
  position: absolute;
  background: transparent;
  z-index: 10;
  transition: background 0.15s;
}
.ws-content.layout-split .ws-panel::after:hover {
  background: var(--accent);
}
.ws-content.layout-split:not(.layout-top-bottom):not(.layout-quad) .ws-panel::after {
  right: -3px;
  top: 0;
  width: 6px;
  height: 100%;
  cursor: col-resize;
}
.ws-content.layout-split.layout-top-bottom .ws-panel::after {
  bottom: -3px;
  left: 0;
  width: 100%;
  height: 6px;
  cursor: row-resize;
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

.conn-menu-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  padding: var(--sp-sm) var(--sp-md);
  border-bottom: 1px solid var(--border);
}

.cm-tag-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 11px;
  font-weight: 500;
  background: color-mix(in srgb, var(--tag-color) 10%, transparent);
  color: color-mix(in srgb, var(--tag-color) 80%, var(--text-muted));
  border: 1px solid color-mix(in srgb, var(--tag-color) 20%, transparent);
  cursor: pointer;
  transition: all 0.15s;
}

.cm-tag-chip:hover {
  background: color-mix(in srgb, var(--tag-color) 20%, transparent);
}

.cm-tag-chip.active {
  background: color-mix(in srgb, var(--tag-color) 25%, transparent);
  color: var(--tag-color);
  border-color: color-mix(in srgb, var(--tag-color) 50%, transparent);
}

.cm-tag-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--tag-color);
  flex-shrink: 0;
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

.cmi-tag {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 8px;
  font-weight: 500;
}

.cmi-tag-more {
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

/* ── Mobile Layout ── */
@media (max-width: 767px) {
  .ws-tabbar {
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
  }

  .ws-tab-name {
    max-width: 80px;
    font-size: var(--fs-xs);
  }

  .ws-tab-close {
    opacity: 1;
  }

  .layout-indicator {
    display: none;
  }

  .ws-content.layout-split {
    grid-template-columns: 1fr !important;
    grid-template-rows: 1fr !important;
  }

  .ws-content.layout-split .ws-panel:not(.active) {
    display: none;
  }

  .conn-menu {
    width: 100%;
    max-width: 100%;
    height: 100%;
    border-radius: 0;
    margin: 0;
  }

  .conn-menu-overlay {
    padding-top: 0;
  }
}

/* ── Small Mobile (< 480px) ── */
@media (max-width: 479px) {
  .ws-tabbar {
    height: 32px;
    padding: 0 var(--sp-xs);
  }

  .ws-tab {
    padding: 4px var(--sp-sm);
    font-size: var(--fs-xs);
  }

  .ws-tab-name {
    max-width: 60px;
  }
}

/* ── Mobile Touch Optimizations ── */
@media (max-width: 767px) {
  /* Connection menu touch optimization */
  .conn-menu-item {
    min-height: 48px;
    padding: var(--sp-sm) var(--sp-md);
  }

  /* Shortcuts panel mobile adaptation */
  .shortcuts-panel-separator {
    margin: var(--sp-sm) 0;
  }

  .shortcut-item {
    padding: var(--sp-sm);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
  }

  .shortcut-key {
    min-width: 60px;
    display: inline-block;
  }
}
</style>
