//! Files operations logic (v0.91.0 T6).
//! Extracted from FilesPage.vue: panel browsing, navigation, selection, delete,
//! rename, new-folder, download/upload, drag & drop, context menu, chmod, ACL,
//! editor/preview state, resize, and keyboard shortcuts. FilesPage.vue keeps the
//! rendering and delegates behaviour here.
//!
//! Capabilities (chmod / presigned / ACL / multipart) are fetched from
//! GET /api/files/connector/{resource_id}/capability and gate the UI via
//! hasCap(), replacing the previous protocol-based isS3 branching.

import { ref, reactive, computed, onMounted, onBeforeUnmount, inject } from 'vue'
import { useI18n } from 'vue-i18n'
import { onClickOutside } from '@vueuse/core'
import * as filesApi from '@/api/files'
import type { FileEntry, FileCapabilities, FileCapability } from '@/api/files'
import { PANE_CTX, type PaneCtx } from '@/features/workspace/paneContext'
import { ownsKeystroke } from '@/features/workspace/paneOwnership'
import { useTransferStore } from '@/stores/transfer'
import { clipboard } from '@/utils/clipboard'
import { isTypingTarget } from '@/utils/isTypingTarget'
import type { Side, FilesPanel, UseFilesOptions } from '@/features/files/types'

const PART_SIZE = 5 * 1024 * 1024 // 5MB
const IMAGE_EXTS = /\.(png|jpe?g|gif|webp|bmp|svg|ico)(\?|$)/i
const TEXT_EXTS = /\.(txt|md|json|js|ts|tsx|jsx|vue|css|scss|less|html|xml|yaml|yml|toml|ini|cfg|conf|sh|bash|zsh|py|rb|go|rs|java|c|cpp|h|hpp|sql|log|csv|env|makefile|dockerfile|docker-compose)(\?|$)/i

function otherSide(side: Side): Side {
  return side === 'left' ? 'right' : 'left'
}

