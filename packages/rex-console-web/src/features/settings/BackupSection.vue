<template>
  <SettingsSection>
    <template #header>{{ t('settings.backup.title') }}</template>

    <!-- 导出区域 -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.backup.exportTitle') }}</div>
        <div class="settings-row-desc">{{ t('settings.backup.exportDesc') }}</div>
      </div>
    </div>

    <div class="backup-export">
      <!-- 环境选择 -->
      <div class="backup-option">
        <label class="backup-option-label">{{ t('settings.backup.exportScope') }}：</label>
        <div class="backup-radio-group">
          <label class="radio-label">
            <input v-model="exportScope" type="radio" value="all" />
            {{ t('settings.backup.allEnvs') }}
          </label>
          <label class="radio-label">
            <input v-model="exportScope" type="radio" value="selected" />
            {{ t('settings.backup.selectEnvs') }}
          </label>
        </div>
        <div v-if="exportScope === 'selected'" class="env-checkboxes">
          <label v-for="env in environments" :key="env.id" class="checkbox-label">
            <input v-model="selectedEnvIds" type="checkbox" :value="env.id" />
            {{ env.name }}
          </label>
        </div>
      </div>

      <!-- 加密选项 -->
      <div class="backup-option">
        <label class="checkbox-label">
          <input v-model="encryptBackup" type="checkbox" />
          {{ t('settings.backup.encryptBackup') }}
        </label>
        <div v-if="encryptBackup" class="password-inputs">
          <input
            v-model="backupPassword"
            type="password"
            :placeholder="t('settings.backup.setPassword')"
            class="form-input"
          />
          <input
            v-model="backupPasswordConfirm"
            type="password"
            :placeholder="t('settings.backup.confirmPassword')"
            class="form-input"
          />
          <div v-if="passwordMismatch" class="field-error">
            {{ t('settings.backup.passwordMismatch') }}
          </div>
        </div>
      </div>

      <button
        class="btn btn-primary"
        :disabled="exporting"
        @click="handleExport"
      >
        {{ exporting ? t('settings.backup.exporting') : t('settings.backup.exportBtn') }}
      </button>
    </div>

    <!-- 分隔线 -->
    <div class="backup-divider"></div>

    <!-- 导入区域 -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.backup.importTitle') }}</div>
        <div class="settings-row-desc">{{ t('settings.backup.importDesc') }}</div>
      </div>
    </div>

    <div class="backup-import">
      <!-- 文件上传 -->
      <div
        class="upload-area"
        :class="{ dragover }"
        @dragover.prevent="dragover = true"
        @dragleave="dragover = false"
        @drop.prevent="handleDrop"
        @click="triggerUpload"
      >
        <input
          ref="fileInput"
          type="file"
          accept=".json"
          hidden
          @change="handleFileSelect"
        />
        <div v-if="!importFile" class="upload-placeholder">
          📁 {{ t('settings.backup.dropFile') }}
        </div>
        <div v-else class="file-info">
          <span>📄 {{ importFile.name }} ({{ formatSize(importFile.size) }})</span>
          <button class="clear-btn" @click.stop="clearFile">✕</button>
        </div>
      </div>

      <!-- 密码输入（加密文件时显示） -->
      <div v-if="importFileEncrypted" class="backup-option">
        <label class="backup-option-label">{{ t('settings.backup.importPassword') }}：</label>
        <input
          v-model="importPassword"
          type="password"
          :placeholder="t('settings.backup.enterPassword')"
          class="form-input"
        />
      </div>

      <!-- 合并策略 -->
      <div class="backup-option">
        <label class="backup-option-label">{{ t('settings.backup.conflictStrategy') }}：</label>
        <select v-model="importStrategy" class="form-select">
          <option value="skip_existing">{{ t('settings.backup.skipExisting') }}</option>
          <option value="overwrite">{{ t('settings.backup.overwrite') }}</option>
        </select>
      </div>

      <!-- 预览按钮 -->
      <div class="backup-actions">
        <button
          class="btn"
          :disabled="!importFile || previewing"
          @click="handlePreview"
        >
          {{ previewing ? t('settings.backup.previewing') : t('settings.backup.previewBtn') }}
        </button>
        <button
          class="btn btn-primary"
          :disabled="!importFile || importing"
          @click="handleImport"
        >
          {{ importing ? t('settings.backup.importing') : t('settings.backup.importBtn') }}
        </button>
      </div>
    </div>

    <!-- 预览弹窗 -->
    <div v-if="previewResult" class="preview-modal-overlay" @click.self="previewResult = null">
      <div class="preview-modal">
        <div class="preview-header">
          <h4>{{ t('settings.backup.previewTitle') }}</h4>
          <button class="close-btn" @click="previewResult = null">✕</button>
        </div>
        <div class="preview-body">
          <div class="preview-meta">
            <span>{{ t('settings.backup.sourceVersion') }}: {{ previewResult.hub_version }}</span>
            <span>{{ t('settings.backup.backupTime') }}: {{ formatTime(previewResult.created_at) }}</span>
            <span v-if="previewResult.encrypted">🔒 {{ t('settings.backup.encrypted') }}</span>
          </div>

          <div class="preview-section">
            <div class="preview-section-title">
              {{ t('settings.backup.environments') }} ({{ previewResult.environments.length }})
            </div>
            <div v-for="env in previewResult.environments" :key="env.id" class="preview-item">
              <span :class="env.exists ? 'tag-exists' : 'tag-new'">
                {{ env.exists ? t('settings.backup.exists') : t('settings.backup.new') }}
              </span>
              {{ env.name }}
            </div>
          </div>

          <div class="preview-section">
            <div class="preview-section-title">
              {{ t('settings.backup.resources') }} ({{ previewResult.resources.length }})
            </div>
            <div v-for="res in previewResult.resources" :key="res.id" class="preview-item">
              <span :class="res.exists ? 'tag-exists' : 'tag-new'">
                {{ res.exists ? t('settings.backup.exists') : t('settings.backup.new') }}
              </span>
              {{ res.name }}
              <span v-if="res.extra" class="tag-protocol">{{ res.extra }}</span>
            </div>
          </div>

          <div class="preview-section">
            <div class="preview-section-title">
              {{ t('settings.backup.settings') }} ({{ previewResult.settings_count }})
            </div>
          </div>
        </div>
        <div class="preview-footer">
          <button class="btn" @click="previewResult = null">{{ t('settings.backup.cancel') }}</button>
          <button class="btn btn-primary" @click="confirmImport">{{ t('settings.backup.confirmImport') }}</button>
        </div>
      </div>
    </div>

    <!-- 导入结果 -->
    <div v-if="importResult" class="import-result">
      <h4>{{ t('settings.backup.importResult') }}</h4>
      <div class="result-stats">
        <div>{{ t('settings.backup.environments') }}：{{ t('settings.backup.created') }} {{ importResult.environments.created }}，{{ t('settings.backup.skipped') }} {{ importResult.environments.skipped }}，{{ t('settings.backup.updated') }} {{ importResult.environments.updated }}</div>
        <div>{{ t('settings.backup.resources') }}：{{ t('settings.backup.created') }} {{ importResult.resources.created }}，{{ t('settings.backup.skipped') }} {{ importResult.resources.skipped }}，{{ t('settings.backup.updated') }} {{ importResult.resources.updated }}</div>
        <div>{{ t('settings.backup.settings') }}：{{ t('settings.backup.created') }} {{ importResult.settings.created }}，{{ t('settings.backup.skipped') }} {{ importResult.settings.skipped }}，{{ t('settings.backup.updated') }} {{ importResult.settings.updated }}</div>
      </div>
      <div v-if="importResult.warnings.length" class="warnings">
        <div v-for="(w, i) in importResult.warnings" :key="i" class="warning-item">⚠ {{ w }}</div>
      </div>
    </div>

    <!-- 错误提示 -->
    <div v-if="error" class="backup-error">✗ {{ error }}</div>
  </SettingsSection>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import SettingsSection from './SettingsSection.vue'
