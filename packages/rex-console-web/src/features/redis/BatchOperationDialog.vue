<template>
  <Teleport to="body">
    <Transition name="dialog">
      <div v-if="visible" class="batch-dialog-overlay" @click.self="$emit('close')">
        <div class="batch-dialog" @click.stop>
          <div class="batch-dialog-header">
            <span class="batch-dialog-title">{{ title }}</span>
            <button class="batch-dialog-close" @click="$emit('close')">×</button>
          </div>
          <div class="batch-dialog-body">
            <!-- Delete confirmation -->
            <template v-if="operation === 'delete'">
              <p class="batch-dialog-message">
                {{ t('redis.keys.batch.deleteConfirm', { count: keys.length }) }}
              </p>
              <div class="batch-dialog-key-list">
                <div v-for="key in displayedKeys" :key="key" class="batch-dialog-key-item">
                  <span class="batch-dialog-key-type">{{ keyTypeIcon }}</span>
                  <span class="batch-dialog-key-name">{{ key }}</span>
                </div>
                <div v-if="keys.length > maxDisplayKeys" class="batch-dialog-more">
                  {{ t('redis.keys.batch.andMore', { count: keys.length - maxDisplayKeys }) }}
                </div>
              </div>
            </template>

            <!-- TTL modification -->
            <template v-if="operation === 'setTtl'">
              <p class="batch-dialog-message">
                {{ t('redis.keys.batch.setTtlConfirm', { count: keys.length }) }}
              </p>
              <div class="batch-dialog-form">
                <label class="batch-dialog-label">{{ t('redis.keys.batch.ttlValue') }}</label>
                <div class="batch-dialog-input-row">
                  <input
                    v-model="ttlValue"
                    type="number"
                    class="batch-dialog-input"
                    min="1"
                    :placeholder="t('redis.keys.batch.ttlPlaceholder')"
                  />
                  <span class="batch-dialog-unit">s</span>
                </div>
                <div class="batch-dialog-presets">
                  <button class="batch-preset-btn" @click="ttlValue = '60'">1 {{ t('redis.keys.batch.minute') }}</button>
                  <button class="batch-preset-btn" @click="ttlValue = '3600'">1 {{ t('redis.keys.batch.hour') }}</button>
                  <button class="batch-preset-btn" @click="ttlValue = '86400'">1 {{ t('redis.keys.batch.day') }}</button>
                  <button class="batch-preset-btn" @click="ttlValue = '604800'">7 {{ t('redis.keys.batch.days') }}</button>
                </div>
                <p class="batch-dialog-hint">{{ t('redis.keys.batch.ttlHint') }}</p>
              </div>
            </template>

            <!-- Export -->
            <template v-if="operation === 'export'">
              <p class="batch-dialog-message">
                {{ t('redis.keys.batch.exportConfirm', { count: keys.length }) }}
              </p>
              <div class="batch-dialog-form">
                <label class="batch-dialog-label">{{ t('redis.keys.batch.exportFormat') }}</label>
                <div class="batch-dialog-radio-group">
                  <label class="batch-dialog-radio">
                    <input v-model="exportFormat" type="radio" value="json" />
                    <span>JSON</span>
                  </label>
                  <label class="batch-dialog-radio">
                    <input v-model="exportFormat" type="radio" value="csv" />
                    <span>CSV</span>
                  </label>
                </div>
              </div>
            </template>
          </div>
          <div class="batch-dialog-footer">
            <button class="batch-dialog-btn batch-dialog-btn-cancel" @click="$emit('close')">
              {{ t('redis.keys.batch.cancel') }}
            </button>
            <button
              v-if="operation === 'delete'"
              class="batch-dialog-btn batch-dialog-btn-danger"
              @click="$emit('confirmDelete', keys)"
            >
              {{ t('redis.keys.batch.deleteBtn', { count: keys.length }) }}
            </button>
            <button
              v-if="operation === 'setTtl'"
              class="batch-dialog-btn batch-dialog-btn-primary"
              :disabled="!ttlValid"
              @click="handleSetTtl"
            >
              {{ t('redis.keys.batch.applyTtl') }}
            </button>
            <button
              v-if="operation === 'export'"
              class="batch-dialog-btn batch-dialog-btn-primary"
              @click="handleExport"
            >
              {{ t('redis.keys.batch.exportBtn') }}
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

const { t } = useI18n()

const props = defineProps<{
  visible: boolean
  operation: 'delete' | 'setTtl' | 'export'
  keys: string[]
}>()

const emit = defineEmits<{
  close: []
  confirmDelete: [keys: string[]]
  confirmSetTtl: [keys: string[], seconds: number]
  confirmExport: [keys: string[], format: 'json' | 'csv']
}>()

