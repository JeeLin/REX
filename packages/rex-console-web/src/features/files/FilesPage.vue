<script setup lang="ts">
//! FilesPage.vue — rendering layer (v0.91.0 T6).
//! Delegates all behaviour to useFiles / useTransfer composables; this file
//! only wires props → options, exposes a notify → toast bridge, and renders
//! the template using child components (FilesToolbar, FilesGrid, MobileFilesBar,
//! FileEditorDialog, FilePreview) plus inline dialogs (chmod, ACL, delete).

import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import FilesToolbar from './FilesToolbar.vue'
import FilesGrid from './FilesGrid.vue'
import MobileFilesBar from './MobileFilesBar.vue'
import FileEditorDialog from './FileEditorDialog.vue'
import FilePreview from './FilePreview.vue'
import FolderSyncDialog from './FolderSyncDialog.vue'
import Toast from '@/components/ui/Toast.vue'
import Button from '@/components/ui/Button.vue'
import { fmtSize } from '@/features/files/format'
import { useFiles } from '@/features/files/composables/useFiles'
import { useTransfer } from '@/features/files/composables/useTransfer'
import type { ToastTone } from '@/features/files/types'

const { t } = useI18n()

const props = defineProps<{
  resourceId?: string
  protocol?: 'sftp' | 's3'
  tabId?: string
}>()
const emit = defineEmits<{
  'update:status': [status: string]
}>()

const toast = ref<InstanceType<typeof Toast> | null>(null)
function notify(message: string, tone: ToastTone) {
  toast.value?.push(message, tone)
}

const transfer = useTransfer()

const editorProtocol = computed(() => connProtocol.value as 'sftp' | 's3')

const {  sessionId,
  showConnect,
  connProtocol,
  showDeleteConfirm,
  confirmDelete,
  executeDelete,
  cancelDelete,
  ctx,
  ctxRef,
  onCtx,
  ctxCopy,
  ctxPresignedUrl,
  ctxDelete,
  showSyncDialog,
  syncSource,
  syncTarget,
  openSync,
  closeSync,
  hasCap,
  canShowDualPanel,
  panels,
  mobileActiveSide,
  loadPanel,
  navigate,
  activate,
  goUp,
  toggleSelect,
  syncBrowsing,
  renamingId,
  renameValue,
  startRename,
  submitRename,
  cancelRename,
  isRenaming,
  downloadSelected,
  uploadTo,
  newFolder,
  mfbNewFolder,
  mfbRename,
  mfbDelete,
  mfbPermissions,
  mfbCopyPath,
  mfbSelectedCount,
  showAclDialog,
  aclPath,
  aclValue,
  openAclDialog,
  applyAcl,
  showChmod,
  chmodPath,
  chmodPerms,
  openChmod,
  calcOctal,
  applyChmod,
  editorVisible,
  editorFilePath,
  editFile,
  onEditorSaved,
  previewVisible,
  previewFile,
  isPreviewable,
  activateEntry,
  openPreview,
  leftW,
  dragging,
  onDS,
  onDE,
  dragData,
  dropTarget,
  onDragStart,
  onDragOver,
  onDragLeave,
  onDrop,
  onDragEnd,
} = useFiles({
  resourceId: props.resourceId,
  protocol: props.protocol || 'sftp',
  tabId: props.tabId,
  onStatus: (status) => emit('update:status', status),
  notify,
})
</script>

