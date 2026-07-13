<template>
  <Teleport to="body">
    <Transition name="dialog">
      <div v-if="visible" class="batch-dialog-overlay" @click.self="handleClose">
        <div class="batch-dialog import-dialog" @click.stop>
          <div class="batch-dialog-header">
            <span class="batch-dialog-title">{{ t('redis.import.title') }}</span>
            <button class="batch-dialog-close" @click="handleClose">×</button>
          </div>
          <div class="batch-dialog-body">
            <!-- Step: Upload -->
            <template v-if="step === 'upload'">
              <div
                class="import-dropzone"
                :class="{ 'import-dropzone-active': isDragging }"
                @dragover.prevent="isDragging = true"
                @dragleave="isDragging = false"
                @drop.prevent="handleDrop"
                @click="fileInputRef?.click()"
              >
                <input
                  ref="fileInputRef"
                  type="file"
                  accept=".json"
                  class="import-file-input"
                  @change="handleFileSelect"
                />
                <div class="import-dropzone-icon">📄</div>
                <div class="import-dropzone-text">{{ t('redis.import.dropHere') }}</div>
                <div class="import-dropzone-hint">{{ t('redis.import.clickToSelect') }}</div>
              </div>
              <p v-if="parseError" class="import-error-text">{{ parseError }}</p>
            </template>

            <!-- Step: Preview -->
            <template v-if="step === 'preview'">
              <p class="batch-dialog-message">
                {{ t('redis.import.detected', { format: detectedFormat, count: entries.length }) }}
              </p>
              <div class="import-preview-table">
                <div class="import-preview-header">
                  <span class="import-col-key">{{ t('redis.import.key') }}</span>
                  <span class="import-col-type">{{ t('redis.import.type') }}</span>
                  <span class="import-col-value">{{ t('redis.import.valuePreview') }}</span>
                </div>
                <div v-for="(entry, i) in previewEntries" :key="i" class="import-preview-row">
                  <span class="import-col-key import-key">{{ entry.key }}</span>
                  <span class="import-col-type import-type">{{ t(`redis.type.${entry.type}`) }}</span>
                  <span class="import-col-value import-value">{{ entry.valuePreview }}</span>
                </div>
                <div v-if="entries.length > maxPreview" class="import-preview-more">
                  {{ t('redis.import.andMore', { count: entries.length - maxPreview }) }}
                </div>
              </div>
            </template>

            <!-- Step: Importing -->
            <template v-if="step === 'importing'">
              <div class="import-progress">
                <div class="import-progress-bar">
                  <div
                    class="import-progress-fill"
                    :style="{ width: progressPercent + '%' }"
                  />
                </div>
                <div class="import-progress-text">
                  {{ t('redis.import.progress', { current: progressCurrent, total: progressTotal }) }}
                </div>
              </div>
            </template>

            <!-- Step: Result -->
            <template v-if="step === 'result'">
              <div class="import-result">
                <div class="import-result-summary">
                  <span class="import-result-success">
                    ✓ {{ t('redis.import.successCount', { count: successCount }) }}
                  </span>
                  <span v-if="errorCount > 0" class="import-result-error">
                    ✕ {{ t('redis.import.errorCount', { count: errorCount }) }}
                  </span>
                </div>
                <div v-if="errors.length > 0" class="import-result-errors">
                  <div v-for="(err, i) in visibleErrors" :key="i" class="import-result-error-item">
                    <span class="import-error-key">{{ err.key }}</span>: {{ err.message }}
                  </div>
                  <div v-if="errors.length > maxErrors" class="import-preview-more">
                    {{ t('redis.import.andMoreErrors', { count: errors.length - maxErrors }) }}
                  </div>
                </div>
              </div>
            </template>
          </div>
          <div class="batch-dialog-footer">
            <button class="batch-dialog-btn batch-dialog-btn-cancel" @click="handleClose">
              {{ step === 'result' ? t('common.close') : t('redis.keys.batch.cancel') }}
            </button>
            <button
              v-if="step === 'upload'"
              class="batch-dialog-btn batch-dialog-btn-primary"
              :disabled="entries.length === 0"
              @click="step = 'preview'"
            >
              {{ t('redis.import.preview') }}
            </button>
            <button
              v-if="step === 'preview'"
              class="batch-dialog-btn batch-dialog-btn-primary"
              @click="startImport"
            >
              {{ t('redis.import.startImport', { count: entries.length }) }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRedisSession } from './useRedisSession'

const { t } = useI18n()

const props = defineProps<{
  visible: boolean
  connectionId: string
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  importComplete: [count: number]
}>()

// ── Types ───────────────────────────────────────────────
type Step = 'upload' | 'preview' | 'importing' | 'result'

