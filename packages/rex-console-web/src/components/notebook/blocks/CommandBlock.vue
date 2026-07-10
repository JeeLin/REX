<template>
  <div
    class="command-block"
    :class="[stateClass, protocolClass]"
    :data-block-id="blockId"
  >
    <!-- Header: resource info + status -->
    <div class="command-header">
      <span class="command-icon">{{ protocolIcon }}</span>
      <span class="command-label">{{ t('notebooks.editor.blockType.command') }}</span>
      <span v-if="protocol" class="command-protocol-badge">{{ protocol }}</span>
      <span v-if="executionState === 'executing'" class="command-status executing">
        <span class="spinner" />
        {{ t('notebooks.editor.command.executing') }}
      </span>
      <span v-else-if="executionState === 'completed'" class="command-status completed">
        {{ t('notebooks.editor.command.completed') }}
      </span>
      <span v-else-if="executionState === 'failed'" class="command-status failed">
        {{ t('notebooks.editor.command.failed') }}
      </span>
    </div>

    <!-- Unbound state: resource picker -->
    <div v-if="!resourceId" class="command-unbound">
      <ResourcePicker
        :model-value="resourceId ?? null"
        :protocol="protocol ?? null"
        @update:model-value="onResourceSelected"
        @update:protocol="onProtocolChanged"
      />
    </div>

    <!-- Bound state: command input -->
    <div v-else class="command-bound">
      <div class="command-input-wrapper">
        <span class="command-prompt">{{ promptSymbol }}</span>
        <textarea
          ref="inputRef"
          v-model="commandInput"
          class="command-input"
          :class="protocolClass"
          :placeholder="t('notebooks.editor.command.executeHint')"
          :rows="isSSH ? 3 : 1"
          :spellcheck="false"
          @keydown="handleKeydown"
          @focus="$emit('focus')"
          @blur="$emit('blur')"
        />
      </div>
      <div class="command-actions">
        <button
          class="command-run-btn"
          :disabled="!commandInput.trim() || executionState === 'executing'"
          @click="execute"
          type="button"
        >
          <span v-if="executionState === 'executing'" class="spinner" />
          <span v-else>▶</span>
          {{ executionState === 'executing' ? t('notebooks.editor.command.executing') : 'Run' }}
        </button>
      </div>
    </div>

    <!-- Result panel -->
    <ResultPanel
      v-if="lastExecution"
      :execution="lastExecution"
      :protocol="protocol"
    />

    <!-- History toggle -->
    <div v-if="resourceId" class="command-history-toggle">
      <button
        class="history-toggle-btn"
        :class="{ active: showHistory }"
        @click="showHistory = !showHistory"
        type="button"
      >
        {{ showHistory ? '▾' : '▸' }} {{ t('notebooks.editor.history.toggle') }}
      </button>
    </div>

    <!-- Execution history -->
    <ExecutionHistory
      v-if="showHistory"
      :block-id="blockId"
      :protocol="protocol"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { executeCommand } from '@/api/notebook'
import type { NotebookExecution } from '@/api/notebook'
import ResourcePicker from '../ResourcePicker.vue'
import ResultPanel from '../ResultPanel.vue'
import ExecutionHistory from '../ExecutionHistory.vue'
import { PROTOCOL_ICONS } from '@/utils/protocols'

const props = defineProps<{
  blockId: string
  content: string
  resourceId?: string
  protocol?: string
}>()

const emit = defineEmits<{
  'update:resource-id': [id: string]
  'update:protocol': [protocol: string]
  'update:content': [content: string]
  execute: [execution: NotebookExecution]
  focus: []
  blur: []
}>()

const { t } = useI18n()

const inputRef = ref<HTMLTextAreaElement>()
const commandInput = ref(props.content || '')
const executionState = ref<'idle' | 'executing' | 'completed' | 'failed'>('idle')
const lastExecution = ref<NotebookExecution | null>(null)
const showHistory = ref(false)

const protocolIcon = computed(() => {
  if (!props.protocol) return '⚡'
  return PROTOCOL_ICONS[props.protocol.toLowerCase()] ?? '⚡'
})

const isSSH = computed(() => props.protocol?.toLowerCase() === 'ssh')

const promptSymbol = computed(() => {
  switch (props.protocol?.toLowerCase()) {
    case 'ssh':
    case 'terminal':
      return '$'
    case 'sql':
      return 'sql>'
    case 'redis':
      return 'redis>'
    default:
      return '>'
  }
})

const protocolClass = computed(() => {
  if (!props.protocol) return ''
  return `protocol-${props.protocol.toLowerCase()}`
})

const stateClass = computed(() => {
  if (!props.resourceId) return 'state-unbound'
  if (executionState.value === 'executing') return 'state-executing'
  if (executionState.value === 'completed') return 'state-completed'
  if (executionState.value === 'failed') return 'state-failed'
  return 'state-bound'
})

