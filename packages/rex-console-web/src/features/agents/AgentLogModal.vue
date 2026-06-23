<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-overlay" @click="$emit('close')">
      <div class="log-panel" @click.stop>
        <div class="log-header">
          <div class="log-title">
            <span style="color: var(--success)">●</span>
            <span>{{ t('ctx.logTitle') }}</span>
          </div>
          <button class="modal-close" @click="$emit('close')">×</button>
        </div>
        <div class="log-toolbar">
          <button
            v-for="level in logLevels"
            :key="level.value"
            class="log-filter-btn"
            :class="{ active: activeLevel === level.value }"
            @click="activeLevel = level.value"
          >
            {{ level.label }}
          </button>
          <span class="log-spacer"></span>
          <label class="log-auto-scroll">
            <input v-model="autoScroll" type="checkbox" style="accent-color: var(--accent)">
            {{ t('ctx.autoScroll') }}
          </label>
        </div>
        <div ref="logBodyRef" class="log-body">
          <div
            v-for="(log, idx) in filteredLogs"
            :key="idx"
            class="log-line"
            :class="log.level"
          >
            <span class="log-time">{{ log.time }}</span>
            <span class="log-level-tag">[{{ log.level.toUpperCase() }}]</span>
            <span class="log-message">{{ log.message }}</span>
          </div>
        </div>
        <div class="log-footer">
          <span>{{ filteredLogs.length }} 行</span>
          <span>{{ t('ctx.realTime') }}</span>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'

defineProps<{ visible: boolean }>()
defineEmits<{ close: [] }>()

const { t } = useI18n()

const activeLevel = ref('all')
const autoScroll = ref(true)
const logBodyRef = ref<HTMLElement | null>(null)

const logLevels = [
  { value: 'all', label: '全部' },
  { value: 'info', label: 'INFO' },
  { value: 'warn', label: 'WARN' },
  { value: 'error', label: 'ERROR' },
  { value: 'debug', label: 'DEBUG' },
]

const mockLogs = [
  { time: '16:51:30', level: 'info', message: 'TLS 握手完成' },
  { time: '16:51:28', level: 'info', message: 'Token 验证成功' },
  { time: '16:50:15', level: 'info', message: 'SSH 会话建立 · root@192.168.1.100' },
  { time: '16:49:01', level: 'info', message: 'MySQL 代理隧道建立 · db.internal:3306' },
  { time: '16:48:22', level: 'warn', message: '延迟告警: 125ms (阈值 100ms)' },
  { time: '16:47:50', level: 'debug', message: '心跳发送 · latency=12ms' },
  { time: '16:47:00', level: 'info', message: '资源扫描完成 · 发现 4 个资源' },
  { time: '16:45:13', level: 'error', message: 'SSH 连接失败: Connection refused (port 22)' },
  { time: '16:44:00', level: 'debug', message: '心跳发送 · latency=8ms' },
  { time: '16:43:22', level: 'info', message: 'SFTP 会话建立 · /opt/rex/' },
  { time: '16:42:10', level: 'warn', message: '磁盘空间告警: /data 剩余 12%' },
  { time: '16:41:00', level: 'debug', message: '心跳发送 · latency=10ms' },
]

const filteredLogs = computed(() => {
  if (activeLevel.value === 'all') return mockLogs
  return mockLogs.filter(l => l.level === activeLevel.value)
})

watch(filteredLogs, async () => {
  if (autoScroll.value) {
    await nextTick()
    if (logBodyRef.value) {
      logBodyRef.value.scrollTop = logBodyRef.value.scrollHeight
    }
  }
})
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

.log-panel {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  width: 680px;
  max-width: 90vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-lg);
  animation: modalIn 0.2s ease;
  overflow: hidden;
}

@keyframes modalIn {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}

.log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-lg) var(--sp-xl);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.log-title {
  font-family: var(--font-mono);
  font-size: var(--fs-md);
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.modal-close {
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--fs-md);
}

.modal-close:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.log-toolbar {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  padding: var(--sp-sm) var(--sp-xl);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.log-filter-btn {
  padding: var(--sp-xs) var(--sp-sm);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-deep);
  color: var(--text-secondary);
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.log-filter-btn:hover {
  color: var(--text-primary);
  border-color: var(--text-muted);
}

.log-filter-btn.active {
  background: var(--accent);
  color: #000;
  border-color: var(--accent);
  font-weight: 600;
}

.log-spacer {
  flex: 1;
}

.log-auto-scroll {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  font-size: var(--fs-xs);
  color: var(--text-secondary);
  cursor: pointer;
}

.log-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--sp-md) var(--sp-xl);
  background: var(--bg-deep);
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  line-height: 1.8;
  min-height: 300px;
}

.log-line {
  display: flex;
  gap: var(--sp-sm);
}

.log-time {
  color: var(--text-muted);
  white-space: nowrap;
}

.log-level-tag {
  font-weight: 600;
  white-space: nowrap;
}

.log-message {
  color: var(--text-primary);
  word-break: break-all;
}

.log-line.info .log-level-tag { color: var(--info); }
.log-line.warn .log-level-tag { color: var(--warning); }
.log-line.error .log-level-tag { color: var(--danger); }
.log-line.debug .log-level-tag { color: var(--text-muted); }

.log-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-sm) var(--sp-xl);
  border-top: 1px solid var(--border);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  flex-shrink: 0;
}
</style>