interface ImportEntry {
  key: string
  type: 'string' | 'hash' | 'list' | 'set' | 'zset'
  value: string | Record<string, string> | string[] | [string, string][]
}

interface ImportError {
  key: string
  message: string
}

// ── State ───────────────────────────────────────────────
const step = ref<Step>('upload')
const isDragging = ref(false)
const fileInputRef = ref<HTMLInputElement | null>(null)
const parseError = ref('')
const entries = ref<ImportEntry[]>([])
const detectedFormat = ref('plain')

const progressCurrent = ref(0)
const progressTotal = ref(0)
const progressPercent = computed(() =>
  progressTotal.value > 0 ? Math.round((progressCurrent.value / progressTotal.value) * 100) : 0,
)

const successCount = ref(0)
const errorCount = ref(0)
const errors = ref<ImportError[]>([])
const maxErrors = 20
const maxPreview = 10

const previewEntries = computed(() =>
  entries.value.slice(0, maxPreview).map((e) => ({
    key: e.key,
    type: e.type,
    valuePreview: formatValuePreview(e),
  })),
)

const visibleErrors = computed(() => errors.value.slice(0, maxErrors))

// ── Session ─────────────────────────────────────────────
const session = useRedisSession(() => props.connectionId)

// ── Reset on open ───────────────────────────────────────
watch(() => props.visible, (val) => {
  if (val) {
    step.value = 'upload'
    entries.value = []
    parseError.value = ''
    progressCurrent.value = 0
    progressTotal.value = 0
    successCount.value = 0
    errorCount.value = 0
    errors.value = []
    isDragging.value = false
  }
})

// ── Close handler ───────────────────────────────────────
function handleClose() {
  if (step.value === 'importing') return
  emit('update:visible', false)
}

// ── File handling ───────────────────────────────────────
function handleDrop(e: DragEvent) {
  isDragging.value = false
  const file = e.dataTransfer?.files[0]
  if (file) processFile(file)
}

function handleFileSelect(e: Event) {
  const target = e.target as HTMLInputElement
  const file = target.files?.[0]
  if (file) processFile(file)
  target.value = ''
}

function processFile(file: File) {
  parseError.value = ''
  if (!file.name.endsWith('.json')) {
    parseError.value = t('redis.import.errorNotJson')
    return
  }

  const reader = new FileReader()
  reader.onload = () => {
    try {
      const data = JSON.parse(reader.result as string) as Record<string, unknown>
      parseData(data)
    } catch {
      parseError.value = t('redis.import.errorInvalidJson')
    }
  }
  reader.readAsText(file)
}

// ── Data parsing ────────────────────────────────────────
function parseData(data: Record<string, unknown>) {
  const result: ImportEntry[] = []
  let isTyped = false

  for (const value of Object.values(data)) {
    if (isTypedEntry(value)) {
      isTyped = true
      break
    }
  }

  detectedFormat.value = isTyped ? 'typed' : 'plain'

  for (const [key, value] of Object.entries(data)) {
    if (isTyped) {
      if (isTypedEntry(value)) {
        const typed = value as { type: string; value: unknown }
        result.push({
          key,
          type: normalizeType(typed.type),
          value: typed.value as ImportEntry['value'],
        })
      } else {
        result.push({
          key,
          type: 'string',
          value: typeof value === 'string' ? value : JSON.stringify(value),
        })
      }
    } else {
      if (typeof value === 'string') {
        result.push({ key, type: 'string', value })
      } else if (Array.isArray(value)) {
        result.push({ key, type: 'list', value: value.map(String) })
      } else if (typeof value === 'object' && value !== null) {
        result.push({ key, type: 'string', value: JSON.stringify(value) })
      } else {
        result.push({ key, type: 'string', value: String(value ?? '') })
      }
    }
  }

  entries.value = result
}

function isTypedEntry(value: unknown): boolean {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return false
  const obj = value as Record<string, unknown>
  return 'type' in obj && 'value' in obj && typeof obj.type === 'string'
}

function normalizeType(type: string): ImportEntry['type'] {
  const t = type.toLowerCase()
  if (t === 'hash' || t === 'list' || t === 'set' || t === 'zset') return t
  return 'string'
}

function formatValuePreview(entry: ImportEntry): string {
  let preview: string
  if (typeof entry.value === 'string') {
    preview = entry.value
  } else if (Array.isArray(entry.value)) {
    preview = JSON.stringify(entry.value)
  } else {
    preview = JSON.stringify(entry.value)
  }
  return preview.length > 50 ? preview.slice(0, 47) + '...' : preview
}

