<template>
  <div class="s3-object-browser">
    <!-- 面包屑路径 -->
    <div class="s3-breadcrumb">
      <span class="s3-breadcrumb-bucket">{{ bucket }}</span>
      <template v-for="(part, i) in pathParts" :key="i">
        <span class="s3-breadcrumb-sep">/</span>
        <span
          class="s3-breadcrumb-part"
          :class="{ clickable: i < pathParts.length - 1 }"
          @click="i < pathParts.length - 1 ? navigateTo(i) : null"
        >{{ part }}</span>
      </template>
      <div class="s3-breadcrumb-spacer" />
      <button class="s3-btn s3-btn-sm" :disabled="loading" @click="$emit('refresh')">🔄</button>
    </div>

    <!-- 工具栏 -->
    <div class="s3-toolbar">
      <button class="s3-btn s3-btn-sm" @click="$emit('upload')">{{ t('s3.upload') }}</button>
      <div class="s3-toolbar-spacer" />
      <span class="s3-toolbar-count" v-if="!loading">
        {{ t('s3.itemCount', { total: items.length, dirs: dirCount, files: fileCount }) }}
      </span>
    </div>

    <!-- 表头 -->
    <div class="s3-table-header">
      <span class="s3-col-name">{{ t('s3.name') }}</span>
      <span class="s3-col-size">{{ t('s3.size') }}</span>
      <span class="s3-col-modified">{{ t('s3.modified') }}</span>
      <span class="s3-col-actions">{{ t('s3.actions') }}</span>
    </div>

    <!-- 加载中 -->
    <div v-if="loading" class="s3-loading">{{ t('s3.loading') }}</div>

    <!-- 空目录 -->
    <div v-else-if="items.length === 0" class="s3-empty">{{ t('s3.emptyDir') }}</div>

    <!-- Object 列表 -->
    <div v-else class="s3-table-body">
      <!-- 上级目录 -->
      <div
        v-if="prefix"
        class="s3-table-row s3-row-parent"
        @click="goUp"
      >
        <span class="s3-col-name">
          <span class="s3-icon">📁</span>
          <span>..</span>
        </span>
        <span class="s3-col-size">—</span>
        <span class="s3-col-modified">—</span>
        <span class="s3-col-actions" />
      </div>

      <div
        v-for="item in sortedItems"
        :key="item.key"
        class="s3-table-row"
        :class="{ 's3-row-dir': item.is_dir }"
        @dblclick="item.is_dir ? enterDir(item.key) : downloadItem(item)"
        @contextmenu.prevent="showCtxMenu($event, item)"
      >
        <span class="s3-col-name">
          <span class="s3-icon">{{ item.is_dir ? '📁' : '📄' }}</span>
          <span>{{ item.is_dir ? item.key.split('/').filter(Boolean).pop() + '/' : item.key.split('/').pop() }}</span>
        </span>
        <span class="s3-col-size">{{ item.is_dir ? '—' : formatSize(item.size) }}</span>
        <span class="s3-col-modified">{{ formatDate(item.last_modified) }}</span>
        <span class="s3-col-actions">
          <button v-if="!item.is_dir" class="s3-action-btn" @click.stop="downloadItem(item)" :title="t('s3.download')">⬇</button>
          <button class="s3-action-btn s3-action-danger" @click.stop="deleteItem(item)" :title="t('s3.delete')">🗑</button>
        </span>
      </div>
    </div>

    <!-- 右键菜单 -->
    <Teleport to="body">
      <div
        v-if="ctxMenu.visible"
        class="s3-ctx-menu"
        :style="{ top: ctxMenu.y + 'px', left: ctxMenu.x + 'px' }"
      >
        <button v-if="ctxMenu.item && !ctxMenu.item.is_dir" class="s3-ctx-item" @click="downloadItem(ctxMenu.item!)">
          {{ t('s3.download') }}
        </button>
        <button class="s3-ctx-item" @click="viewInfo(ctxMenu.item)">
          {{ t('s3.properties') }}
        </button>
        <div class="s3-ctx-divider" />
        <button class="s3-ctx-item s3-ctx-danger" @click="deleteItem(ctxMenu.item!)">
          {{ t('s3.delete') }}
        </button>
      </div>
    </Teleport>

    <!-- 属性弹窗 -->
    <Teleport to="body">
      <div v-if="infoModal.visible" class="s3-modal-overlay" @click.self="infoModal.visible = false">
        <div class="s3-modal">
          <div class="s3-modal-header">
            <span>{{ t('s3.properties') }}</span>
            <button class="s3-btn s3-btn-sm" @click="infoModal.visible = false">✕</button>
          </div>
          <div class="s3-modal-body" v-if="infoModal.item">
            <div class="s3-info-row"><span class="s3-info-label">{{ t('s3.name') }}</span><span>{{ infoModal.item.key }}</span></div>
            <div class="s3-info-row"><span class="s3-info-label">{{ t('s3.size') }}</span><span>{{ formatSize(infoModal.item.size) }}</span></div>
            <div class="s3-info-row" v-if="infoModal.item.content_type"><span class="s3-info-label">Type</span><span>{{ infoModal.item.content_type }}</span></div>
            <div class="s3-info-row" v-if="infoModal.item.etag"><span class="s3-info-label">ETag</span><span>{{ infoModal.item.etag }}</span></div>
            <div class="s3-info-row" v-if="infoModal.item.storage_class"><span class="s3-info-label">Class</span><span>{{ infoModal.item.storage_class }}</span></div>
            <div class="s3-info-row" v-if="infoModal.item.last_modified"><span class="s3-info-label">{{ t('s3.modified') }}</span><span>{{ infoModal.item.last_modified }}</span></div>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import type { S3ObjectInfo } from './useS3Session'