import { exportBackup, previewBackup, importBackup } from '@/api/backup'
import type { PreviewResult, ImportResult } from '@/api/backup'
import { listEnvironments } from '@/api/env'
import type { Environment } from '@/api/env'

const { t } = useI18n()

// 环境列表
const environments = ref<Environment[]>([])

// ── 导出 ──────────────────────────────────────────────────
const exportScope = ref<'all' | 'selected'>('all')
const selectedEnvIds = ref<string[]>([])
const encryptBackup = ref(false)
const backupPassword = ref('')
const backupPasswordConfirm = ref('')
const exporting = ref(false)

const passwordMismatch = computed(() => {
  return encryptBackup.value && backupPassword.value && backupPasswordConfirm.value &&
    backupPassword.value !== backupPasswordConfirm.value
})

async function handleExport() {
  if (encryptBackup.value && passwordMismatch.value) return

  exporting.value = true
  try {
    const options: { envIds?: string[]; password?: string } = {}
    if (exportScope.value === 'selected' && selectedEnvIds.value.length) {
      options.envIds = selectedEnvIds.value
    }
    if (encryptBackup.value && backupPassword.value) {
      options.password = backupPassword.value
    }

    const blob = await exportBackup(options)

    // 触发下载
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
    a.href = url
    a.download = `rex-backup-${ts}.json`
    a.click()
    URL.revokeObjectURL(url)
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    exporting.value = false
  }
}