// ── Redis command building ──────────────────────────────
function escapeRedisArg(arg: string): string {
  return '"' + arg.replace(/\\/g, '\\\\').replace(/"/g, '\\"') + '"'
}

function buildCommand(entry: ImportEntry): string {
  switch (entry.type) {
    case 'string':
      return `SET ${escapeRedisArg(entry.key)} ${escapeRedisArg(entry.value as string)}`

    case 'hash': {
      const fields = entry.value as Record<string, string>
      const args = Object.entries(fields)
        .flatMap(([f, v]) => [escapeRedisArg(f), escapeRedisArg(v)])
        .join(' ')
      return `HSET ${escapeRedisArg(entry.key)} ${args}`
    }

    case 'list': {
      const items = entry.value as string[]
      const args = items.map((i) => escapeRedisArg(String(i))).join(' ')
      return `RPUSH ${escapeRedisArg(entry.key)} ${args}`
    }

    case 'set': {
      const members = entry.value as string[]
      const args = members.map((m) => escapeRedisArg(String(m))).join(' ')
      return `SADD ${escapeRedisArg(entry.key)} ${args}`
    }

    case 'zset': {
      const pairs = entry.value as [string, string][]
      const args = pairs
        .map(([member, score]) => `${escapeRedisArg(String(score))} ${escapeRedisArg(String(member))}`)
        .join(' ')
      return `ZADD ${escapeRedisArg(entry.key)} ${args}`
    }
    default:
      // 类型均已穷尽，保持默认分支用于边界保护
      return `SET ${escapeRedisArg(entry.key)} ${escapeRedisArg(String(entry.value))}`
  }
}

// ── Import execution ────────────────────────────────────
function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

async function startImport() {
  step.value = 'importing'
  progressTotal.value = entries.value.length
  progressCurrent.value = 0
  successCount.value = 0
  errorCount.value = 0
  errors.value = []

  try {
    if (!session.connected.value) {
      await session.connect()
    }
  } catch (e) {
    errors.value = [{ key: '*', message: String(e) }]
    errorCount.value = 1
    step.value = 'result'
    return
  }

  for (let i = 0; i < entries.value.length; i++) {
    const entry = entries.value[i]!
    try {
      const cmd = buildCommand(entry)
      await session.execute(cmd)
      successCount.value++
    } catch (e) {
      errorCount.value++
      errors.value.push({ key: entry.key, message: String(e) })
    }
    progressCurrent.value = i + 1
    await delay(50)
  }

  step.value = 'result'
  emit('importComplete', successCount.value)
}
</script>

<style scoped>
.import-dialog {
  width: 560px;
}

.import-file-input {
  display: none;
}

.import-dropzone {
  border: 2px dashed var(--border);
  border-radius: 8px;
  padding: 32px 16px;
  text-align: center;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}

.import-dropzone:hover,
.import-dropzone-active {
  border-color: var(--accent);
  background: var(--bg-hover);
}

.import-dropzone-icon {
  font-size: 32px;
  margin-bottom: 8px;
}

.import-dropzone-text {
  font-size: 13px;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.import-dropzone-hint {
  font-size: 11px;
  color: var(--text-muted);
}

.import-error-text {
  color: #f85149;
  font-size: 12px;
  margin: 8px 0 0 0;
}

.import-preview-table {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: hidden;
}

.import-preview-header {
  display: flex;
  gap: 8px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border);
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
}

.import-preview-row {
  display: flex;
  gap: 8px;
  padding: 4px 10px;
  font-size: 12px;
  font-family: var(--font-mono);
}

.import-preview-row:not(:last-child) {
  border-bottom: 1px solid var(--border);
}

.import-col-key {
  flex: 2;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.import-col-type {
  flex: 1;
  color: var(--accent);
  font-weight: 500;
}

.import-col-value {
  flex: 3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-secondary);
}

.import-key {
  color: var(--text-primary);
}

.import-type {
  color: var(--accent);
}

.import-value {
  color: var(--text-secondary);
}

.import-preview-more {
  padding: 4px 10px;
  font-size: 11px;
  color: var(--text-muted);
}

.import-progress {
  padding: 16px 0;
}

.import-progress-bar {
  height: 6px;
  background: var(--bg-deep);
  border-radius: 3px;
  overflow: hidden;
}

.import-progress-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 3px;
  transition: width 0.1s linear;
}

.import-progress-text {
  text-align: center;
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 8px;
}

.import-result {
  padding: 4px 0;
}

.import-result-summary {
  display: flex;
  gap: 16px;
  margin-bottom: 12px;
  font-size: 13px;
  font-weight: 500;
}

.import-result-success {
  color: #3fb950;
}

.import-result-error {
  color: #f85149;
}

.import-result-errors {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 4px;
  max-height: 160px;
  overflow-y: auto;
  padding: 8px;
}

.import-result-error-item {
  font-size: 11px;
  color: var(--text-secondary);
  padding: 2px 0;
  font-family: var(--font-mono);
}

.import-error-key {
  color: #f85149;
  font-weight: 500;
}
</style>