const props = defineProps<{
  bucket: string
  prefix: string
  items: S3ObjectInfo[]
  loading: boolean
}>()

const emit = defineEmits<{
  navigate: [prefix: string]
  upload: []
  refresh: []
  download: [item: S3ObjectInfo]
  delete: [item: S3ObjectInfo]
}>()

const { t } = useI18n()

const pathParts = computed(() => {
  if (!props.prefix) return []
  return props.prefix.split('/').filter(Boolean)
})

const dirCount = computed(() => props.items.filter(i => i.is_dir).length)
const fileCount = computed(() => props.items.filter(i => !i.is_dir).length)

const sortedItems = computed(() => {
  return [...props.items].sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1
    return a.key.localeCompare(b.key)
  })
})

function navigateTo(index: number) {
  const newPrefix = pathParts.value.slice(0, index + 1).join('/') + '/'
  emit('navigate', newPrefix)
}

function goUp() {
  const parts = pathParts.value
  if (parts.length <= 1) {
    emit('navigate', '')
  } else {
    emit('navigate', parts.slice(0, -1).join('/') + '/')
  }
}

function enterDir(key: string) {
  emit('navigate', key)
}

function downloadItem(item: S3ObjectInfo) {
  hideCtxMenu()
  emit('download', item)
}

function deleteItem(item: S3ObjectInfo) {
  hideCtxMenu()
  emit('delete', item)
}

function viewInfo(item: S3ObjectInfo | null) {
  hideCtxMenu()
  if (item) {
    infoModal.item = item
    infoModal.visible = true
  }
}

// ── 右键菜单 ──────────────────────────────────────────────
const ctxMenu = reactive({ visible: false, x: 0, y: 0, item: null as S3ObjectInfo | null })
const infoModal = reactive({ visible: false, item: null as S3ObjectInfo | null })

function showCtxMenu(event: MouseEvent, item: S3ObjectInfo) {
  ctxMenu.item = item
  ctxMenu.x = event.clientX
  ctxMenu.y = event.clientY
  ctxMenu.visible = true
}

