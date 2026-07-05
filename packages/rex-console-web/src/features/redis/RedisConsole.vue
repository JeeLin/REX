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
      <select
        v-if="session.connected.value"
        v-model="selectedDb"
        class="redis-db-select"
        @change="handleDbChange"
      >
        <option v-for="n in 16" :key="n - 1" :value="n - 1">DB{{ n - 1 }}</option>
      </select>
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
      <button
        v-if="session.connected.value"
        class="redis-btn redis-btn-create"
        @click="showCreateKey = true"
      >
        {{ t('redis.keys.createKey') }}
      </button>
    </div>

    <div class="redis-body">
      <!-- 键浏览器 -->
      <RedisKeyBrowser
        v-if="session.connected.value && showKeyBrowser"
        :connected="session.connected.value"
        :keys="keyBrowserKeys"
        @selectKey="handleKeySelect"
        @search="handleKeyBrowserSearch"
        @deleteKey="handleKeyBrowserDelete"
        @setTtl="handleKeyBrowserSetTtl"
      />

      <!-- 历史记录面板 -->
      <RedisHistory
        v-if="showHistory"
        :history="session.history.value"
        @select="handleHistorySelect"
        @clear="session.clearHistory()"
      />

      <!-- 主区域 -->
      <div class="redis-main">
        <!-- 值查看器 -->
        <RedisValueViewer
          v-if="selectedKey"
          :key-name="selectedKey"
          :value-type="selectedKeyType"
          :value="selectedKeyValue"
          :ttl="selectedKeyTtl"
          :loading="valueLoading"
          @refresh="refreshSelectedKey"
          @deleteKey="deleteSelectedKey"
          @saveString="handleSaveString"
          @saveHash="handleSaveHash"
          @saveList="handleSaveList"
          @saveSet="handleSaveSet"
          @saveZset="handleSaveZset"
        />

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
        <div class="redis-input-wrapper">
          <div v-if="showAutocomplete" class="redis-autocomplete">
            <div
              v-for="(cmd, index) in filteredCommands"
              :key="cmd"
              class="redis-autocomplete-item"
              :class="{ active: index === autocompleteIndex }"
              @mousedown.prevent="selectAutocomplete(cmd)"
            >
              {{ cmd }}
            </div>
          </div>
          <div class="redis-input-area">
            <span class="redis-prompt">&gt;</span>
            <input
              ref="inputRef"
              v-model="inputValue"
              class="redis-input"
              :placeholder="t('redis.placeholder')"
              :disabled="!session.connected.value"
              @input="updateAutocomplete"
              @keydown="handleKeydown"
            />
          </div>
        </div>
      </div>
    </div>

    <!-- Create Key Modal -->
    <div v-if="showCreateKey" class="redis-modal-overlay" @click.self="showCreateKey = false">
      <div class="redis-modal" @click.stop>
        <div class="redis-modal-header">
          <span>{{ t('redis.keys.createKeyTitle') }}</span>
          <button class="redis-modal-close" @click="showCreateKey = false">×</button>
        </div>
        <div class="redis-modal-body">
          <div class="redis-form-row">
            <label>{{ t('redis.keys.keyName') }}</label>
            <input
              v-model="createKeyForm.key"
              class="redis-form-input"
              :placeholder="t('redis.keys.keyNamePlaceholder')"
            />
          </div>
          <div class="redis-form-row">
            <label>{{ t('redis.keys.type') }}</label>
            <div class="redis-type-selector">
              <button
                v-for="t in createKeyTypes"
                :key="t"
                class="redis-type-btn"
                :class="{ active: createKeyForm.type === t }"
                @click="createKeyForm.type = t"
              >
                {{ t }}
              </button>
            </div>
          </div>
          <div class="redis-form-row">
            <label>{{ t('redis.keys.value') }}</label>
            <div v-if="createKeyForm.type === 'string'" class="redis-value-input">
              <textarea
                v-model="createKeyForm.stringValue"
                class="redis-form-textarea"
                rows="3"
                :placeholder="t('redis.keys.stringValuePlaceholder')"
              />
            </div>
            <div v-else-if="createKeyForm.type === 'hash'" class="redis-value-input">
              <div
                v-for="(entry, idx) in createKeyForm.hashEntries"
                :key="idx"
                class="redis-hash-entry"
              >
                <input
                  v-model="entry.field"
                  class="redis-form-input redis-hash-field"
                  :placeholder="t('redis.keys.field')"
                />
                <input
                  v-model="entry.value"
                  class="redis-form-input redis-hash-value"
                  :placeholder="t('redis.keys.fieldValue')"
                />
                <button class="redis-btn redis-btn-sm redis-btn-danger" @click="createKeyForm.hashEntries.splice(idx, 1)">×</button>
              </div>
              <button class="redis-btn redis-btn-sm" @click="createKeyForm.hashEntries.push({ field: '', value: '' })">
                + {{ t('redis.keys.addField') }}
              </button>
            </div>
            <div v-else-if="createKeyForm.type === 'list'" class="redis-value-input">
              <div v-for="(val, idx) in createKeyForm.listValues" :key="idx" class="redis-list-entry">
                <input
                  v-model="createKeyForm.listValues[idx]"
                  class="redis-form-input"
                  :placeholder="t('redis.keys.element')"
                />
                <button class="redis-btn redis-btn-sm redis-btn-danger" @click="createKeyForm.listValues.splice(idx, 1)">×</button>
              </div>
              <button class="redis-btn redis-btn-sm" @click="createKeyForm.listValues.push('')">
                + {{ t('redis.keys.addElement') }}
              </button>
            </div>
            <div v-else-if="createKeyForm.type === 'set'" class="redis-value-input">
              <div v-for="(val, idx) in createKeyForm.setMembers" :key="idx" class="redis-list-entry">
                <input
                  v-model="createKeyForm.setMembers[idx]"
                  class="redis-form-input"
                  :placeholder="t('redis.keys.member')"
                />
                <button class="redis-btn redis-btn-sm redis-btn-danger" @click="createKeyForm.setMembers.splice(idx, 1)">×</button>
              </div>
              <button class="redis-btn redis-btn-sm" @click="createKeyForm.setMembers.push('')">
                + {{ t('redis.keys.addMember') }}
              </button>
            </div>
            <div v-else-if="createKeyForm.type === 'zset'" class="redis-value-input">
              <div v-for="(entry, idx) in createKeyForm.zsetEntries" :key="idx" class="redis-hash-entry">
                <input
                  v-model="entry.member"
                  class="redis-form-input redis-hash-field"
                  :placeholder="t('redis.keys.member')"
                />
                <input
                  v-model="entry.score"
                  class="redis-form-input redis-hash-value"
                  type="number"
                  :placeholder="t('redis.keys.score')"
                />
                <button class="redis-btn redis-btn-sm redis-btn-danger" @click="createKeyForm.zsetEntries.splice(idx, 1)">×</button>
              </div>
              <button class="redis-btn redis-btn-sm" @click="createKeyForm.zsetEntries.push({ member: '', score: '0' })">
                + {{ t('redis.keys.addMember') }}
              </button>
            </div>
          </div>
        </div>
        <div class="redis-modal-footer">
          <button class="redis-btn" @click="showCreateKey = false">{{ t('common.cancel') }}</button>
          <button
            class="redis-btn redis-btn-create"
            :disabled="!createKeyForm.key.trim()"
            @click="handleCreateKey"
          >
            {{ t('redis.keys.create') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, nextTick, onMounted, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRedisSession } from './useRedisSession'
import RedisResult from './RedisResult.vue'
import RedisHistory from './RedisHistory.vue'
import RedisKeyBrowser from './RedisKeyBrowser.vue'
import RedisValueViewer from './RedisValueViewer.vue'
import type { RedisValue } from '@/api/redis'
import type { KeyWithType, OutputEntry } from './types'

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
const showKeyBrowser = ref(true)
const selectedDb = ref(0)

// Key browser state
const keyBrowserKeys = ref<KeyWithType[]>([])
const searchPattern = ref('*')

// Autocomplete state
const showAutocomplete = ref(false)
const filteredCommands = ref<string[]>([])
const autocompleteIndex = ref(0)

// Value viewer state
const selectedKey = ref<string | null>(null)
const selectedKeyType = ref('string')
const selectedKeyValue = ref<RedisValue | null>(null)
const selectedKeyTtl = ref<number | null>(null)
const valueLoading = ref(false)

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

async function handleDbChange() {
  if (!session.connected.value) return
  await session.execute(`SELECT ${selectedDb.value}`)
}

function updateAutocomplete() {
  const input = inputValue.value.trim().toUpperCase()
  if (!input) {
    showAutocomplete.value = false
    return
  }
  filteredCommands.value = session.REDIS_COMMANDS.filter(cmd => cmd.startsWith(input)).slice(0, 10)
  autocompleteIndex.value = 0
  showAutocomplete.value = filteredCommands.value.length > 0 && input !== filteredCommands.value[0]
}

function selectAutocomplete(cmd: string) {
  inputValue.value = cmd + ' '
  showAutocomplete.value = false
  inputRef.value?.focus()
}

function handleKeySelect(key: string) {
  selectedKey.value = key
  loadKeyValue(key)
}

async function handleKeyBrowserSearch(pattern: string) {
  if (!session.connected.value) return
  keyBrowserKeys.value = []
  const cmd = `SCAN 0 MATCH ${pattern} COUNT 1000`
  try {
    const result = await session.execute(cmd)
    if (result.type === 'response' && result.value.type === 'Array') {
      const items = result.value.value as RedisValue[]
      const parsed: KeyWithType[] = []
      for (const item of items) {
        if (item.type === 'Array' && item.value.length >= 2) {
          const keyVal = item.value[0]
          const typeVal = item.value[1]
          if (keyVal.type === 'Bulk' && typeVal.type === 'Bulk') {
            parsed.push({
              key: keyVal.value ?? '',
              type: typeVal.value ?? 'unknown',
            })
          }
        }
      }
      keyBrowserKeys.value = parsed
    }
  } catch {
    // ignore errors
  }
}

async function loadKeyValue(key: string) {
  if (!session.connected.value) return
  valueLoading.value = true

  try {
    // Get type
    const typeResult = await session.execute(`TYPE ${key}`)
    if (typeResult.type === 'response' && typeResult.value.type === 'Status') {
      selectedKeyType.value = typeResult.value.value
    }

    // Get value based on type
    let valueResult
    switch (selectedKeyType.value) {
      case 'hash':
        valueResult = await session.execute(`HGETALL ${key}`)
        break
      case 'list':
        valueResult = await session.execute(`LRANGE ${key} 0 -1`)
        break
      case 'set':
        valueResult = await session.execute(`SMEMBERS ${key}`)
        break
      case 'zset':
        valueResult = await session.execute(`ZRANGE ${key} 0 -1 WITHSCORES`)
        break
      default:
        valueResult = await session.execute(`GET ${key}`)
    }

    if (valueResult.type === 'response') {
      selectedKeyValue.value = valueResult.value
    }

    // Get TTL
    const ttlResult = await session.execute(`TTL ${key}`)
    if (ttlResult.type === 'response' && ttlResult.value.type === 'Integer') {
      selectedKeyTtl.value = ttlResult.value.value
    }
  } catch {
    // ignore errors
  } finally {
    valueLoading.value = false
  }
}

function refreshSelectedKey() {
  if (selectedKey.value) {
    loadKeyValue(selectedKey.value)
  }
}

async function deleteSelectedKey(key: string) {
  if (!session.connected.value) return
  await session.execute(`DEL ${key}`)
  selectedKey.value = null
  selectedKeyValue.value = null
}

async function handleKeyBrowserDelete(key: string) {
  if (!session.connected.value) return
  await session.execute(`DEL ${key}`)
  // Refresh key browser
  handleKeyBrowserSearch(searchPattern.value || '*')
  // Clear value viewer if this key was selected
  if (selectedKey.value === key) {
    selectedKey.value = null
    selectedKeyValue.value = null
  }
}

async function handleKeyBrowserSetTtl(key: string, seconds: number) {
  if (!session.connected.value) return
  await session.execute(`EXPIRE ${key} ${seconds}`)
}

async function handleSaveString(key: string, value: string) {
  if (!session.connected.value) return
  await session.execute(`SET ${key} ${value}`)
  refreshSelectedKey()
}

async function handleSaveHash(key: string, added: { field: string; value: string }[], removed: string[]) {
  if (!session.connected.value) return
  for (const field of removed) {
    await session.execute(`HDEL ${key} ${field}`)
  }
  for (const entry of added) {
    if (entry.field.trim()) {
      await session.execute(`HSET ${key} ${entry.field} ${entry.value}`)
    }
  }
  refreshSelectedKey()
}

async function handleSaveList(key: string, added: string[], removedIndices: number[]) {
  if (!session.connected.value) return
  // Remove from end to preserve indices
  for (let i = removedIndices.length - 1; i >= 0; i--) {
    const idx = removedIndices[i]
    await session.execute(`LSET ${key} ${idx} __REX_DEL__`)
    await session.execute(`LREM ${key} 1 __REX_DEL__`)
  }
  for (const val of added) {
    if (val.trim()) await session.execute(`RPUSH ${key} ${val}`)
  }
  refreshSelectedKey()
}

async function handleSaveSet(key: string, added: string[], removed: string[]) {
  if (!session.connected.value) return
  for (const member of removed) {
    await session.execute(`SREM ${key} ${member}`)
  }
  for (const member of added) {
    if (member.trim()) await session.execute(`SADD ${key} ${member}`)
  }
  refreshSelectedKey()
}

async function handleSaveZset(key: string, added: { member: string; score: string }[], removed: string[]) {
  if (!session.connected.value) return
  for (const member of removed) {
    await session.execute(`ZREM ${key} ${member}`)
  }
  for (const entry of added) {
    if (entry.member.trim()) {
      await session.execute(`ZADD ${key} ${entry.score} ${entry.member}`)
    }
  }
  refreshSelectedKey()
}

// Create key state
const showCreateKey = ref(false)
const createKeyTypes = ['string', 'hash', 'list', 'set', 'zset']
const createKeyForm = reactive({
  key: '',
  type: 'string',
  stringValue: '',
  hashEntries: [{ field: '', value: '' }],
  listValues: [''],
  setMembers: [''],
  zsetEntries: [{ member: '', score: '0' }],
})

async function handleCreateKey() {
  if (!session.connected.value || !createKeyForm.key.trim()) return
  const key = createKeyForm.key.trim()

  try {
    switch (createKeyForm.type) {
      case 'string':
        await session.execute(`SET ${key} ${createKeyForm.stringValue}`)
        break
      case 'hash':
        for (const entry of createKeyForm.hashEntries) {
          if (entry.field.trim()) {
            await session.execute(`HSET ${key} ${entry.field} ${entry.value}`)
          }
        }
        break
      case 'list':
        for (const val of createKeyForm.listValues) {
          if (val.trim()) await session.execute(`RPUSH ${key} ${val}`)
        }
        break
      case 'set':
        for (const member of createKeyForm.setMembers) {
          if (member.trim()) await session.execute(`SADD ${key} ${member}`)
        }
        break
      case 'zset':
        for (const entry of createKeyForm.zsetEntries) {
          if (entry.member.trim()) {
            await session.execute(`ZADD ${key} ${entry.score} ${entry.member}`)
          }
        }
        break
    }
    showCreateKey.value = false
    // Reset form
    createKeyForm.key = ''
    createKeyForm.stringValue = ''
    createKeyForm.hashEntries = [{ field: '', value: '' }]
    createKeyForm.listValues = ['']
    createKeyForm.setMembers = ['']
    createKeyForm.zsetEntries = [{ member: '', score: '0' }]
    // Refresh key browser
    handleKeyBrowserSearch(searchPattern.value || '*')
  } catch {
    // errors are shown in console output
  }
}

async function handleKeydown(e: KeyboardEvent) {
  // Autocomplete navigation
  if (showAutocomplete.value) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      autocompleteIndex.value = (autocompleteIndex.value + 1) % filteredCommands.value.length
      return
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      autocompleteIndex.value = (autocompleteIndex.value - 1 + filteredCommands.value.length) % filteredCommands.value.length
      return
    }
    if (e.key === 'Tab') {
      e.preventDefault()
      selectAutocomplete(filteredCommands.value[autocompleteIndex.value])
      return
    }
    if (e.key === 'Escape') {
      showAutocomplete.value = false
      return
    }
  }

  // Enter → 执行命令
  if (e.key === 'Enter') {
    e.preventDefault()
    showAutocomplete.value = false
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
  // 自动连接（如果尚未连接）
  if (!session.connected.value) {
    session.connect().catch(() => {
      // 连接失败时静默处理，用户可手动点击连接按钮
    })
  }
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

/* 数据库选择器 */
.redis-db-select {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-primary);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
}
.redis-db-select:focus {
  outline: none;
  border-color: var(--accent);
}

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

