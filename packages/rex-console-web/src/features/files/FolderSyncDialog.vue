<script setup lang="ts">
//! 文件夹同步对话框 —— v0.92.0 子任务 4。
//!
//! 对标 Xftp 同步对话框（`docs/PRODUCT.md` §3.8 五要素）：方向 / 比较依据 /
//! 包含·排除掩码 / 删除孤儿 / 预览。浏览器只提交选项并查看 dry-run 计划——
//! diff 与搬运全部在 Hub 侧完成（文件数据不经过浏览器）。
//!
//! `options` 是唯一选项状态：预览（`POST /api/files/sync/preview`）与执行
//! （`POST /api/files/sync`）复用同一份 `requestBody`，因此两者判定必然一致。
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import Button from '@/components/ui/Button.vue'
import SyncPlanPreview from './SyncPlanPreview.vue'
import { createSync, previewSync } from '@/api/files'
import type {
  SyncCompareBasis,
  SyncDirection,
  SyncOptions,
  SyncPlan,
  SyncRequestBody,
  TransferEndpoint,
} from '@/api/files'

const props = defineProps<{
  open: boolean
  source: TransferEndpoint | null
  target: TransferEndpoint | null
}>()

const emit = defineEmits<{
  close: []
  created: [taskId: string]
  error: [message: string]
}>()

const { t } = useI18n()

// --- 选项状态（预览与执行共用这一份） ---
const direction = ref<SyncDirection>('upload')
const compare = ref<SyncCompareBasis>('modified_time')
const includeText = ref('')
const excludeText = ref('')
const deleteOrphans = ref(false)

/** 多行 glob → 掩码数组：逐行 trim 并丢弃空行（空掩码永不命中，保留只会误导）。 */
function parseMasks(text: string): string[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
}

const options = computed<SyncOptions>(() => ({
  direction: direction.value,
  compare: compare.value,
  include: parseMasks(includeText.value),
  exclude: parseMasks(excludeText.value),
  delete_orphans: deleteOrphans.value,
}))

/** 预览与创建共用的请求体：两端必须看到完全一致的选项。 */
const requestBody = computed<SyncRequestBody | null>(() =>
  props.source && props.target
    ? { source: props.source, target: props.target, options: options.value }
    : null,
)

/** 双向同步下引擎两侧都保留（`diff` 不产出删除），开关置灰并清零。 */
const orphansAllowed = computed(() => direction.value !== 'bidirectional')

const orphanSideHint = computed(() => {
  if (direction.value === 'bidirectional') return t('files.syncOrphanSideNone')
  return direction.value === 'download'
    ? t('files.syncOrphanSideDownload')
    : t('files.syncOrphanSideUpload')
})

const plan = ref<SyncPlan | null>(null)
const previewing = ref(false)
const starting = ref(false)
const previewError = ref('')
const createError = ref('')
const hasPreviewed = ref(false)
const stale = ref(false)

const ready = computed(() => !!requestBody.value)
const busy = computed(() => previewing.value || starting.value)

watch(direction, (d) => {
  if (d === 'bidirectional') deleteOrphans.value = false
})

/** 选项改动即作废旧预览：旧计划不再代表当前配置，不能继续当作依据展示。 */
watch(options, () => {
  if (!hasPreviewed.value) return
  plan.value = null
  stale.value = true
})

watch(
  () => props.open,
  (open) => {
    if (!open) return
    direction.value = 'upload'
    compare.value = 'modified_time'
    includeText.value = ''
    excludeText.value = ''
    deleteOrphans.value = false
    plan.value = null
    previewError.value = ''
    createError.value = ''
    hasPreviewed.value = false
    stale.value = false
    previewing.value = false
    starting.value = false
  },
)