export function useFiles(opts: UseFilesOptions) {
  const { resourceId, protocol, tabId, onStatus, notify } = opts
  const { t } = useI18n()
  const store = useTransferStore()
  const paneCtx = inject<PaneCtx | null>(PANE_CTX, null)

  // --- Connection ---
  const sessionId = ref<string | null>(null)
  const showConnect = ref(!resourceId)
  const connProtocol = ref(protocol || 'sftp')
  const connError = ref('')
  const connLoading = ref(false)
  const capabilities = ref<FileCapabilities | null>(null)

  /** True when the connector reports the given capability. */
  function hasCap(cap: FileCapability): boolean {
    return capabilities.value?.[cap] ?? false
  }

  /** S3 single-bucket/prefix model renders as one panel; other protocols (SFTP)
   * render the dual-panel layout. Derived from the presigned_url capability. */
  const canShowDualPanel = computed(() => !hasCap('presigned_url'))

  async function doConnect() {
    if (!resourceId) return
    connLoading.value = true
    connError.value = ''
    onStatus('connecting')
    try {
      sessionId.value = await filesApi.connect(resourceId)
      showConnect.value = false
      onStatus('online')
      await fetchCapability()
      store.connectWs()
      await loadPanel('left')
      await loadPanel('right')
    } catch (e: unknown) {
      connError.value = e instanceof Error ? e.message : String(e)
      onStatus('error')
    } finally {
      connLoading.value = false
    }
  }

  /** Fetch the connector's static capabilities (replaces isS3). Non-fatal on failure. */
  async function fetchCapability() {
    if (!resourceId) {
      capabilities.value = null
      return
    }
    try {
      const resp = await filesApi.getCapability(resourceId)
      capabilities.value = resp.capabilities
      if (resp.protocol) connProtocol.value = resp.protocol
    } catch {
      // Capability endpoint unavailable: fall back to protocol prop.
      capabilities.value = null
    }
  }

  // --- Panels ---
  const panels = reactive<{ left: FilesPanel; right: FilesPanel }>({
    left: {
      path: '/',
      entries: [] as FileEntry[],
      loading: false,
      selected: new Set<string>(),
      active: true,
    },
    right: {
      path: '/',
      entries: [] as FileEntry[],
      loading: false,
      selected: new Set<string>(),
      active: false,
    },
  })
  const mobileActiveSide = ref<Side>('left')

  async function loadPanel(side: Side) {
    const p = panels[side]
    if (!sessionId.value) return
    p.loading = true
    try {
      p.entries = await filesApi.listFiles(sessionId.value, p.path)
    } catch {
      p.entries = []
    } finally {
      p.loading = false
    }
  }

  // Sync browsing
  const syncBrowsing = ref(false)

  function navigate(side: Side, entry: FileEntry) {
    if (entry.is_dir) {
      panels[side].path = entry.path.endsWith('/') ? entry.path : entry.path + '/'
      panels[side].selected.clear()
      loadPanel(side)
      if (syncBrowsing.value) {
        const other = otherSide(side)
        const currentPath = panels[side].path
        const basePath = panels[side].path.split('/').slice(0, -2).join('/') + '/'
        const relativePath = currentPath.replace(basePath, '')
        if (relativePath && relativePath !== currentPath) {
          const targetPath = panels[other].path + relativePath
          panels[other].path = targetPath
          panels[other].selected.clear()
          loadPanel(other)
        }
      }
    }
  }

  function activate(side: Side) {
    panels.left.active = side === 'left'
    panels.right.active = side === 'right'
  }

  function goUp(side: Side) {
    const parts = panels[side].path.replace(/\/$/, '').split('/')
    parts.pop()
    panels[side].path = parts.length ? parts.join('/') + '/' : '/'
    panels[side].selected.clear()
    loadPanel(side)
  }

  function toggleSelect(side: Side, name: string, e: MouseEvent) {
    const sel = panels[side].selected
    if (e.shiftKey && sel.size > 0) {
      const entries = panels[side].entries
      const last = Array.from(sel).pop()!
      const si = entries.findIndex((x) => x.name === last)
      const ei = entries.findIndex((x) => x.name === name)
      const [a, b] = si < ei ? [si, ei] : [ei, si]
      for (let i = a; i <= b; i++) sel.add(entries[i]!.name)
    } else if (e.ctrlKey || e.metaKey) {
      if (sel.has(name)) sel.delete(name)
      else sel.add(name)
    } else {
      sel.clear()
      sel.add(name)
    }
    panels[side].selected = new Set(sel)
  }

  // --- Delete confirmation ---
  const showDeleteConfirm = ref(false)
  const pendingDelete = ref<{ side: Side; names: string[] } | null>(null)
  const pendingCtxDelete = ref(false)

  function confirmDelete(side: Side) {
    if (!sessionId.value || panels[side].selected.size === 0) return
    pendingDelete.value = { side, names: Array.from(panels[side].selected) }
    showDeleteConfirm.value = true
  }

  async function executeDelete() {
    if (!sessionId.value || !pendingDelete.value) return
    try {
      if (pendingCtxDelete.value) {
        await filesApi.deleteFile(sessionId.value, ctx.value.path)
        pendingCtxDelete.value = false
        await loadPanel('left')
        await loadPanel('right')
      } else {
        const { side, names } = pendingDelete.value
        for (const name of names) {
          const entry = panels[side].entries.find((e) => e.name === name)
          if (entry) await filesApi.deleteFile(sessionId.value, entry.path)
        }
        panels[side].selected.clear()
        loadPanel(side)
      }
      showDeleteConfirm.value = false
      pendingDelete.value = null
    } catch (e) {
      notify(
        t('files.deleteFailed', 'Delete failed') +
          (e instanceof Error ? `: ${e.message}` : ''),
        'error',
      )
    }
  }

  function cancelDelete() {
    showDeleteConfirm.value = false
    pendingDelete.value = null
  }

  function confirmCtxDelete() {
    pendingCtxDelete.value = true
    showDeleteConfirm.value = true
    pendingDelete.value = { side: ctx.value.side, names: [ctx.value.name] }
  }

  async function ctxDelete() {
    confirmCtxDelete()
    ctx.value.show = false
  }

  // --- Inline rename ---
  const renamingId = ref<string | null>(null)
  const renameValue = ref('')

  function startRename(side: Side, entry: FileEntry) {
    renamingId.value = `${side}:${entry.name}`
    renameValue.value = entry.name
    ctx.value.show = false
  }

  async function submitRename(side: Side) {
    if (!sessionId.value || !renamingId.value) return
    const entry = panels[side].entries.find((e) => `${side}:${e.name}` === renamingId.value)
    if (!entry) {
      renamingId.value = null
      return
    }
    const newName = renameValue.value.trim()
    if (newName && newName !== entry.name) {
      try {
        await filesApi.renameFile(
          sessionId.value,
          panels[side].path + entry.name,
          panels[side].path + newName,
        )
      } catch (e) {
        notify(
          t('files.renameFailed', 'Rename failed') +
            (e instanceof Error ? `: ${e.message}` : ''),
          'error',
        )
      }
    }
    renamingId.value = null
    await loadPanel(side)
  }

  function cancelRename() {
    renamingId.value = null
  }

  function isRenaming(side: Side, name: string) {
    return renamingId.value === `${side}:${name}`
  }

  // --- Keyboard ---
  function activeSide(): Side {
    return panels.left.active ? 'left' : 'right'
  }

  function onKeyDown(e: KeyboardEvent) {
    if (!ownsKeystroke(tabId, paneCtx)) return
    if (renamingId.value) return
    if (isTypingTarget(e.target)) return

    if (e.key === 'F2') {
      e.preventDefault()
      const side = activeSide()
      const sel = Array.from(panels[side].selected)
      if (sel.length === 1) {
        const entry = panels[side].entries.find((en) => en.name === sel[0])
        if (entry) startRename(side, entry)
      }
      return
    }
    if (e.key === 'F7') {
      e.preventDefault()
      newFolder(activeSide())
      return
    }
    if (e.key === 'F8' || e.key === 'Delete') {
      e.preventDefault()
      confirmDelete(activeSide())
      return
    }
    if (
      (e.ctrlKey || e.metaKey) &&
      !e.shiftKey &&
      !e.altKey &&
      (e.key === 'r' || e.key === 'R')
    ) {
      e.preventDefault()
      loadPanel('left')
      loadPanel('right')
    }
  }

  // --- Download / upload (browser-blob tasks via shared store) ---
  async function downloadSelected(side: Side) {
    if (!sessionId.value || panels[side].selected.size === 0) return
    const names = Array.from(panels[side].selected)
    for (const name of names) {
      const entry = panels[side].entries.find((e) => e.name === name)
      if (!entry || entry.is_dir) continue
      const id = `tr-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
      store.pushBrowserTask({
        id,
        direction: 'down',
        name: entry.name,
        size: entry.size,
        status: 'running',
        progress: 0,
        transferred: 0,
        source_path: entry.path,
        target_path: entry.name,
        sessionId: sessionId.value,
      })
      try {
        const blob = await filesApi.downloadFile(sessionId.value, entry.path)
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = entry.name
        a.click()
        URL.revokeObjectURL(url)
        store.updateBrowserTask(id, {
          status: 'done',
          progress: 100,
          transferred: entry.size || blob.size,
        })
      } catch (e) {
        store.updateBrowserTask(id, {
          status: 'error',
          error: e instanceof Error ? e.message : String(e),
        })
        notify(
          t('files.downloadFailed', 'Download failed') +
            (e instanceof Error ? `: ${e.message}` : ''),
          'error',
        )
      }
    }
  }

  async function uploadTo(side: Side) {
    if (!sessionId.value) return
    const input = document.createElement('input')
    input.type = 'file'
    input.multiple = true
    input.onchange = async () => {
      if (!sessionId.value) return
      for (const file of Array.from(input.files || [])) {
        const remotePath = panels[side].path + file.name
        const id = `tr-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
        store.pushBrowserTask({
          id,
          direction: 'up',
          name: file.name,
          size: file.size,
          status: 'running',
          progress: 0,
          transferred: 0,
          source_path: '',
          target_path: remotePath,
          sessionId: sessionId.value,
          file,
        })
        await runBrowserUpload(id, sessionId.value, remotePath, file)
      }
      loadPanel(side)
    }
    input.click()
  }

  /** Upload a browser-blob file and drive the shared-store task through progress events. */
  async function runBrowserUpload(
    id: string,
    sid: string,
    remotePath: string,
    file: File,
  ) {
    let lastLoaded = 0
    let lastTime = Date.now()
    try {
      if (file.size > PART_SIZE) {
        const result = await filesApi.uploadFileWithProgress(
          sid,
          remotePath,
          file,
          (_pct, loaded) => {
            const now = Date.now()
            const dt = (now - lastTime) / 1000
            if (dt >= 0.5) {
              store.updateBrowserTask(id, {
                transferred: loaded,
                progress:
                  file.size > 0
                    ? Math.round((loaded / file.size) * 100)
                    : 0,
                speed: Math.round((loaded - lastLoaded) / dt),
              })
              lastLoaded = loaded
              lastTime = now
            }
          },
        )
        if (result?.upload_id) {
          store.updateBrowserTask(id, { uploadId: result.upload_id })
        }
        store.updateBrowserTask(id, {
          status: 'done',
          progress: 100,
          transferred: file.size,
        })
      } else {
        await filesApi.uploadFile(sid, remotePath, file)
        store.updateBrowserTask(id, {
          status: 'done',
          progress: 100,
          transferred: file.size,
        })
      }
    } catch (e) {
      store.updateBrowserTask(id, {
        status: 'error',
        error: e instanceof Error ? e.message : String(e),
      })
      notify(
        t('files.uploadFailed', 'Upload failed') +
          (e instanceof Error ? `: ${e.message}` : ''),
        'error',
      )
    }
  }

  // --- Context menu ---
  const ctx = ref({
    show: false,
    x: 0,
    y: 0,
    path: '',
    name: '',
    side: 'left' as Side,
  })
  const ctxRef = ref<HTMLElement | null>(null)
  onClickOutside(ctxRef, () => {
    ctx.value.show = false
  })

  function onCtx(e: MouseEvent, entry: FileEntry, side: Side) {
    e.preventDefault()
    ctx.value = {
      show: true,
      x: e.clientX,
      y: e.clientY,
      path: entry.path,
      name: entry.name,
      side,
    }
  }

  function ctxCopy() {
    clipboard.writeText(ctx.value.path)
    ctx.value.show = false
  }

  async function ctxPresignedUrl() {
    if (!sessionId.value) return
    try {
      const url = await filesApi.presignedUrl(sessionId.value, ctx.value.path)
      clipboard.writeText(url)
      notify(t('files.presignedCopied', 'Presigned URL copied'), 'success')
    } catch (e) {
      notify(
        t('files.presignedFailed', 'Failed to generate presigned URL') +
          (e instanceof Error ? `: ${e.message}` : ''),
        'error',
      )
    }
    ctx.value.show = false
  }

  // --- New folder ---
  function newFolder(side: Side) {
    if (!sessionId.value) return
    const name = prompt(t('files.folderNamePrompt'))
    if (!name) return
    filesApi
      .mkdir(sessionId.value, panels[side].path + name)
      .then(() => loadPanel(side))
      .catch((e: unknown) => {
        notify(
          t('files.createFolderFailed', 'Failed to create folder') +
            (e instanceof Error ? `: ${e.message}` : ''),
          'error',
        )
      })
  }

  function mfbNewFolder() {
    newFolder(mobileActiveSide.value)
  }

  function mfbRename() {
    if (!sessionId.value) return
    const side = mobileActiveSide.value
    const sel = Array.from(panels[side].selected)
    if (sel.length !== 1) return
    const entry = panels[side].entries.find((e) => e.name === sel[0])
    if (!entry) return
    const newName = prompt(t('files.newNamePrompt'), entry.name)
    if (!newName || newName === entry.name) return
    filesApi
      .renameFile(sessionId.value, entry.path, panels[side].path + newName)
      .then(() => loadPanel(side))
  }

  function mfbDelete() {
    confirmDelete(mobileActiveSide.value)
  }

  function mfbPermissions() {
    const side = mobileActiveSide.value
    const sel = Array.from(panels[side].selected)
    if (sel.length !== 1) return
    const entry = panels[side].entries.find((e) => e.name === sel[0])
    if (entry) {
      if (hasCap('acl')) openAclDialog(entry.path)
      else openChmod(entry.path)
    }
  }

  function mfbCopyPath() {
    const side = mobileActiveSide.value
    const sel = Array.from(panels[side].selected)
    if (sel.length !== 1) return
    const entry = panels[side].entries.find((e) => e.name === sel[0])
    if (entry) clipboard.writeText(entry.path)
  }

  const mfbSelectedCount = computed(
    () => panels[mobileActiveSide.value].selected.size,
  )

  // --- ACL dialog (S3) ---
  const showAclDialog = ref(false)
  const aclPath = ref('')
  const aclValue = ref('private')

  function openAclDialog(path: string) {
    aclPath.value = path
    aclValue.value = 'private'
    showAclDialog.value = true
    if (sessionId.value) {
      filesApi
        .getAcl(sessionId.value, path)
        .then((acl) => {
          aclValue.value = acl
        })
        .catch(() => {})
    }
  }

  async function applyAcl() {
    if (!sessionId.value) return
    try {
      await filesApi.putAcl(sessionId.value, aclPath.value, aclValue.value)
      showAclDialog.value = false
      await loadPanel('left')
      await loadPanel('right')
    } catch (e) {
      notify(
        t('files.aclFailed', 'Failed to update ACL') +
          (e instanceof Error ? `: ${e.message}` : ''),
        'error',
      )
    }
  }

  // --- Chmod permissions (SFTP) ---
  const showChmod = ref(false)
  const chmodPath = ref('')
  const chmodPerms = reactive({
    owner: { read: true, write: true, exec: false },
    group: { read: true, write: false, exec: false },
    other: { read: false, write: false, exec: false },
  })

  function openChmod(path: string) {
    chmodPath.value = path
    showChmod.value = true
  }

  function calcOctal(): number {
    let octal = 0
    if (chmodPerms.owner.read) octal += 400
    if (chmodPerms.owner.write) octal += 200
    if (chmodPerms.owner.exec) octal += 100
    if (chmodPerms.group.read) octal += 40
    if (chmodPerms.group.write) octal += 20
    if (chmodPerms.group.exec) octal += 10
    if (chmodPerms.other.read) octal += 4
    if (chmodPerms.other.write) octal += 2
    if (chmodPerms.other.exec) octal += 1
    return octal
  }

  async function applyChmod() {
    if (!sessionId.value) return
    const octal = calcOctal()
    try {
      await filesApi.chmod(sessionId.value, chmodPath.value, octal.toString(8))
      showChmod.value = false
      await loadPanel('left')
      await loadPanel('right')
    } catch (e) {
      notify(
        t('files.chmodFailed', 'Failed to change permissions') +
          (e instanceof Error ? `: ${e.message}` : ''),
        'error',
      )
    }
  }

  // --- Edit file ---
  const editorVisible = ref(false)
  const editorFilePath = ref('')
  function editFile(path: string) {
    if (!sessionId.value) return
    editorFilePath.value = path
    editorVisible.value = true
    ctx.value.show = false
  }
  function onEditorSaved() {
    editorVisible.value = false
    loadPanel('left')
    loadPanel('right')
  }

  // --- File preview ---
  const previewVisible = ref(false)
  const previewFile = ref<{ name: string; path: string; mime?: string } | null>(
    null,
  )

  function isPreviewable(entry: FileEntry): boolean {
    if (entry.is_dir) return false
    return IMAGE_EXTS.test(entry.name) || TEXT_EXTS.test(entry.name)
  }

  /** Double-click a row: preview when previewable, otherwise navigate into dirs. */
  function activateEntry(side: Side, entry: FileEntry) {
    if (renamingId.value) return
    if (isPreviewable(entry)) openPreview(entry)
    else navigate(side, entry)
  }

  function openPreview(entry: FileEntry) {
    if (!isPreviewable(entry)) return
    previewFile.value = { name: entry.name, path: entry.path }
    previewVisible.value = true
  }

  // --- Resize ---
  const leftW = ref(400)
  const dragging = ref(false)
  let sx = 0
  let sw = 0
  function onDS(e: MouseEvent) {
    dragging.value = true
    sx = e.clientX
    sw = leftW.value
    document.addEventListener('mousemove', onDM)
    document.addEventListener('mouseup', onDE)
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
  }
  function onDM(e: MouseEvent) {
    leftW.value = Math.min(800, Math.max(250, sw + (e.clientX - sx)))
  }
  function onDE() {
    dragging.value = false
    document.removeEventListener('mousemove', onDM)
    document.removeEventListener('mouseup', onDE)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }

  // --- Drag & drop transfer ---
  const dragData = ref<{ side: Side; names: string[] } | null>(null)
  const dropTarget = ref<Side | null>(null)

  function onDragStart(e: DragEvent, side: Side, name: string) {
    const sel = panels[side].selected
    const allNames = sel.has(name) ? Array.from(sel) : [name]
    const names = allNames.filter((n) => {
      const entry = panels[side].entries.find((en) => en.name === n)
      return entry && !entry.is_dir
    })
    if (names.length === 0) return
    dragData.value = { side, names }
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'copy'
      e.dataTransfer.setData('text/plain', names.join(','))
    }
  }

  function onDragOver(e: DragEvent, side: Side) {
    e.preventDefault()
    if (dragData.value && dragData.value.side === side) return
    e.dataTransfer!.dropEffect = 'copy'
    dropTarget.value = side
  }

  function onDragLeave(e: DragEvent) {
    const related = e.relatedTarget as HTMLElement | null
    if (related && (e.currentTarget as HTMLElement).contains(related)) return
    dropTarget.value = null
  }

  async function handleExternalFileDrop(files: FileList, side: Side) {
    if (!sessionId.value) return
    for (const file of Array.from(files)) {
      const remotePath = panels[side].path + file.name
      const id = `tr-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
      store.pushBrowserTask({
        id,
        direction: 'up',
        name: file.name,
        size: file.size,
        status: 'running',
        progress: 0,
        source_path: '',
        target_path: remotePath,
        sessionId: sessionId.value,
        file,
      })
      await runBrowserUpload(id, sessionId.value, remotePath, file)
    }
    loadPanel(side)
  }

  async function onDrop(e: DragEvent, targetSide: Side) {
    e.preventDefault()
    dropTarget.value = null
    if (!sessionId.value) return
    if (e.dataTransfer?.files.length) {
      await handleExternalFileDrop(e.dataTransfer.files, targetSide)
      return
    }
    if (!dragData.value) return
    const { side: sourceSide, names } = dragData.value

    for (const name of names) {
      const srcEntry = panels[sourceSide].entries.find((en) => en.name === name)
      if (!srcEntry) continue
      if (srcEntry.is_dir) continue

      const srcPath = srcEntry.path
      const dstPath = panels[targetSide].path + name
      try {
        if (!resourceId) continue
        // Server-side move/copy: bytes never transit the browser.
        await store.copy(
          { resource_id: resourceId, path: srcPath },
          { resource_id: resourceId, path: dstPath },
        )
      } catch (err) {
        notify(
          t('files.transferFailed', 'Transfer failed') +
            (err instanceof Error ? `: ${err.message}` : ''),
          'error',
        )
      }
    }

    dragData.value = null
    loadPanel(targetSide)
  }

  function onDragEnd() {
    dragData.value = null
    dropTarget.value = null
  }

  // --- Lifecycle ---
  onMounted(async () => {
    document.addEventListener('keydown', onKeyDown)
    if (resourceId) {
      await doConnect()
    }
  })

  onBeforeUnmount(async () => {
    document.removeEventListener('keydown', onKeyDown)
    document.removeEventListener('mousemove', onDM)
    document.removeEventListener('mouseup', onDE)
    if (resourceId) {
      store.disconnectWs()
    }
    if (sessionId.value) {
      try {
        await filesApi.disconnect(sessionId.value)
      } catch {
        /* ignore */
      }
    }
  })

  return {
    // connection
    sessionId,
    showConnect,
    connProtocol,
    connError,
    connLoading,
    capabilities,
    hasCap,
    canShowDualPanel,
    // panels
    panels,
    mobileActiveSide,
    loadPanel,
    navigate,
    activate,
    goUp,
    toggleSelect,
    syncBrowsing,
    // delete
    showDeleteConfirm,
    confirmDelete,
    executeDelete,
    cancelDelete,
    // rename
    renamingId,
    renameValue,
    startRename,
    submitRename,
    cancelRename,
    isRenaming,
    // download / upload
    downloadSelected,
    uploadTo,
    // context menu
    ctx,
    ctxRef,
    onCtx,
    ctxCopy,
    ctxPresignedUrl,
    ctxDelete,
    // new folder / mobile
    newFolder,
    mfbNewFolder,
    mfbRename,
    mfbDelete,
    mfbPermissions,
    mfbCopyPath,
    mfbSelectedCount,
    // ACL dialog
    showAclDialog,
    aclPath,
    aclValue,
    openAclDialog,
    applyAcl,
    // chmod dialog
    showChmod,
    chmodPath,
    chmodPerms,
    openChmod,
    calcOctal,
    applyChmod,
    // editor
    editorVisible,
    editorFilePath,
    editFile,
    onEditorSaved,
    // preview
    previewVisible,
    previewFile,
    isPreviewable,
    activateEntry,
    openPreview,
    // resize
    leftW,
    dragging,
    onDS,
    onDE,
    // drag & drop
    dragData,
    dropTarget,
    onDragStart,
    onDragOver,
    onDragLeave,
    onDrop,
    onDragEnd,
  }
}