/* 自动补全 */
.redis-input-wrapper {
  position: relative;
  flex-shrink: 0;
}
.redis-autocomplete {
  position: absolute;
  bottom: 100%;
  left: 0;
  right: 0;
  max-height: 200px;
  overflow-y: auto;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-bottom: none;
  border-radius: 4px 4px 0 0;
  z-index: 10;
}
.redis-autocomplete-item {
  padding: 4px 12px;
  font-size: 12px;
  font-family: inherit;
  color: var(--text-primary);
  cursor: pointer;
}
.redis-autocomplete-item:hover,
.redis-autocomplete-item.active {
  background: var(--bg-hover);
  color: var(--accent);
}

/* Create Key Modal */
.redis-btn-create {
  border-color: #3fb950;
  color: #3fb950;
}

.redis-modal-overlay {
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

.redis-modal {
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
  border-radius: 8px;
  width: 480px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
}

.redis-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-primary);
  font-weight: 600;
  font-size: 14px;
}

.redis-modal-close {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 18px;
  cursor: pointer;
  padding: 0 4px;
}

.redis-modal-close:hover {
  color: var(--text-primary);
}

.redis-modal-body {
  padding: 16px;
  overflow-y: auto;
  flex: 1;
}

.redis-modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid var(--border-primary);
}