function hideCtxMenu() {
  ctxMenu.visible = false
  ctxMenu.item = null
}

// 全局点击关闭菜单
onUnmounted(() => {
  document.removeEventListener('click', hideCtxMenu)
})
document.addEventListener('click', hideCtxMenu)

// ── 格式化 ──────────────────────────────────────────────
function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return '—'
  try {
    return new Date(dateStr).toLocaleDateString()
  } catch {
    return dateStr
  }
}
</script>

<style scoped>
.s3-object-browser {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.s3-breadcrumb {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  font-size: 12px;
  font-family: 'JetBrains Mono', monospace;
}

.s3-breadcrumb-bucket { font-weight: 600; color: var(--accent); }
.s3-breadcrumb-sep { color: var(--text-muted); }
.s3-breadcrumb-part { color: var(--text-secondary); }
.s3-breadcrumb-part.clickable { cursor: pointer; color: var(--text-primary); }
.s3-breadcrumb-part.clickable:hover { color: var(--accent); }
.s3-breadcrumb-spacer { flex: 1; }

.s3-toolbar {
  display: flex;
  align-items: center;
  padding: 4px 12px;
  border-bottom: 1px solid var(--border-primary);
  font-size: 12px;
}

.s3-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-primary);
  padding: 3px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}

.s3-btn:hover:not(:disabled) { background: var(--bg-hover); }
.s3-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.s3-btn-sm { padding: 2px 6px; }
.s3-toolbar-spacer { flex: 1; }
.s3-toolbar-count { color: var(--text-muted); font-size: 11px; }

.s3-table-header {
  display: flex;
  padding: 4px 12px;
  border-bottom: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
}

.s3-col-name { flex: 3; display: flex; align-items: center; gap: 6px; }
.s3-col-size { flex: 1; text-align: right; }
.s3-col-modified { flex: 1; text-align: right; }
.s3-col-actions { width: 60px; text-align: right; }

.s3-loading, .s3-empty {
  padding: 24px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 12px;
}

.s3-table-body {
  flex: 1;
  overflow-y: auto;
}

.s3-table-row {
  display: flex;
  align-items: center;
  padding: 5px 12px;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s;
}

.s3-table-row:hover { background: var(--bg-hover); }
.s3-row-parent { color: var(--text-secondary); }
.s3-row-dir .s3-col-name span:last-child { color: var(--info); }

.s3-icon { font-size: 14px; width: 18px; text-align: center; }

.s3-action-btn {
  background: none;
  border: none;
  cursor: pointer;
  padding: 2px 4px;
  font-size: 12px;
  opacity: 0.5;
  transition: opacity 0.15s;
}

.s3-action-btn:hover { opacity: 1; }
.s3-action-danger:hover { color: #f85149; }

/* 右键菜单 */
.s3-ctx-menu {
  position: fixed;
  z-index: 9999;
  background: var(--bg-surface, #1c2128);
  border: 1px solid var(--border-primary);
  border-radius: 6px;
  padding: 4px 0;
  min-width: 140px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}

.s3-ctx-item {
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

.s3-ctx-item:hover { background: var(--bg-hover); }
.s3-ctx-danger { color: #f85149; }
.s3-ctx-danger:hover { background: #f8514922; }
.s3-ctx-divider { height: 1px; background: var(--border-primary); margin: 4px 0; }

/* 属性弹窗 */
.s3-modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
}

.s3-modal {
  background: var(--bg-surface, #1c2128);
  border: 1px solid var(--border-primary);
  border-radius: 8px;
  width: 480px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
}

.s3-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-primary);
  font-size: 13px;
  font-weight: 600;
}

.s3-modal-body {
  padding: 12px 16px;
  font-size: 12px;
}

.s3-info-row {
  display: flex;
  padding: 4px 0;
  gap: 12px;
}

.s3-info-label {
  color: var(--text-muted);
  min-width: 80px;
}
</style>