<template>
  <div class="fp" @mousemove.prevent>
    <!-- Mobile panel switcher -->
    <div class="fp-switcher">
      <button
        class="fp-switcher-btn"
        :class="{ 'fp-switcher-btn--active': mobileActiveSide === 'left' }"
        @click="mobileActiveSide = 'left'"
      >
        {{ t('files.left') }}
      </button>
      <button
        class="fp-switcher-btn"
        :class="{ 'fp-switcher-btn--active': mobileActiveSide === 'right' }"
        @click="mobileActiveSide = 'right'"
      >
        {{ t('files.right') }}
      </button>
    </div>

    <div v-if="showConnect" class="fp-overlay">
      <div class="fp-dialog">
        <h3>{{ t('files.connectToServer') }}</h3>
        <p style="color: var(--text-secondary); margin-bottom: 12px;">
          {{ t('files.connect_via_workspace') }}
        </p>
      </div>
    </div>

    <template v-for="side in (['left', 'right'] as const)" :key="side">
      <!-- S3 single-bucket/prefix model: dual-panel is pointless → render left only -->
      <div
        v-if="!(canShowDualPanel !== true && side === 'right')"
        class="fp-panel"
        :class="{
          'fp-panel--active': panels[side].active,
          'fp-panel--drop': dropTarget === side,
          'fp-panel--mobile-hidden': mobileActiveSide !== side,
        }"
        :style="canShowDualPanel ? (side === 'left' ? { width: leftW + 'px' } : { flex: '1', minWidth: '0' }) : { flex: '1', minWidth: '0' }"
        @click="activate(side)"
        @dragover="onDragOver($event, side)"
        @dragleave="onDragLeave"
        @drop="onDrop($event, side)"
      >
        <FilesToolbar
          :path="panels[side].path"
          :sync-browsing="syncBrowsing"
          @go-up="goUp(side)"
          @toggle-sync="syncBrowsing = !syncBrowsing"
          @upload="uploadTo(side)"
          @refresh="loadPanel(side)"
        />
        <FilesGrid
          :side="side"
          :panel="panels[side]"
          :renaming-id="renamingId"
          :rename-value="renameValue"
          :show-storage-class="hasCap('acl')"
          @select="toggleSelect"
          @activate="activateEntry"
          @context="onCtx"
          @update:rename-value="renameValue = $event"
          @rename-submit="submitRename"
          @rename-cancel="cancelRename"
          @drag-start="onDragStart"
          @drag-end="onDragEnd"
        />
        <div v-if="panels[side].selected.size > 0" class="batch-bar">
          <span class="batch-bar-count">{{ panels[side].selected.size }} {{ t('files.selected') }}</span>
          <div class="batch-bar-actions">
            <Button variant="ghost" icon title="Download selected" @click="downloadSelected(side)">⬇</Button>
            <Button variant="danger" icon title="Delete selected" @click="confirmDelete(side)">🗑</Button>
          </div>
        </div>
        <div class="ps">
          {{ panels[side].entries.length }} {{ t('files.items') }}
          <template v-if="panels[side].selected.size">
            · {{ panels[side].selected.size }} {{ t('files.selected') }}
          </template>
        </div>
      </div>
      <div
        v-if="side === 'left' && canShowDualPanel"
        class="fh2"
        :class="{ 'fh2--a': dragging }"
        @mousedown.prevent="onDS"
      />
    </template>

    <!-- Context menu -->
    <div
      v-if="ctx.show"
      ref="ctxRef"
      class="fctx"
      :style="{ top: ctx.y + 'px', left: ctx.x + 'px' }"
    >
      <div class="ci" @click="editFile(ctx.path)">{{ t('files.edit') }}</div>
      <div
        class="ci"
        @click="
          (() => {
            const entry = panels[ctx.side].entries.find((e) => e.name === ctx.name)
            if (entry) startRename(ctx.side, entry)
          })()
        "
      >
        {{ t('files.rename') }}
      </div>
      <div class="ci" @click="ctxCopy">{{ t('files.copyPath') }}</div>
      <div
        v-if="hasCap('presigned_url')"
        class="ci"
        @click="ctxPresignedUrl"
      >
        {{ t('files.copyPresignedUrl') }}
      </div>
      <div
        class="ci"
        @click="hasCap('acl') ? openAclDialog(ctx.path) : openChmod(ctx.path)"
      >
        {{ t('files.permissions') }}
      </div>
      <!-- PRODUCT §3.8：文件夹右键才有「同步」入口（文件无同步语义）。 -->
      <div v-if="ctx.isDir" class="ci" @click="openSync">{{ t('files.folderSync') }}</div>
      <div class="ci ci--d" @click="ctxDelete">{{ t('files.delete') }}</div>
    </div>

    <!-- Chmod Modal -->
    <Teleport to="body">
      <div v-if="showChmod" class="fp-overlay" @click.self="showChmod = false">
        <div class="fp-dialog">
          <h3>{{ t('files.permissions') }}: {{ chmodPath }}</h3>
          <div class="chmod-grid">
            <div class="chmod-header">
              <span></span>
              <span>{{ t('files.owner') }}</span>
              <span>{{ t('files.group') }}</span>
              <span>{{ t('files.others') }}</span>
            </div>
            <div class="chmod-row">
              <span>{{ t('files.read') }}</span>
              <input v-model="chmodPerms.owner.read" type="checkbox" />
              <input v-model="chmodPerms.group.read" type="checkbox" />
              <input v-model="chmodPerms.other.read" type="checkbox" />
            </div>
            <div class="chmod-row">
              <span>{{ t('files.write') }}</span>
              <input v-model="chmodPerms.owner.write" type="checkbox" />
              <input v-model="chmodPerms.group.write" type="checkbox" />
              <input v-model="chmodPerms.other.write" type="checkbox" />
            </div>
            <div class="chmod-row">
              <span>{{ t('files.execute') }}</span>
              <input v-model="chmodPerms.owner.exec" type="checkbox" />
              <input v-model="chmodPerms.group.exec" type="checkbox" />
              <input v-model="chmodPerms.other.exec" type="checkbox" />
            </div>
          </div>
          <div class="chmod-octal">{{ t('files.octal') }}: {{ calcOctal().toString(8) }}</div>
          <div style="display:flex;gap:var(--space-2);justify-content:flex-end">
            <Button variant="primary" @click="showChmod = false">{{ t('files.cancel') }}</Button>
            <Button variant="primary" @click="applyChmod">{{ t('files.apply') }}</Button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- S3 ACL Dialog -->
    <Teleport to="body">
      <div v-if="showAclDialog" class="fp-overlay" @click.self="showAclDialog = false">
        <div class="fp-dialog">
          <h3>{{ t('files.acl') }}: {{ aclPath }}</h3>
          <div style="margin:var(--space-3) 0">
            <label style="display:block;font-size:var(--text-sm);color:var(--text-muted);margin-bottom:var(--space-1)">{{ t('files.cannedAcl') }}</label>
            <select
              v-model="aclValue"
              style="width:100%;padding:var(--space-2);background:var(--bg-surface);border:1px solid var(--border);border-radius:var(--radius-sm);color:var(--text-primary);font-size:var(--text-sm)"
            >
              <option value="private">private</option>
              <option value="public-read">public-read</option>
              <option value="public-read-write">public-read-write</option>
              <option value="authenticated-read">authenticated-read</option>
            </select>
          </div>
          <div style="display:flex;gap:var(--space-2);justify-content:flex-end">
            <Button variant="ghost" @click="showAclDialog = false">{{ t('files.cancel') }}</Button>
            <Button variant="primary" @click="applyAcl">{{ t('files.apply') }}</Button>
          </div>
        </div>
      </div>
    </Teleport>

    <FileEditorDialog
      :visible="editorVisible"
      :session-id="sessionId || ''"
      :file-path="editorFilePath"
      :protocol="editorProtocol"
      @close="editorVisible = false"
      @saved="onEditorSaved"
    />

    <FilePreview
      :show="previewVisible"
      :file="previewFile"
      :session-id="sessionId || ''"
      @close="previewVisible = false"
    />

    <!-- Folder Sync Dialog: browser only configures options / reads the plan. -->
    <FolderSyncDialog
      :open="showSyncDialog"
      :source="syncSource"
      :target="syncTarget"
      @close="closeSync"
      @created="notify(t('files.syncCreated'), 'success')"
      @error="(msg) => notify(msg, 'error')"
    />

    <!-- Delete Confirmation -->
    <Teleport to="body">
      <div v-if="showDeleteConfirm" class="fp-overlay" @click.self="cancelDelete">
        <div class="fp-dialog">
          <h3>{{ t('files.confirmDelete') }}</h3>
          <p style="color:var(--text-secondary);margin:0">
            {{ t('files.deleteConfirm') }}
          </p>
          <div style="display:flex;gap:var(--space-2);justify-content:flex-end">
            <Button variant="ghost" @click="cancelDelete">{{ t('files.cancel') }}</Button>
            <Button variant="danger" @click="executeDelete">{{ t('files.delete') }}</Button>
          </div>
        </div>
      </div>
    </Teleport>

    <MobileFilesBar
      :selected-count="mfbSelectedCount"
      @upload="uploadTo(mobileActiveSide)"
      @download="downloadSelected(mobileActiveSide)"
      @new-folder="mfbNewFolder"
      @refresh="loadPanel(mobileActiveSide)"
      @rename="mfbRename"
      @delete="mfbDelete"
      @permissions="mfbPermissions"
      @copy-path="mfbCopyPath"
    />

    <Toast ref="toast" />

    <!-- Transfer Queue Toggle -->
    <button
      v-if="transfer.queueItems.length > 0"
      class="tq-toggle"
      @click="transfer.showTransferQueue = !transfer.showTransferQueue"
    >
      📥 {{ transfer.queueItems.length }}
      <span v-if="transfer.activeCount">· {{ transfer.activeCount }} {{ t('files.active') }}</span>
      <span v-if="transfer.completedCount" class="tq-badge tq-badge--done">{{ transfer.completedCount }} ✓</span>
    </button>

    <!-- Transfer Queue Panel -->
    <Teleport to="body">
      <Transition name="tq-slide">
        <div v-if="transfer.showTransferQueue" class="tq-panel">
          <div class="tq-header">
            <span>{{ t('files.transferQueue') }} ({{ transfer.queueItems.length }})</span>
            <div class="tq-header-actions">
              <button
                v-if="transfer.completedCount"
                class="tq-btn tq-btn--sm"
                @click="transfer.dismissCompleted"
              >
                {{ t('files.clearDone') }}
              </button>
              <button class="tq-btn tq-btn--sm" @click="transfer.showTransferQueue = false">✕</button>
            </div>
          </div>
          <div class="tq-list">
            <div
              v-for="item in transfer.queueItems"
              :key="item.id"
              class="tq-item"
              :class="`tq-item--${item.status}`"
            >
              <div class="tq-item-info">
                <span class="tq-item-type">
                  {{ item.kind === 'browser' ? (item.direction === 'up' ? '⬆' : '⬇') : item.op === 'move' ? '🔄' : '📄' }}
                </span>
                <div class="tq-item-details">
                  <span class="tq-item-name">{{ item.name }}</span>
                  <span class="tq-item-path">{{ transfer.taskPath(item) }}</span>
                </div>
              </div>
              <div class="tq-item-status">
                <!-- Progress bar for active transfers -->
                <template v-if="item.status === 'running'">
                  <div class="tq-progress">
                    <div class="tq-progress-bar" :style="{ width: item.progress + '%' }"></div>
                  </div>
                  <span class="tq-item-pct">{{ item.progress }}%</span>
                  <span v-if="item.speed > 0" class="tq-item-pct">{{ fmtSize(item.speed) }}/s</span>
                </template>
                <!-- Error: show error + retry (browser tasks only) -->
                <template v-else-if="item.status === 'error'">
                  <span class="tq-item-error" :title="item.error ?? undefined">{{ item.error || t('files.failed') }}</span>
                  <button
                    v-if="item.kind === 'browser'"
                    class="tq-btn tq-btn--retry"
                    @click="transfer.retryTransfer(item.id)"
                  >
                    ↻ {{ t('files.retry') }}
                  </button>
                </template>
                <!-- Done -->
                <template v-else-if="item.status === 'done'">
                  <span class="tq-item-done">✓</span>
                </template>
                <!-- Canceled -->
                <template v-else-if="item.status === 'canceled'">
                  <span class="tq-item-pending">{{ t('files.canceled', 'Canceled') }}</span>
                </template>
                <!-- Pending -->
                <template v-else-if="item.status === 'pending'">
                  <span class="tq-item-pending">{{ t('files.waiting') }}</span>
                </template>
              </div>
            </div>
            <div v-if="!transfer.queueItems.length" class="tq-empty">{{ t('files.noTransfers') }}</div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.fp{display:flex;height:100%;background:var(--bg-page);position:relative}