.redis-form-row {
  margin-bottom: 12px;
}

.redis-form-row label {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.redis-form-input {
  width: 100%;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-primary);
  padding: 6px 10px;
  border-radius: 4px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
  box-sizing: border-box;
}

.redis-form-input:focus {
  outline: none;
  border-color: var(--accent);
}

.redis-form-textarea {
  width: 100%;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-primary);
  padding: 6px 10px;
  border-radius: 4px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
  resize: vertical;
  box-sizing: border-box;
}

.redis-form-textarea:focus {
  outline: none;
  border-color: var(--accent);
}

.redis-type-selector {
  display: flex;
  gap: 4px;
}

.redis-type-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-secondary);
  padding: 4px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  font-family: 'JetBrains Mono', monospace;
}

.redis-type-btn.active {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
}

.redis-value-input {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.redis-hash-entry,
.redis-list-entry {
  display: flex;
  gap: 4px;
  align-items: center;
}

.redis-hash-field {
  flex: 1;
}

.redis-hash-value {
  flex: 1;
}

.redis-btn-danger {
  color: #f85149;
  border-color: #f85149;
}

.redis-btn-danger:hover {
  background: #f8514922;
}

.redis-btn-sm {
  padding: 3px 8px;
  font-size: 11px;
}
</style>
