<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-overlay" @click="$emit('close')">
      <div class="confirm-panel" @click.stop>
        <div class="confirm-title">{{ t('ctx.resetTokenTitle') }}</div>
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
          <button class="btn btn-ghost" @click="$emit('close')">{{ t('common.cancel') }}</button>
          <button class="btn btn-danger" :disabled="!confirmed" @click="handleReset">
            {{ t('ctx.confirmReset') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Agent } from '@/api/agent'

defineProps<{ agent: Agent | null; visible: boolean }>()
defineEmits<{ close: [] }>()

const { t } = useI18n()
const confirmed = ref(false)

function handleReset() {
  // TODO: call API to reset token
  confirmed.value = false
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
  animation: modalIn 0.2s ease;
}

@keyframes modalIn {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
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
