<template>
  <Teleport to="body">
    <Transition name="modal">
    <div v-if="visible" class="modal-overlay" @click="$emit('close')">
      <div
        ref="dialogEl"
        class="confirm-panel"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        @click.stop
        @keydown.tab="trapFocus"
        @keydown.esc="$emit('close')"
      >
        <div class="confirm-title" :id="titleId">{{ t('ctx.resetTokenTitle') }}</div>
        <p class="confirm-desc">
          {{ t('ctx.resetTokenDesc1') }}
          <strong>{{ agent?.name }}</strong>
          {{ t('ctx.resetTokenDesc2') }}<br>
          {{ t('ctx.resetTokenWarning') }}
        </p>
        <div class="confirm-checkbox">
          <label>
            <input v-model="confirmed" type="checkbox" style="accent-color: var(--danger)">
            {{ t('ctx.resetTokenConfirm') }}
          </label>
        </div>
        <div class="confirm-actions">
          <button ref="cancelBtnEl" class="btn btn-ghost" @click="$emit('close')">{{ t('common.cancel') }}</button>
          <button class="btn btn-danger" :disabled="!confirmed || loading" @click="handleReset">
            {{ t('ctx.confirmReset') }}
          </button>
        </div>
      </div>
    </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useId } from '@/composables/useId'
import { useI18n } from 'vue-i18n'
import { resetAgentToken } from '@/api/agent'
import type { Agent } from '@/api/agent'

const props = defineProps<{ agent: Agent | null; visible: boolean }>()
const emit = defineEmits<{ close: []; success: [] }>()

const titleId = useId('agent-reset-token-title')
const dialogEl = ref<HTMLElement>()
const cancelBtnEl = ref<HTMLElement>()
let previousActive: HTMLElement | null = null

watch(() => props.visible, (v) => {
  if (v) {
    previousActive = document.activeElement as HTMLElement | null
    nextTick(() => cancelBtnEl.value?.focus())
  } else if (previousActive) {
    previousActive.focus()
    previousActive = null
  }
})

function trapFocus(e: KeyboardEvent) {
  const focusable = dialogEl.value?.querySelectorAll<HTMLElement>(
    'a, button, input, textarea, select, [tabindex]:not([tabindex="-1"])',
  )
  if (!focusable || focusable.length === 0) return
  const first = focusable[0]!
  const last = focusable[focusable.length - 1]!
  if (e.shiftKey && document.activeElement === first) {
    e.preventDefault()
    last.focus()
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault()
    first.focus()
  }
}
const { t } = useI18n()
const confirmed = ref(false)
const loading = ref(false)

async function handleReset() {
  if (!props.agent) return
  loading.value = true
  try {
    await resetAgentToken(props.agent.id)
    emit('success')
    emit('close')
  } catch (e) {
    alert(String(e))
  } finally {
    loading.value = false
    confirmed.value = false
  }
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 400;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0,0,0,0.5);
  backdrop-filter: blur(4px);
}

.confirm-panel {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  width: 440px;
  max-width: 90vw;
  padding: var(--sp-xl);
  box-shadow: var(--shadow-lg);
}

.confirm-title {
  font-family: var(--font-mono);
  font-size: var(--fs-md);
  font-weight: 600;
  margin-bottom: var(--sp-lg);
}

.confirm-desc {
  color: var(--text-secondary);
  font-size: var(--fs-sm);
  line-height: 1.6;
  margin-bottom: var(--sp-lg);
}

.confirm-checkbox {
  padding: var(--sp-md);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  margin-bottom: var(--sp-lg);
}

.confirm-checkbox label {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  cursor: pointer;
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
}

.btn {
  padding: var(--sp-sm) var(--sp-lg);
  border-radius: var(--radius-md);
  font-size: var(--fs-sm);
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
  border: 1px solid var(--border);
  background: var(--bg-deep);
  color: var(--text-primary);
}

.btn:hover {
  background: var(--bg-hover);
}

.btn-ghost {
  background: none;
  border-color: transparent;
}

.btn-danger {
  background: var(--danger);
  border-color: var(--danger);
  color: #fff;
}

.btn-danger:hover {
  opacity: 0.9;
}

.btn-danger:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
