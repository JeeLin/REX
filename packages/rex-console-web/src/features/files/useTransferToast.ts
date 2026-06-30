import { watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useToast } from '@/composables/useToast'
import type { Ref } from 'vue'
import type { TransferTask } from '@/api/transfer'

export function useTransferToast(
  transferTasks: Ref<TransferTask[]>,
  prevTasks: Ref<TransferTask[]>,
) {
  const { t } = useI18n()
  const { success, error: toastError } = useToast()

  watch(transferTasks, (newTasks) => {
    for (const task of newTasks) {
      const prev = prevTasks.value.find(t => t.id === task.id)
      if (!prev) continue

      if (prev.status !== 'completed' && task.status === 'completed') {
        const filename = task.target.path.split('/').pop() || task.target.path
        success(t('files.transfer.completedToast', { file: filename }))
      } else if (prev.status !== 'failed' && task.status === 'failed') {
        const filename = task.target.path.split('/').pop() || task.target.path
        toastError(t('files.transfer.failedToast', { file: filename, error: task.status_detail || '' }))
      }
    }
  })
}