.fp-overlay{position:fixed;inset:0;z-index:100;display:flex;align-items:center;justify-content:center;background:rgba(0,0,0,.6)}
.fp-dialog{background:var(--bg-elevated);border:1px solid var(--border);border-radius:var(--radius);padding:var(--space-5);min-width:340px;display:flex;flex-direction:column;gap:var(--space-3)}
.fp-dialog h3{margin:0;color:var(--text-primary)}
.f{display:flex;flex-direction:column;gap:var(--space-1)}
.f label{font-size:var(--text-xs);color:var(--text-muted);text-transform:uppercase}
.f input,.f select{padding:var(--space-2);background:var(--bg-deep);border:1px solid var(--border);border-radius:var(--radius-sm);color:var(--text-primary);font-size:var(--text-sm);outline:none}
.f input:focus,.f select:focus{border-color:var(--accent)}
.err{color:var(--danger);font-size:var(--text-sm)}
.fp-panel{display:flex;flex-direction:column;border-right:1px solid var(--border);overflow:hidden;flex-shrink:0}
.fp-panel--active{border-left:2px solid var(--accent)}
.fp-panel--drop{background:var(--accent-soft);outline:2px dashed var(--accent);outline-offset:-2px}
.ptb{display:flex;align-items:center;gap:var(--space-1);padding:var(--space-1) var(--space-2);border-bottom:1px solid var(--border);background:var(--bg-surface)}
.pb--active{color:var(--accent);background:var(--accent-soft)}
.pp{flex:1;font-size:var(--text-xs);color:var(--text-secondary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.pf{flex:1;overflow-y:auto}
.fr{display:flex;padding:var(--space-1) var(--space-3);font-size:var(--text-sm);cursor:pointer}
.fr:hover{background:var(--bg-hover)}
.fr--sel{background:var(--bg-hover);border-left:2px solid var(--accent)}
.fh{font-weight:600;color:var(--text-muted);font-size:var(--text-xs);text-transform:uppercase;cursor:default}
.fh:hover{background:none}
.cn{flex:1;display:flex;align-items:center;gap:var(--space-2);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.cs{width:80px;text-align:right}
.cm{width:140px;text-align:right}
.csc{width:100px;text-align:right}
.fi{font-size:14px}
.mu{color:var(--text-muted)}
.pe{padding:var(--space-4);text-align:center;color:var(--text-muted);font-size:var(--text-sm)}
.ps{padding:var(--space-1) var(--space-3);font-size:var(--text-xs);color:var(--text-muted);border-top:1px solid var(--border);background:var(--bg-surface)}
.fh2{width:4px;cursor:col-resize;background:var(--border);flex-shrink:0}
.fh2:hover,.fh2--a{background:var(--accent)}
.fctx{position:fixed;z-index:200;min-width:160px;background:var(--bg-elevated);border:1px solid var(--border);border-radius:var(--radius);box-shadow:var(--shadow);padding:var(--space-1) 0}
.ci{padding:var(--space-2) var(--space-3);font-size:var(--text-sm);cursor:pointer;color:var(--text-primary)}
.ci:hover{background:var(--bg-hover)}
.ci--d{color:var(--danger)}
.chmod-grid{display:grid;grid-template-columns:auto 1fr 1fr 1fr;gap:var(--space-2);margin:var(--space-3) 0}
.chmod-header{display:contents;font-weight:600;font-size:var(--text-xs);color:var(--text-muted);text-transform:uppercase}
.chmod-header span{text-align:center}
.chmod-row{display:contents}
.chmod-row span{font-size:var(--text-sm);color:var(--text-primary)}
.chmod-row input[type="checkbox"]{margin:0 auto;accent-color:var(--accent)}
.chmod-octal{text-align:center;font-family:var(--font-mono);font-size:var(--text-lg);color:var(--accent);margin:var(--space-3) 0}
.fp-rename-input{flex:1;background:var(--bg-deep);border:1px solid var(--accent);border-radius:2px;color:var(--text-primary);font-size:var(--text-sm);padding:0 4px;outline:none;min-width:0}
.fp-switcher{display:none}
@media(max-width:768px){
  .fp{flex-direction:column}
  .fp-switcher{display:flex;gap:0;border-bottom:1px solid var(--border);background:var(--bg-surface);flex-shrink:0}
  .fp-switcher-btn{flex:1;padding:var(--space-2);background:none;border:none;color:var(--text-muted);font-size:var(--text-sm);cursor:pointer;border-bottom:2px solid transparent}
  .fp-switcher-btn--active{color:var(--accent);border-bottom-color:var(--accent);background:var(--accent-soft)}
  .fp-panel--mobile-hidden{display:none !important}
  .fh2{display:none !important}
  .cm{display:none !important}
  .csc{display:none !important}
  .fp-panel{border-right:none !important}
  .fp-dialog{min-width:auto;width:90vw;max-width:340px}
  .fp{padding-bottom:56px}
  .ptb{gap:2px;padding:var(--space-1)}
  .pb{padding:4px;font-size:var(--text-xs)}
  .pp{font-size:11px}
}

/* Transfer queue toggle button */
.tq-toggle{position:fixed;bottom:var(--space-4);right:var(--space-4);z-index:90;display:flex;align-items:center;gap:var(--space-2);padding:var(--space-2) var(--space-3);background:var(--bg-elevated);border:1px solid var(--border);border-radius:var(--radius);cursor:pointer;font-size:var(--text-sm);color:var(--text-primary);box-shadow:var(--shadow);transition:border-color 0.15s}
.tq-toggle:hover{border-color:var(--accent)}
.tq-badge{font-size:var(--text-xs);padding:1px 6px;border-radius:var(--radius-sm)}
.tq-badge--done{background:var(--success-soft);color:var(--success)}

/* Transfer queue panel */
.tq-panel{position:fixed;bottom:0;right:0;z-index:200;width:380px;max-height:50vh;background:var(--bg-elevated);border-top:1px solid var(--border);border-left:1px solid var(--border);border-radius:var(--radius) var(--radius) 0 0;display:flex;flex-direction:column;box-shadow:0 -4px 16px rgba(0,0,0,0.25)}
.tq-header{display:flex;align-items:center;justify-content:space-between;padding:var(--space-2) var(--space-3);border-bottom:1px solid var(--border);font-size:var(--text-sm);font-weight:600;color:var(--text-primary)}
.tq-header-actions{display:flex;gap:var(--space-2)}
.tq-list{flex:1;overflow-y:auto;padding:var(--space-1) 0}
.tq-item{display:flex;align-items:flex-start;justify-content:space-between;padding:var(--space-2) var(--space-3);font-size:var(--text-sm);border-bottom:1px solid var(--border);gap:var(--space-3)}
.tq-item:last-child{border-bottom:none}
.tq-item--error{background:rgba(239,68,68,0.04)}
.tq-item--done{opacity:0.6}
.tq-item-info{display:flex;align-items:center;gap:var(--space-2);min-width:0;flex:1}
.tq-item-type{font-size:14px;flex-shrink:0}
.tq-item-details{display:flex;flex-direction:column;min-width:0}
.tq-item-name{color:var(--text-primary);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.tq-item-path{font-size:var(--text-xs);color:var(--text-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.tq-item-status{display:flex;align-items:center;gap:var(--space-2);flex-shrink:0}
.tq-item-pct{font-size:var(--text-xs);color:var(--text-muted);min-width:36px;text-align:right}
.tq-item-error{font-size:var(--text-xs);color:var(--danger);max-width:120px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.tq-item-done{color:var(--success);font-weight:600}
.tq-item-pending{font-size:var(--text-xs);color:var(--text-muted)}

/* Progress bar */
.tq-progress{width:60px;height:4px;background:var(--bg-deep);border-radius:2px;overflow:hidden}
.tq-progress-bar{height:100%;background:var(--accent);border-radius:2px;transition:width 0.2s}

/* Buttons */
.tq-btn{padding:var(--space-1) var(--space-2);background:var(--bg-hover);border:1px solid var(--border);border-radius:var(--radius-sm);cursor:pointer;font-size:var(--text-xs);color:var(--text-primary);white-space:nowrap}
.tq-btn:hover{background:var(--bg-deep)}
.tq-btn--retry{color:var(--accent);border-color:var(--accent)}
.tq-btn--retry:hover{background:var(--accent-soft)}
.tq-btn--sm{padding:2px var(--space-2);font-size:var(--text-xs)}
.tq-empty{padding:var(--space-4);text-align:center;color:var(--text-muted);font-size:var(--text-sm)}

/* Slide transition */
.tq-slide-enter-active,.tq-slide-leave-active{transition:transform 0.2s ease,opacity 0.2s ease}
.tq-slide-enter-from,.tq-slide-leave-to{transform:translateY(100%);opacity:0}

@media(max-width:768px){
  .tq-panel{width:100%;max-height:60vh}
  .tq-toggle{bottom:60px}
}

/* Batch action bar */
.batch-bar{display:flex;align-items:center;justify-content:space-between;padding:var(--space-1) var(--space-3);background:var(--accent-soft);border-top:1px solid var(--accent);font-size:var(--text-xs);min-height:28px}
.batch-bar-count{color:var(--accent);font-weight:600}
.batch-bar-actions{display:flex;gap:var(--space-1)}

/* Enhanced selected file highlight */
.fr--sel{background:var(--accent-soft) !important;border-left:2px solid var(--accent) !important}
</style>
