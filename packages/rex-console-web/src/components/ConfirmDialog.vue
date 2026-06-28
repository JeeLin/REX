<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="visible" class="confirm-overlay" @click.self="$emit('cancel')">
        <div
          ref="dialogEl"
          class="confirm-dialog"
          role="alertdialog"
          aria-modal="true"
          :aria-labelledby="titleId"
          :aria-describedby="descId"
          @keydown.tab="trapFocus"
        >
          <h3 :id="titleId" class="confirm-title">{{ title }}</h3>
          <p :id="descId" class="confirm-message">{{ message }}</p>
          <div class="confirm-actions">
            <button ref="cancelBtnEl" class="btn btn-ghost btn-sm" @click="$emit('cancel')">
              {{ actualCancelLabel }}
            </button>
            <button
              ref="confirmBtnEl"
              class="btn btn-sm"
              :class="danger ? 'btn-danger' : 'btn-primary'"
              @click="$emit('confirm')"
            >
              {{ actualConfirmLabel }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import { useId } from '@/composables/useId'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const titleId = useId('confirm-title')
const descId = useId('confirm-desc')

const props = withDefaults(defineProps<{
  visible: boolean
  title: string
  message: string
  confirmLabel?: string
  cancelLabel?: string
  danger?: boolean
}>(), {
  confirmLabel: undefined,
  cancelLabel: undefined,
  danger: false,
})

defineEmits<{
  confirm: []
  cancel: []
}>()

const dialogEl = ref<HTMLElement>()
const cancelBtnEl = ref<HTMLElement>()
const confirmBtnEl = ref<HTMLElement>()
let previousActive: HTMLElement | null = null

const actualConfirmLabel = computed(() => props.confirmLabel || t('common.confirm'))
const actualCancelLabel = computed(() => props.cancelLabel || t('common.cancel'))

// Focus management: save trigger element, focus confirm button on open, restore on close
watch(() => props.visible, (val) => {
  if (val) {
    previousActive = document.activeElement as HTMLElement
    nextTick(() => confirmBtnEl.value?.focus())
  } else if (previousActive) {
    previousActive.focus()
    previousActive = null
  }
})

function trapFocus(e: KeyboardEvent) {
  const focusable = [cancelBtnEl.value, confirmBtnEl.value].filter(Boolean) as HTMLElement[]
  if (focusable.length === 0) return
  const first = focusable[0]
  const last = focusable[focusable.length - 1]
  if (e.shiftKey && document.activeElement === first) {
    e.preventDefault()
    last.focus()
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault()
    first.focus()
  }
}
</script>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: var(--z-modal, 1000);
}

.confirm-dialog {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  min-width: 320px;
  max-width: 420px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

.confirm-title {
  font-size: var(--fs-base);
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 var(--sp-sm);
}

.confirm-message {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  margin: 0 0 var(--sp-xl);
  line-height: 1.5;
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
}

.btn-danger {
  background: var(--danger);
  color: #fff;
  border: 1px solid var(--danger);
}

.btn-danger:hover {
  opacity: 0.9;
}

.modal-enter-active { transition: all 0.2s ease; }
.modal-leave-active { transition: all 0.15s ease; }
.modal-enter-from, .modal-leave-to { opacity: 0; }
.modal-enter-from .confirm-dialog { transform: scale(0.95); }
</style>
