<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

defineProps<{
  visible: boolean
  sourcePath: string
  targetPath: string
}>()

const emit = defineEmits<{
  close: []
  sync: [options: SyncOptions]
}>()

export interface SyncOptions {
  direction: 'upload' | 'download' | 'bidirectional'
  compareSize: boolean
  compareTime: boolean
  includePattern: string
  excludePattern: string
  deleteOrphans: boolean
}

const direction = ref<'upload' | 'download' | 'bidirectional'>('upload')
const compareSize = ref(true)
const compareTime = ref(true)
const includePattern = ref('*')
const excludePattern = ref('')
const deleteOrphans = ref(false)

// Preview diff
interface DiffEntry {
  name: string
  action: 'copy' | 'update' | 'delete' | 'skip'
  size: string
  modified: string
}

const previewEntries = ref<DiffEntry[]>([])
const showPreview = ref(false)

function generatePreview() {
  // Simulated preview — in real implementation this would call a backend API
  // For now show a placeholder
  previewEntries.value = [
    { name: 'index.html', action: 'update', size: '2.1 KB', modified: new Date().toISOString().slice(0, 10) },
    { name: 'new-file.js', action: 'copy', size: '0.5 KB', modified: new Date().toISOString().slice(0, 10) },
  ]
  if (deleteOrphans.value) {
    previewEntries.value.push({ name: 'old-file.bak', action: 'delete', size: '1.2 KB', modified: '2026-07-15' })
  }
  showPreview.value = true
}

function onSync() {
  emit('sync', {
    direction: direction.value,
    compareSize: compareSize.value,
    compareTime: compareTime.value,
    includePattern: includePattern.value,
    excludePattern: excludePattern.value,
    deleteOrphans: deleteOrphans.value,
  })
}

function actionLabel(a: string) {
  if (a === 'copy') return t('files.copy')
  if (a === 'update') return t('files.update')
  if (a === 'delete') return t('files.delete')
  return t('files.skip')
}

function actionClass(a: string) {
  if (a === 'delete') return 'action-delete'
  if (a === 'update') return 'action-update'
  return 'action-copy'
}
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="fsd-overlay" @click.self="emit('close')">
      <div class="fsd-dialog">
        <div class="fsd-header">
          <span class="fsd-title">{{ t('files.folderSync') }}</span>
          <button class="fsd-close" @click="emit('close')">×</button>
        </div>

        <div class="fsd-body">
          <!-- Source / Target -->
          <div class="fsd-paths">
            <div class="fsd-path">
              <span class="fsd-path-label">{{ t('files.source') }}:</span>
              <span class="fsd-path-value mono">{{ sourcePath }}</span>
            </div>
            <div class="fsd-path">
              <span class="fsd-path-label">{{ t('files.target') }}:</span>
              <span class="fsd-path-value mono">{{ targetPath }}</span>
            </div>
          </div>

          <!-- Direction -->
          <div class="fsd-section">
            <label class="fsd-label">{{ t('files.direction') }}</label>
            <div class="fsd-radio-group">
              <label class="fsd-radio"><input v-model="direction" type="radio" value="upload" /> {{ t('files.uploadToRemote') }}</label>
              <label class="fsd-radio"><input v-model="direction" type="radio" value="download" /> {{ t('files.downloadToLocal') }}</label>
              <label class="fsd-radio"><input v-model="direction" type="radio" value="bidirectional" /> {{ t('files.bidirectional') }}</label>
            </div>
          </div>

          <!-- Compare By -->
          <div class="fsd-section">
            <label class="fsd-label">{{ t('files.compareBy') }}</label>
            <div class="fsd-check-group">
              <label class="fsd-check"><input v-model="compareSize" type="checkbox" /> {{ t('files.size') }}</label>
              <label class="fsd-check"><input v-model="compareTime" type="checkbox" /> {{ t('files.modifiedTime') }}</label>
            </div>
          </div>

          <!-- Include / Exclude -->
          <div class="fsd-section fsd-row">
            <div class="fsd-field">
              <label class="fsd-label">{{ t('files.include') }}</label>
              <input v-model="includePattern" class="fsd-input mono" placeholder="*.html,*.css,*.js" />
            </div>
            <div class="fsd-field">
              <label class="fsd-label">{{ t('files.exclude') }}</label>
              <input v-model="excludePattern" class="fsd-input mono" placeholder="node_modules/**" />
            </div>
          </div>

          <!-- Delete Orphans -->
          <div class="fsd-section">
            <label class="fsd-check"><input v-model="deleteOrphans" type="checkbox" /> {{ t('files.deleteOrphans') }}</label>
          </div>

          <!-- Preview -->
          <div v-if="showPreview" class="fsd-preview">
            <div class="fsd-label">{{ t('files.syncPreview') }} ({{ previewEntries.length }} {{ t('files.previewChanges') }})</div>
            <table class="fsd-table">
              <thead><tr><th>#</th><th>{{ t('files.file') }}</th><th>{{ t('files.action') }}</th><th class="fsd-col-size">{{ t('files.size') }}</th><th class="fsd-col-modified">{{ t('files.modified') }}</th></tr></thead>
              <tbody>
                <tr v-for="(entry, i) in previewEntries" :key="i">
                  <td class="muted">{{ i + 1 }}</td>
                  <td class="mono">{{ entry.name }}</td>
                  <td><span class="fsd-action" :class="actionClass(entry.action)">{{ actionLabel(entry.action) }}</span></td>
                  <td class="muted fsd-col-size">{{ entry.size }}</td>
                  <td class="muted fsd-col-modified">{{ entry.modified }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div class="fsd-footer">
          <button class="fsd-btn" @click="emit('close')">{{ t('files.cancel') }}</button>
          <button class="fsd-btn" @click="generatePreview">{{ t('files.syncPreview') }}</button>
          <button class="fsd-btn fsd-btn--primary" @click="onSync">{{ t('files.startSync') }}</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.fsd-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
}

