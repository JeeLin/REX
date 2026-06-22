<template>
  <div class="docker-logs">
    <div class="docker-logs-toolbar">
      <span class="docker-logs-title">{{ containerName }}</span>
      <div class="docker-logs-spacer" />
      <label class="docker-logs-auto">
        <input v-model="autoScroll" type="checkbox" checked />
        {{ t('docker.logs.autoScroll') }}
      </label>
      <button class="docker-btn docker-btn-sm" @click="$emit('close')">
        {{ t('docker.logs.close') }}
      </button>
    </div>
    <div ref="logRef" class="docker-logs-output">
      <pre v-if="logs" class="docker-logs-content">{{ logs }}</pre>
      <div v-else-if="loading" class="docker-logs-loading">
        {{ t('common.loading') }}...
      </div>
      <div v-else-if="errorMsg" class="docker-logs-error">
        {{ errorMsg }}
      </div>
      <div v-else class="docker-logs-empty">
        {{ t('docker.logs.noLogs') }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  containerName: string
  logs: string | null
  loading: boolean
  errorMsg: string | null
}>()

defineEmits<{
  close: []
}>()

const { t } = useI18n()

const logRef = ref<HTMLDivElement>()
const autoScroll = ref(true)

watch(
  () => props.logs,
  async () => {
    if (autoScroll.value) {
      await nextTick()
      if (logRef.value) {
        logRef.value.scrollTop = logRef.value.scrollHeight
      }
    }
  },
)
</script>

<style scoped>
.docker-logs {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
}

.docker-logs-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  font-size: 12px;
  flex-shrink: 0;
}

.docker-logs-title {
  font-weight: 600;
  color: var(--text-primary);
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
}

.docker-logs-spacer { flex: 1; }

.docker-logs-auto {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--text-secondary);
  font-size: 11px;
  cursor: pointer;
}

.docker-logs-auto input {
  cursor: pointer;
}

.docker-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-primary);
  padding: 3px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}

.docker-btn:hover { background: var(--bg-hover); }

.docker-logs-output {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
}

.docker-logs-content {
  margin: 0;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-all;
}

.docker-logs-loading,
.docker-logs-error,
.docker-logs-empty {
  color: var(--text-secondary);
  font-size: 13px;
  padding: 20px 0;
}

.docker-logs-error {
  color: var(--danger, #f85149);
}
</style>
