<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{ registered: boolean }>()
const emit = defineEmits<{
  dial: [destination: string]
}>()

const { t } = useI18n()
const number = ref('')

const keys = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '*', '0', '#']
const canDial = computed(() => props.registered && number.value.trim().length > 0)

function press(k: string) {
  number.value += k
}

function clear() {
  number.value = ''
}

function backspace() {
  number.value = number.value.slice(0, -1)
}

function call() {
  if (!canDial.value) return
  emit('dial', number.value.trim())
}
</script>

<template>
  <div class="dialpad">
    <div class="dialpad-display">
      <input
        v-model="number"
        class="form-input dial-number"
        :placeholder="t('sip.enterNumber')"
        inputmode="tel"
        @keyup.enter="call"
      />
      <button class="dialpad-icon-btn" :title="t('common.delete')" @click="backspace">⌫</button>
    </div>

    <div class="dialpad-grid">
      <button
        v-for="k in keys"
        :key="k"
        class="dialpad-key"
        @click="press(k)"
      >
        {{ k }}
      </button>
    </div>

    <div class="dialpad-actions">
      <button class="dialpad-btn-clear" @click="clear">{{ t('sip.clear') }}</button>
      <button
        class="dialpad-btn-call"
        :disabled="!canDial"
        @click="call"
      >
        ☎ {{ t('sip.call') }}
      </button>
    </div>

    <p v-if="!registered" class="dialpad-hint muted">{{ t('sip.notRegistered') }}</p>
  </div>
</template>

<style scoped>
.dialpad {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-4);
  max-width: 320px;
  margin: 0 auto;
}
.dialpad-display {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}
.dial-number {
  flex: 1;
  font-family: var(--font-mono);
  font-size: var(--text-lg);
  text-align: center;
  letter-spacing: 2px;
}
.dialpad-icon-btn {
  background: var(--bg-hover);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  border-radius: 6px;
  width: 36px;
  height: 36px;
  cursor: pointer;
}
.dialpad-icon-btn:hover {
  color: var(--text-primary);
}
.dialpad-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-2);
}
.dialpad-key {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  color: var(--text-primary);
  border-radius: 10px;
  font-size: var(--text-lg);
  height: 52px;
  cursor: pointer;
  transition: all var(--transition);
}
.dialpad-key:hover {
  border-color: var(--accent);
  background: var(--bg-hover);
}
.dialpad-actions {
  display: flex;
  gap: var(--space-2);
}
.dialpad-btn-clear {
  background: var(--bg-hover);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  border-radius: 8px;
  padding: 10px var(--space-3);
  cursor: pointer;
}
.dialpad-btn-call {
  flex: 1;
  background: var(--accent);
  border: none;
  color: var(--text-on-accent, #fff);
  border-radius: 8px;
  padding: 10px;
  font-weight: 600;
  cursor: pointer;
}
.dialpad-btn-call:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.dialpad-hint {
  text-align: center;
  font-size: var(--text-xs);
}
</style>