// Sync content from props
watch(() => props.content, (val) => {
  if (val !== commandInput.value) {
    commandInput.value = val || ''
  }
})

// Sync commandInput back to block content
watch(commandInput, (val) => {
  emit('update:content', val)
})

function onResourceSelected(id: string | null) {
  if (id) emit('update:resource-id', id)
}

function onProtocolChanged(protocol: string | null) {
  if (protocol) emit('update:protocol', protocol)
}

function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault()
    execute()
  }
  // Allow Enter for multi-line in SSH/terminal, prevent default otherwise
  if (e.key === 'Enter' && !e.ctrlKey && !e.metaKey) {
    if (isSSH.value || props.protocol?.toLowerCase() === 'terminal') {
      // Allow natural newline for multi-line commands
      return
    }
  }
}

async function execute() {
  const cmd = commandInput.value.trim()
  if (!cmd || executionState.value === 'executing') return

  executionState.value = 'executing'
  try {
    const result = await executeCommand(props.blockId)
    lastExecution.value = result
    executionState.value = result.status === 'success' ? 'completed' : 'failed'
    emit('execute', result)
  } catch (err) {
    executionState.value = 'failed'
    lastExecution.value = {
      id: '',
      block_id: props.blockId,
      status: 'error',
      output: err instanceof Error ? err.message : String(err),
      duration_ms: null,
      executed_at: new Date().toISOString(),
    }
  }
}

function focus() {
  nextTick(() => {
    inputRef.value?.focus()
  })
}

defineExpose({ focus })
</script>

<style scoped>
.command-block {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
  overflow: hidden;
}

.command-block.state-executing {
  border-color: var(--warning, #f59e0b);
}

.command-block.state-completed {
  border-color: var(--success, #10b981);
}

.command-block.state-failed {
  border-color: var(--danger, #ef4444);
}

/* Protocol-specific terminal styling */
.command-block.protocol-ssh,
.command-block.protocol-terminal {
  background: #1a1a2e;
  border-color: #333366;
}

.command-block.protocol-ssh .command-header,
.command-block.protocol-terminal .command-header {
  color: #a0a0c0;
}

.command-block.protocol-ssh .command-input,
.command-block.protocol-terminal .command-input {
  background: #0d0d1a;
  color: #00ff88;
  font-family: var(--font-mono);
}

/* SQL syntax styling */
.command-block.protocol-sql {
  border-color: #4a90d9;
}

.command-block.protocol-sql .command-input {
  font-family: var(--font-mono);
  color: #c0c0ff;
}

/* Redis styling */
.command-block.protocol-redis {
  border-color: #dc382c;
}

.command-block.protocol-redis .command-input {
  font-family: var(--font-mono);
  color: #ff6b6b;
}

.command-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-sm) var(--sp-md);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  font-size: var(--fs-sm);
  color: var(--text-secondary);
}

.command-icon {
  font-size: var(--fs-md);
}

.command-label {
  font-weight: 500;
  color: var(--text-primary);
}

.command-protocol-badge {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  background: var(--bg-elevated);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
}

.command-status {
  margin-left: auto;
  font-size: var(--fs-xs);
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
}

.command-status.executing {
  color: var(--warning, #f59e0b);
}

.command-status.completed {
  color: var(--success, #10b981);
}

.command-status.failed {
  color: var(--danger, #ef4444);
}

.spinner {
  display: inline-block;
  width: 12px;
  height: 12px;
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Unbound state */
.command-unbound {
  padding: var(--sp-md);
}

/* Bound state */
.command-bound {
  padding: var(--sp-sm) var(--sp-md);
}

.command-input-wrapper {
  display: flex;
  align-items: flex-start;
  gap: var(--sp-sm);
}

.command-prompt {
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  color: var(--text-muted);
  padding-top: 6px;
  user-select: none;
  flex-shrink: 0;
}

.command-input {
  flex: 1;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  line-height: var(--lh-relaxed);
  resize: none;
  outline: none;
  min-height: 24px;
  padding: 4px 0;
}

.command-input::placeholder {
  color: var(--text-muted);
  font-style: italic;
}

.command-actions {
  display: flex;
  justify-content: flex-end;
  padding-top: var(--sp-xs);
}

.command-run-btn {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  padding: var(--sp-xs) var(--sp-md);
  border: 1px solid var(--accent);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--accent);
  font-size: var(--fs-xs);
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.command-run-btn:hover:not(:disabled) {
  background: var(--accent);
  color: white;
}

.command-run-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* History toggle */
.command-history-toggle {
  padding: var(--sp-xs) var(--sp-md);
  border-top: 1px solid var(--border);
}

.history-toggle-btn {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  padding: var(--sp-xs) var(--sp-sm);
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: var(--fs-xs);
  cursor: pointer;
  transition: color var(--transition-fast);
}

.history-toggle-btn:hover,
.history-toggle-btn.active {
  color: var(--accent);
}
</style>