function errText(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

function close() {
  if (busy.value) return
  emit('close')
}

async function runPreview() {
  const body = requestBody.value
  if (!body || busy.value) return
  previewing.value = true
  previewError.value = ''
  createError.value = ''
  hasPreviewed.value = true
  stale.value = false
  try {
    plan.value = await previewSync(body)
  } catch (e) {
    plan.value = null
    previewError.value = errText(e)
    emit('error', `${t('files.syncPreviewFailed')}: ${errText(e)}`)
  } finally {
    previewing.value = false
  }
}

async function startSync() {
  const body = requestBody.value
  if (!body || busy.value) return
  starting.value = true
  createError.value = ''
  previewError.value = ''
  try {
    const created = await createSync(body)
    emit('created', created.id)
    emit('close')
  } catch (e) {
    createError.value = errText(e)
    emit('error', `${t('files.syncFailed')}: ${errText(e)}`)
  } finally {
    starting.value = false
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fsd-overlay" @click.self="close">
      <div class="fsd-dialog" role="dialog" :aria-label="t('files.folderSync')">
        <div class="fsd-head">
          <span class="fsd-title">{{ t('files.folderSync') }}</span>
        </div>

        <div class="fsd-endpoints">
          <div class="fsd-endpoint">
            <span class="fsd-endpoint-label">{{ t('files.source') }}</span>
            <span class="fsd-endpoint-path mono" :title="props.source?.path">{{ props.source?.path || '-' }}</span>
          </div>
          <span class="fsd-arrow">→</span>
          <div class="fsd-endpoint">
            <span class="fsd-endpoint-label">{{ t('files.target') }}</span>
            <span class="fsd-endpoint-path mono" :title="props.target?.path">{{ props.target?.path || '-' }}</span>
          </div>
        </div>

        <fieldset class="fsd-group">
          <legend class="fsd-label">{{ t('files.direction') }}</legend>
          <div class="fsd-radios">
            <label class="fsd-radio">
              <input v-model="direction" type="radio" value="upload" />
              <span>{{ t('files.upload') }}</span>
            </label>
            <label class="fsd-radio">
              <input v-model="direction" type="radio" value="download" />
              <span>{{ t('files.download') }}</span>
            </label>
            <label class="fsd-radio">
              <input v-model="direction" type="radio" value="bidirectional" />
              <span>{{ t('files.bidirectional') }}</span>
            </label>
          </div>
          <p class="fsd-hint">{{ t('files.syncDirectionHint') }}</p>
        </fieldset>

        <fieldset class="fsd-group">
          <legend class="fsd-label">{{ t('files.compareBy') }}</legend>
          <div class="fsd-radios">
            <label class="fsd-radio">
              <input v-model="compare" type="radio" value="size" />
              <span>{{ t('files.bySize') }}</span>
            </label>
            <label class="fsd-radio">
              <input v-model="compare" type="radio" value="modified_time" />
              <span>{{ t('files.byTime') }}</span>
            </label>
          </div>
        </fieldset>

        <div class="fsd-group">
          <label class="fsd-label" for="fsd-include">{{ t('files.include') }}</label>
          <textarea
            id="fsd-include"
            v-model="includeText"
            class="fsd-textarea mono"
            rows="2"
            spellcheck="false"
          />
        </div>

        <div class="fsd-group">
          <label class="fsd-label" for="fsd-exclude">{{ t('files.exclude') }}</label>
          <textarea
            id="fsd-exclude"
            v-model="excludeText"
            class="fsd-textarea mono"
            rows="2"
            spellcheck="false"
          />
        </div>
        <p class="fsd-hint">{{ t('files.syncMaskHint') }}</p>

        <label class="fsd-checkbox">
          <input v-model="deleteOrphans" type="checkbox" :disabled="!orphansAllowed" />
          <span>{{ t('files.deleteOrphans') }}</span>
        </label>
        <p class="fsd-hint">{{ orphanSideHint }}</p>

        <div v-if="hasPreviewed" class="fsd-plan">
          <SyncPlanPreview :plan="plan" :loading="previewing" :error="previewError" />
        </div>
        <p v-if="stale" class="fsd-hint fsd-hint--stale">{{ t('files.syncPlanStale') }}</p>
        <div v-if="createError" class="fsd-error">{{ createError }}</div>

        <div class="fsd-actions">
          <Button variant="ghost" :disabled="busy" @click="close">{{ t('files.cancel') }}</Button>
          <Button variant="secondary" :disabled="!ready || busy" :loading="previewing" @click="runPreview">
            {{ t('files.preview') }}
          </Button>
          <Button variant="primary" :disabled="!ready || busy" :loading="starting" @click="startSync">
            {{ t('files.startSync') }}
          </Button>
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
  width: 92vw;
  max-width: 520px;
  max-height: 88vh;
  overflow-y: auto;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.fsd-head { display: flex; align-items: baseline; justify-content: space-between; gap: var(--space-2); }
.fsd-title { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); }

.fsd-endpoints {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}
.fsd-endpoint { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.fsd-endpoint-label { font-size: var(--text-xs); color: var(--text-muted); }
.fsd-endpoint-path {
  font-size: var(--text-xs);
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.fsd-arrow { color: var(--text-muted); flex-shrink: 0; }

.fsd-group { border: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: var(--space-1); }
.fsd-label { font-size: var(--text-xs); color: var(--text-muted); text-transform: uppercase; }
.fsd-radios { display: flex; flex-wrap: wrap; gap: var(--space-3); }
.fsd-radio, .fsd-checkbox { display: flex; align-items: center; gap: var(--space-1); font-size: var(--text-sm); color: var(--text-primary); cursor: pointer; }
.fsd-radio input, .fsd-checkbox input { margin: 0; accent-color: var(--accent); }
.fsd-radio input:disabled, .fsd-checkbox input:disabled { cursor: not-allowed; }

.fsd-textarea {
  width: 100%;
  padding: var(--space-2);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-xs);
  resize: vertical;
  outline: none;
}
.fsd-textarea:focus { border-color: var(--accent); }

.fsd-hint { margin: 0; font-size: var(--text-xs); color: var(--text-muted); }
.fsd-hint--stale { color: var(--warning); }
.fsd-error {
  padding: var(--space-2);
  background: var(--danger-soft);
  color: var(--danger);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
}

.fsd-plan {
  max-height: 240px;
  overflow: auto;
  padding: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
}

.fsd-actions { display: flex; justify-content: flex-end; gap: var(--space-2); }

@media (max-width: 768px) {
  .fsd-dialog { max-width: none; }
}
</style>