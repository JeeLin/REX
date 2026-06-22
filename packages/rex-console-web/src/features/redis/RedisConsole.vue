<template>
  <div class="redis-console">
    <!-- 顶部状态栏 -->
    <div class="redis-topbar">
      <span class="redis-status-dot" :class="{ connected: session.connected.value }" />
      <span class="redis-topbar-label">Redis</span>
      <span class="redis-topbar-name">{{ resourceName }}</span>
      <span class="redis-topbar-state">
        {{ session.connected.value ? t('redis.connected') : t('redis.disconnected') }}
      </span>
      <div class="redis-topbar-spacer" />
      <button
        v-if="!session.connected.value"
        class="redis-btn redis-btn-connect"
        @click="handleConnect"
      >
        {{ t('redis.connect') }}
      </button>
      <button
        v-else
        class="redis-btn redis-btn-disconnect"
        @click="session.disconnect()"
      >
        {{ t('redis.disconnect') }}
      </button>
      <button class="redis-btn" @click="showHistory = !showHistory">
        {{ t('redis.history') }}
      </button>
    </div>

    <div class="redis-body">
      <!-- 历史记录面板 -->
      <RedisHistory
        v-if="showHistory"
        :history="session.history.value"
        @select="handleHistorySelect"
        @clear="session.clearHistory()"
      />

      <!-- 主区域 -->
      <div class="redis-main">
        <!-- 输出区域 -->
        <div ref="outputRef" class="redis-output">
          <div v-if="!session.connected.value && !session.error.value" class="redis-welcome">
            {{ t('redis.welcome') }}
          </div>
          <div v-if="session.error.value" class="redis-error-banner">
            {{ session.error.value }}
          </div>
          <div
            v-for="entry in outputEntries"
            :key="entry.id"
            class="redis-entry"
          >
            <div class="redis-command-line">
              <span class="redis-prompt">&gt;</span>
              <span class="redis-command">{{ entry.command }}</span>
            </div>
            <div class="redis-response-line">
              <RedisResult v-if="entry.response" :value="entry.response" />
              <span v-if="entry.error" class="redis-error">{{ entry.error }}</span>
              <span v-if="entry.elapsed_ms !== undefined" class="redis-elapsed">
                ({{ entry.elapsed_ms }}ms)
              </span>
            </div>
          </div>
        </div>

        <!-- 输入区域 -->
        <div class="redis-input-area">
          <span class="redis-prompt">&gt;</span>
          <input
            ref="inputRef"
            v-model="inputValue"
            class="redis-input"
            :placeholder="t('redis.placeholder')"
            :disabled="!session.connected.value"
            @keydown="handleKeydown"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, nextTick, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRedisSession } from './useRedisSession'
import RedisResult from './RedisResult.vue'
import RedisHistory from './RedisHistory.vue'
import type { RedisValue } from '@/api/redis'

const props = defineProps<{
  resourceId: string
  resourceName: string
}>()

const { t } = useI18n()

const session = useRedisSession(() => props.resourceId)

const inputValue = ref('')
const outputRef = ref<HTMLDivElement>()
const inputRef = ref<HTMLInputElement>()
const showHistory = ref(false)

interface OutputEntry {
  id: number
  command: string
  response?: RedisValue
  error?: string
  elapsed_ms?: number
}

const outputEntries = ref<OutputEntry[]>([])
let nextEntryId = 0

async function handleConnect() {
  try {
    await session.connect()
  } catch {
    // error is set in session.error
  }
}

function handleHistorySelect(command: string) {
  inputValue.value = command
  showHistory.value = false
  inputRef.value?.focus()
}

async function handleKeydown(e: KeyboardEvent) {
  // Enter → 执行命令
  if (e.key === 'Enter') {
    e.preventDefault()
    const cmd = inputValue.value.trim()
    if (!cmd || !session.connected.value) return

    inputValue.value = ''
    session.addToHistory(cmd)

    const entry: OutputEntry = {
      id: nextEntryId++,
      command: cmd,
    }
    outputEntries.value.push(entry)

    try {
      const result = await session.execute(cmd)
      if (result.type === 'response') {
        entry.response = result.value
        entry.elapsed_ms = result.elapsed_ms
      } else if (result.type === 'error') {
        entry.error = result.message
      }
    } catch (err: unknown) {
      entry.error = err instanceof Error ? err.message : String(err)
    }

    await nextTick()
    if (outputRef.value) {
      outputRef.value.scrollTop = outputRef.value.scrollHeight
    }
    return
  }

  // ↑ 历史上翻
  if (e.key === 'ArrowUp') {
    e.preventDefault()
    const cmd = session.historyUp()
    if (cmd !== null) inputValue.value = cmd
    return
  }

  // ↓ 历史下翻
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    const cmd = session.historyDown()
    if (cmd !== null) inputValue.value = cmd
    return
  }

  // Ctrl+L 清屏
  if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
    e.preventDefault()
    outputEntries.value = []
    return
  }
}

onMounted(() => {
  inputRef.value?.focus()
})
</script>

<style scoped>
.redis-console {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
}

/* 顶部状态栏 */
.redis-topbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  font-size: 13px;
  flex-shrink: 0;
}
.redis-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #f85149;
}
.redis-status-dot.connected { background: #3fb950; }
.redis-topbar-label { font-weight: 600; color: #f85149; }
.redis-topbar-name { color: var(--text-secondary); }
.redis-topbar-state { color: var(--text-secondary); font-size: 12px; }
.redis-topbar-spacer { flex: 1; }

/* 按钮 */
.redis-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-primary);
  padding: 3px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}
.redis-btn:hover { background: var(--bg-hover); }
.redis-btn-connect { border-color: #3fb950; color: #3fb950; }
.redis-btn-disconnect { border-color: #f85149; color: #f85149; }

/* 主区域 */
.redis-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}
.redis-main {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}

/* 输出区域 */
.redis-output {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
}
.redis-welcome {
  color: var(--text-secondary);
  font-size: 13px;
  padding: 20px 0;
}
.redis-error-banner {
  color: #f85149;
  padding: 8px 12px;
  margin-bottom: 8px;
  border: 1px solid #f8514933;
  border-radius: 4px;
  background: #f8514911;
}

/* 命令/响应条目 */
.redis-entry { margin-bottom: 8px; }
.redis-command-line {
  display: flex;
  gap: 8px;
  margin-bottom: 2px;
}
.redis-prompt { color: #3fb950; font-weight: 600; }
.redis-command { color: #e6edf3; }
.redis-response-line {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding-left: 16px;
}
.redis-elapsed { color: var(--text-secondary); font-size: 11px; }
.redis-error { color: #f85149; }

/* 输入区域 */
.redis-input-area {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-top: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  flex-shrink: 0;
}
.redis-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-primary);
  font-family: inherit;
  font-size: 13px;
}
.redis-input::placeholder { color: var(--text-secondary); }
</style>