.fsd-dialog {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  min-width: 520px;
  max-width: 700px;
  display: flex;
  flex-direction: column;
}

.fsd-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}

.fsd-title {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
}

.fsd-close {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-xl);
}

.fsd-close:hover {
  color: var(--text-primary);
}

.fsd-body {
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.fsd-paths {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.fsd-path {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-sm);
}

.fsd-path-label {
  color: var(--text-muted);
  flex-shrink: 0;
}

.fsd-path-value {
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.fsd-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.fsd-row {
  flex-direction: row;
  gap: var(--space-4);
}

.fsd-field {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.fsd-label {
  font-size: var(--text-xs);
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
}

.fsd-input {
  padding: var(--space-2);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}

.fsd-input:focus {
  border-color: var(--accent);
}

.fsd-radio-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.fsd-radio {
  font-size: var(--text-sm);
  color: var(--text-primary);
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.fsd-radio input {
  accent-color: var(--accent);
}

.fsd-check-group {
  display: flex;
  gap: var(--space-4);
}

.fsd-check {
  font-size: var(--text-sm);
  color: var(--text-primary);
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.fsd-check input {
  accent-color: var(--accent);
}

/* ---- preview table ---- */
.fsd-preview {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--space-3);
}

.fsd-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-sm);
  margin-top: var(--space-2);
}

.fsd-table th,
.fsd-table td {
  padding: var(--space-1) var(--space-2);
  text-align: left;
  border-bottom: 1px solid var(--border);
}

.fsd-table th {
  font-size: var(--text-xs);
  color: var(--text-muted);
  text-transform: uppercase;
}

.fsd-action {
  font-size: var(--text-xs);
  padding: 1px 6px;
  border-radius: 3px;
  font-weight: 500;
}

.action-copy {
  background: rgba(63, 185, 80, 0.15);
  color: #3FB950;
}

.action-update {
  background: rgba(232, 145, 45, 0.15);
  color: #E8912D;
}

.action-delete {
  background: rgba(248, 81, 73, 0.15);
  color: #F85149;
}

.muted {
  color: var(--text-muted);
}

/* ---- footer ---- */
.fsd-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-top: 1px solid var(--border);
}

.fsd-btn {
  padding: var(--space-2) var(--space-4);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--text-sm);
  cursor: pointer;
}

.fsd-btn:hover {
  background: var(--bg-hover);
}

.fsd-btn--primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.fsd-btn--primary:hover {
  opacity: 0.9;
}

@media (max-width: 768px) {
  .fsd-dialog {
    min-width: auto;
    width: 95vw;
    max-width: 520px;
  }
  .fsd-row {
    flex-direction: column;
    gap: var(--space-2);
  }
  .fsd-table .fsd-col-size,
  .fsd-table .fsd-col-modified,
  .fsd-table th:nth-child(4),
  .fsd-table th:nth-child(5),
  .fsd-table td:nth-child(4),
  .fsd-table td:nth-child(5) {
    display: none;
  }
}
</style>
