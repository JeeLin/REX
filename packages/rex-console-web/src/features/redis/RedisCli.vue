<script setup lang="ts">
import { ref } from 'vue'
import { runCommand } from '@/api/redis'

const props = defineProps<{ sessionId: string }>()

const input = ref('')
const history = ref<string[]>([])
const log = ref<string[]>([])
let historyIdx = -1

async function execute() {
  const cmd = input.value.trim()
  if (!cmd) return
  history.value.push(cmd)
  historyIdx = history.value.length
  log.value.push(`> ${cmd}`)

  const args = cmd.split(/\s+/)
  try {
    const result = await runCommand(props.sessionId, args)
    log.value.push(result)
  } catch (e: unknown) {
    log.value.push(`(error) ${e instanceof Error ? e.message : e}`)
  }
  input.value = ''
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (historyIdx > 0) {
      historyIdx--
      input.value = history.value[historyIdx] || ''
    }
  } else if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (historyIdx < history.value.length - 1) {
      historyIdx++
      input.value = history.value[historyIdx] || ''
    } else {
      historyIdx = history.value.length
      input.value = ''
    }
  } else if (e.key === 'l' && e.ctrlKey) {
    e.preventDefault()
    log.value = []
  }
}
</script>

<template>
  <div class="redis-cli">
    <div class="redis-cli-log mono">
      <div v-for="(line, i) in log" :key="i" class="cli-line">{{ line }}</div>
    </div>
    <div class="redis-cli-input">
      <span class="cli-prompt">redis&gt;</span>
      <input
        v-model="input"
        class="cli-input mono"
        placeholder="Enter command..."
        @keydown="onKeydown"
        @keydown.enter="execute"
      />
    </div>
  </div>
</template>

<style scoped>
.redis-cli {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-deep);
}

.redis-cli-log {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.cli-line {
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.5;
}

.redis-cli-input {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-top: 1px solid var(--border);
  background: var(--bg-surface);
}

.cli-prompt {
  color: var(--accent);
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  white-space: nowrap;
}

.cli-input {
  flex: 1;
  background: none;
  border: none;
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}
</style>
