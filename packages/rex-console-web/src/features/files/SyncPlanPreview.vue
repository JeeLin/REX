<script setup lang="ts">
//! 同步预览（dry-run）只读渲染 —— v0.92.0 子任务 3。
//!
//! 组件只展示 Hub 返回的 `SyncPlan`：不创建任务、不启动引擎、不搬运字节
//! （文件数据不经过浏览器）。实际动作由对话框（子任务 4）确认后创建任务执行。
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { fmtSize } from '@/features/files/format'
import type { SyncAction, SyncPlan } from '@/api/files'

const props = withDefaults(
  defineProps<{
    /** Hub 计算出的计划；null = 尚未预览。 */
    plan: SyncPlan | null
    loading?: boolean
    /** 预览失败文案（ApiError 归一后的 message）。 */
    error?: string
  }>(),
  { loading: false, error: '' },
)

const { t } = useI18n()

/** 复制方向用 ↑/↓ 标注：`dir` 决定箭头（to_target = 源→目标）。 */
function actionLabel(a: SyncAction): string {
  if (a.action === 'delete') return t('files.delete')
  if (a.action === 'conflict') return t('files.conflict')
  return a.dir === 'to_target'
    ? t('files.syncActionCopyUp')
    : t('files.syncActionCopyDown')
}

function actionClass(a: SyncAction): string {
  if (a.action === 'delete') return 'sp-act sp-act--delete'
  if (a.action === 'conflict') return 'sp-act sp-act--conflict'
  return a.dir === 'to_target' ? 'sp-act sp-act--up' : 'sp-act sp-act--down'
}

/** Unix 秒 → 本地时间；不可解析（协议未返回/格式不符）显示占位符。 */
function fmtMtime(sec?: number | null): string {
  if (sec === null || sec === undefined) return t('files.syncUnknown')
  const d = new Date(sec * 1000)
  if (Number.isNaN(d.getTime())) return t('files.syncUnknown')
  return d.toLocaleString()
}

const empty = computed(() => !!props.plan && props.plan.actions.length === 0)
</script>

<template>
  <div class="sp">
    <div class="sp-head">
      <span class="sp-title">{{ t('files.syncPreview') }}</span>
      <template v-if="props.plan">
        <span class="sp-badge sp-badge--copy">{{ t('files.copy') }} {{ props.plan.summary.copies }}</span>
        <span class="sp-badge sp-badge--delete">{{ t('files.delete') }} {{ props.plan.summary.deletes }}</span>
        <span class="sp-badge sp-badge--conflict">{{ t('files.conflict') }} {{ props.plan.summary.conflicts }}</span>
        <span class="sp-total mono muted">{{ fmtSize(props.plan.summary.total_bytes) }}</span>
      </template>
    </div>

    <div v-if="props.loading" class="sp-state muted">{{ t('files.syncPreviewLoading') }}</div>
    <div v-else-if="props.error" class="sp-state sp-state--err">{{ props.error }}</div>
    <div v-else-if="empty" class="sp-state sp-state--ok">{{ t('files.syncUpToDate') }}</div>

    <table v-else-if="props.plan" class="sp-table">
      <thead>
        <tr>
          <th>{{ t('files.path') }}</th>
          <th>{{ t('files.action') }}</th>
          <th class="ta-r">{{ t('files.size') }}</th>
          <th>{{ t('files.syncSourceMtime') }}</th>
          <th>{{ t('files.syncTargetMtime') }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="a in props.plan.actions" :key="`${a.action}:${a.dir}:${a.rel_path}`">
          <td class="mono sp-path" :title="a.rel_path">{{ a.rel_path }}</td>
          <td><span :class="actionClass(a)">{{ actionLabel(a) }}</span></td>
          <td class="mono ta-r muted">{{ fmtSize(a.size) }}</td>
          <td class="mono muted">{{ fmtMtime(a.source_mtime) }}</td>
          <td class="mono muted">{{ fmtMtime(a.target_mtime) }}</td>
        </tr>
      </tbody>
    </table>

    <ul class="sp-notes muted">
      <li>{{ t('files.syncNoteFilesOnly') }}</li>
      <li>{{ t('files.syncNoteDeleteFiles') }}</li>
      <li>{{ t('files.syncNoteMtimeFallback') }}</li>
      <li>{{ t('files.syncNoteConflictNewer') }}</li>
    </ul>
  </div>
</template>

<style scoped>
.sp { display: flex; flex-direction: column; gap: var(--space-2); min-height: 0; }
.muted { color: var(--text-muted); }
.ta-r { text-align: right; }

.sp-head { display: flex; align-items: center; gap: var(--space-2); flex-wrap: wrap; }
.sp-title { font-size: var(--text-sm); font-weight: 600; color: var(--text-primary); }
.sp-badge {
  padding: 1px var(--space-2);
  border-radius: var(--badge-radius);
  font-size: var(--text-xs);
  border: 1px solid var(--border-strong);
  color: var(--text-secondary);
}
.sp-badge--copy { color: var(--success); border-color: var(--success); background: var(--success-soft); }
.sp-badge--delete { color: var(--danger); border-color: var(--danger); background: var(--danger-soft); }
.sp-badge--conflict { color: var(--warning); border-color: var(--warning); background: var(--warning-soft); }
.sp-total { margin-left: auto; font-size: var(--text-xs); }

.sp-state { padding: var(--space-3); font-size: var(--text-sm); text-align: center; }
.sp-state--ok { color: var(--success); background: var(--success-soft); border-radius: var(--radius); }
.sp-state--err { color: var(--danger); background: var(--danger-soft); border-radius: var(--radius); }

.sp-table { width: 100%; border-collapse: collapse; font-size: var(--text-xs); table-layout: fixed; }
.sp-table th {
  text-align: left; font-weight: 500; color: var(--text-muted);
  padding: var(--space-1) var(--space-2);
  border-bottom: 1px solid var(--border);
  position: sticky; top: 0; background: var(--bg-surface);
}
.sp-table td { padding: var(--space-1) var(--space-2); border-bottom: 1px solid var(--border-subtle); }
.sp-table tbody tr:hover { background: var(--table-row-hover); }
.sp-path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-primary); }

.sp-act { padding: 1px var(--space-1); border-radius: var(--radius-sm); white-space: nowrap; }
.sp-act--up { color: var(--accent); background: var(--accent-soft); }
.sp-act--down { color: var(--info); background: var(--info-soft); }
.sp-act--delete { color: var(--danger); background: var(--danger-soft); }
.sp-act--conflict { color: var(--warning); background: var(--warning-soft); }

.sp-notes {
  margin: 0; padding-left: var(--space-4);
  display: flex; flex-direction: column; gap: 2px;
  font-size: var(--text-xs);
}
</style>