// ── 导入 ──────────────────────────────────────────────────
const fileInput = ref<HTMLInputElement | null>(null)
const importFile = ref<File | null>(null)
const dragover = ref(false)
const importPassword = ref('')
const importStrategy = ref<'skip_existing' | 'overwrite'>('skip_existing')
const importing = ref(false)
const previewing = ref(false)
const previewResult = ref<PreviewResult | null>(null)
const importResult = ref<ImportResult | null>(null)
const error = ref('')

// 加密文件检测：读取文件前几个字节判断是否为加密格式
// 简化实现：加密备份文件的 JSON 中 encrypted=true
const importFileEncrypted = ref(false)

function triggerUpload() {
  fileInput.value?.click()
}

function handleFileSelect(e: Event) {
  const input = e.target as HTMLInputElement
  if (input.files?.length) {
    setFile(input.files[0])
  }
}

function handleDrop(e: DragEvent) {
  dragover.value = false
  const files = e.dataTransfer?.files
  if (files?.length) {
    setFile(files[0])
  }
}

async function setFile(file: File) {
  importFile.value = file
  importResult.value = null
  previewResult.value = null
  error.value = ''

  // 快速读取检查是否加密
  try {
    const text = await file.text()
    const json = JSON.parse(text)
    importFileEncrypted.value = !!json.encrypted
  } catch {
    importFileEncrypted.value = false
  }
}

function clearFile() {
  importFile.value = null
  importFileEncrypted.value = false
  importPassword.value = ''
  previewResult.value = null
  error.value = ''
  if (fileInput.value) fileInput.value.value = ''
}

async function handlePreview() {
  if (!importFile.value) return
  previewing.value = true
  error.value = ''
  try {
    const password = importFileEncrypted.value ? importPassword.value : undefined
    previewResult.value = await previewBackup(importFile.value, password)
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    previewing.value = false
  }
}

async function handleImport() {
  if (!importFile.value) return
  importing.value = true
  error.value = ''
  try {
    const password = importFileEncrypted.value ? importPassword.value : undefined
    importResult.value = await importBackup(importFile.value, {
      password,
      strategy: importStrategy.value,
    })
    previewResult.value = null
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    importing.value = false
  }
}

async function confirmImport() {
  previewResult.value = null
  await handleImport()
}

// ── 辅助函数 ──────────────────────────────────────────────

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

// ── 加载环境列表 ──────────────────────────────────────────

onMounted(async () => {
  try {
    const result = await listEnvironments()
    environments.value = result
  } catch {
    // ignore
  }
})
</script>

<style scoped>
.backup-export,
.backup-import {
  padding: 0 0 var(--sp-md);
}

.backup-divider {
  border-top: 1px solid var(--border);
  margin: var(--sp-md) 0;
}