const maxDisplayKeys = 20

const ttlValue = ref('3600')
const exportFormat = ref<'json' | 'csv'>('json')

const title = computed(() => {
  switch (props.operation) {
    case 'delete': return t('redis.keys.batch.deleteTitle')
    case 'setTtl': return t('redis.keys.batch.setTtlTitle')
    case 'export': return t('redis.keys.batch.exportTitle')
  }
})

const displayedKeys = computed(() => props.keys.slice(0, maxDisplayKeys))

const keyTypeIcon = computed(() => {
  // For batch operations, we show a generic icon
  return '⚡'
})

const ttlValid = computed(() => {
  const v = parseInt(ttlValue.value, 10)
  return !isNaN(v) && v > 0
})

watch(() => props.visible, (val) => {
  if (val) {
    ttlValue.value = '3600'
    exportFormat.value = 'json'
  }
})

function handleSetTtl() {
  const seconds = parseInt(ttlValue.value, 10)
  if (!isNaN(seconds) && seconds > 0) {
    emit('confirmSetTtl', props.keys, seconds)
  }
}

function handleExport() {
  emit('confirmExport', props.keys, exportFormat.value)
}
</script>

<style scoped>
.batch-dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

.batch-dialog {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 8px;
  width: 440px;
  max-width: 90vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.batch-dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}

.batch-dialog-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.batch-dialog-close {
  background: none;
  border: none;
  font-size: 18px;
  color: var(--text-muted);
  cursor: pointer;
  padding: 0;
  line-height: 1;
}

.batch-dialog-close:hover {
  color: var(--text-primary);
}

.batch-dialog-body {
  padding: 16px;
  overflow-y: auto;
  flex: 1;
}

.batch-dialog-message {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 0 0 12px 0;
  line-height: 1.5;
}

.batch-dialog-key-list {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 4px;
  max-height: 160px;
  overflow-y: auto;
  padding: 8px;
}

.batch-dialog-key-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-size: 12px;
  font-family: var(--font-mono);
}

.batch-dialog-key-type {
  color: var(--accent);
  font-weight: 600;
}

.batch-dialog-key-name {
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.batch-dialog-more {
  color: var(--text-muted);
  font-size: 11px;
  padding: 4px 0;
}

.batch-dialog-form {
  margin-top: 12px;
}

.batch-dialog-label {
  display: block;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 6px;
}

.batch-dialog-input-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.batch-dialog-input {
  flex: 1;
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  padding: 6px 10px;
  font-size: 13px;
  font-family: var(--font-mono);
}

.batch-dialog-input:focus {
  outline: none;
  border-color: var(--accent);
}

.batch-dialog-unit {
  color: var(--text-muted);
  font-size: 13px;
}

.batch-dialog-presets {
  display: flex;
  gap: 6px;
  margin-top: 8px;
  flex-wrap: wrap;
}

.batch-preset-btn {
  padding: 3px 8px;
  font-size: 11px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-secondary);
  cursor: pointer;
}

.batch-preset-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.batch-dialog-hint {
  font-size: 11px;
  color: var(--text-muted);
  margin: 8px 0 0 0;
}

.batch-dialog-radio-group {
  display: flex;
  gap: 16px;
}

.batch-dialog-radio {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text-primary);
  cursor: pointer;
}

.batch-dialog-radio input {
  accent-color: var(--accent);
}

.batch-dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid var(--border);
}

.batch-dialog-btn {
  padding: 6px 14px;
  font-size: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-weight: 500;
}

.batch-dialog-btn-cancel {
  background: var(--bg-surface);
  color: var(--text-secondary);
}

.batch-dialog-btn-cancel:hover {
  background: var(--bg-elevated);
}

.batch-dialog-btn-danger {
  background: #f85149;
  border-color: #f85149;
  color: #fff;
}

.batch-dialog-btn-danger:hover {
  background: #da3633;
}

.batch-dialog-btn-primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.batch-dialog-btn-primary:hover {
  opacity: 0.9;
}

.batch-dialog-btn-primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* Transition */
.dialog-enter-active,
.dialog-leave-active {
  transition: opacity 0.15s ease;
}

.dialog-enter-active .batch-dialog,
.dialog-leave-active .batch-dialog {
  transition: transform 0.15s ease;
}

.dialog-enter-from,
.dialog-leave-to {
  opacity: 0;
}

.dialog-enter-from .batch-dialog {
  transform: scale(0.95) translateY(-8px);
}

.dialog-leave-to .batch-dialog {
  transform: scale(0.95) translateY(8px);
}
</style>