.backup-option {
  margin-bottom: var(--sp-md);
}

.backup-option-label {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  margin-bottom: var(--sp-xs);
  display: block;
}

.backup-radio-group {
  display: flex;
  gap: var(--sp-lg);
  margin-top: var(--sp-xs);
}

.radio-label,
.checkbox-label {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  font-size: var(--fs-sm);
  color: var(--text-primary);
  cursor: pointer;
}

.env-checkboxes {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-md);
  margin-top: var(--sp-sm);
  padding-left: var(--sp-lg);
}

.password-inputs {
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
  margin-top: var(--sp-sm);
}

.form-input {
  padding: var(--sp-xs) var(--sp-sm);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-deep);
  color: var(--text-primary);
  font-size: var(--fs-sm);
  outline: none;
  max-width: 280px;
}

.form-input:focus {
  border-color: var(--accent);
}

.field-error {
  font-size: var(--fs-xs);
  color: var(--error, #dc3545);
  margin-top: 2px;
}

/* Upload area */
.upload-area {
  border: 2px dashed var(--border);
  border-radius: var(--radius-md);
  padding: var(--sp-xl);
  text-align: center;
  cursor: pointer;
  transition: border-color var(--transition-fast), background var(--transition-fast);
  margin-bottom: var(--sp-md);
}

.upload-area:hover,
.upload-area.dragover {
  border-color: var(--accent);
  background: rgba(232, 145, 45, 0.05);
}

.upload-placeholder {
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.file-info {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-sm);
  font-size: var(--fs-sm);
  color: var(--text-primary);
}

.clear-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--fs-md);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
}

.clear-btn:hover {
  color: var(--error, #dc3545);
  background: rgba(220, 53, 69, 0.1);
}

.backup-actions {
  display: flex;
  gap: var(--sp-sm);
}

/* Preview modal */
.preview-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.preview-modal {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  width: 90%;
  max-width: 520px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
}

.preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-lg) var(--sp-xl);
  border-bottom: 1px solid var(--border);
}

.preview-header h4 {
  margin: 0;
  font-size: var(--fs-md);
}

.close-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--fs-lg);
}

.preview-body {
  padding: var(--sp-lg) var(--sp-xl);
  overflow-y: auto;
  flex: 1;
}

.preview-meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-md);
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  margin-bottom: var(--sp-lg);
}

.preview-section {
  margin-bottom: var(--sp-md);
}

.preview-section-title {
  font-weight: 600;
  font-size: var(--fs-sm);
  margin-bottom: var(--sp-sm);
}

.preview-item {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-xs) 0;
  font-size: var(--fs-sm);
}

.tag-new {
  background: var(--success-bg, rgba(40, 167, 69, 0.1));
  color: var(--success, #28a745);
  padding: 1px 6px;
  border-radius: var(--radius-sm);
  font-size: var(--fs-xs);
}

.tag-exists {
  background: var(--warning-bg, rgba(255, 193, 7, 0.1));
  color: var(--warning, #ffc107);
  padding: 1px 6px;
  border-radius: var(--radius-sm);
  font-size: var(--fs-xs);
}

.tag-protocol {
  color: var(--text-muted);
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
}

.preview-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
  padding: var(--sp-lg) var(--sp-xl);
  border-top: 1px solid var(--border);
}

/* Import result */
.import-result {
  margin-top: var(--sp-md);
  padding: var(--sp-md);
  background: var(--bg-deep);
  border-radius: var(--radius-sm);
}

.import-result h4 {
  margin: 0 0 var(--sp-sm);
  font-size: var(--fs-sm);
  font-weight: 600;
}

.result-stats {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.warnings {
  margin-top: var(--sp-sm);
}

.warning-item {
  font-size: var(--fs-xs);
  color: var(--warning, #ffc107);
  padding: 2px 0;
}

.backup-error {
  margin-top: var(--sp-md);
  padding: var(--sp-sm) var(--sp-md);
  background: rgba(220, 53, 69, 0.1);
  border: 1px solid #dc3545;
  border-radius: var(--radius-sm);
  color: #dc3545;
  font-size: var(--fs-sm);
}
</style>
