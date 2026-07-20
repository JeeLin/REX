<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { useRouter } from 'vue-router'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const router = useRouter()
const query = ref('')
const selectedIndex = ref(0)
const inputRef = ref<HTMLInputElement | null>(null)

interface Command {
  id: string
  label: string
  icon: string
  category: 'resource' | 'command' | 'setting'
  action?: () => void
}

// Mock data - in production this would come from stores
const commands: Command[] = [
  { id: 'new-connection', label: 'New Connection', icon: '📡', category: 'command', action: () => { router.push('/resource-new'); emit('close') } },
  { id: 'new-tab', label: 'New Tab', icon: '📄', category: 'command' },
  { id: 'settings', label: 'Settings', icon: '⚙️', category: 'command', action: () => { router.push('/settings'); emit('close') } },
  { id: 'dashboard', label: 'Dashboard', icon: '📊', category: 'command', action: () => { router.push('/dashboard'); emit('close') } },
  { id: 'environments', label: 'Environments', icon: '🌍', category: 'command', action: () => { router.push('/environments'); emit('close') } },
  { id: 'agents', label: 'Agents', icon: '🤖', category: 'command', action: () => { router.push('/agents'); emit('close') } },
  { id: 'audit-log', label: 'Audit Log', icon: '📋', category: 'command', action: () => { router.push('/audit-log'); emit('close') } },
  { id: 'theme-dark', label: 'Theme: Dark', icon: '🎨', category: 'setting' },
  { id: 'theme-light', label: 'Theme: Light', icon: '🎨', category: 'setting' },
  { id: 'language-en', label: 'Language: English', icon: '🌐', category: 'setting' },
  { id: 'language-zh', label: 'Language: 中文', icon: '🌐', category: 'setting' },
]

const filteredCommands = computed(() => {
  if (!query.value) return commands
  const q = query.value.toLowerCase()
  return commands.filter(cmd =>
    cmd.label.toLowerCase().includes(q) ||
    cmd.category.toLowerCase().includes(q)
  )
})

const groupedCommands = computed(() => {
  const groups: Record<string, Command[]> = {}
  for (const cmd of filteredCommands.value) {
    const category = cmd.category === 'resource' ? 'Resources' :
                     cmd.category === 'command' ? 'Commands' : 'Settings'
    if (!groups[category]) groups[category] = []
    groups[category].push(cmd)
  }
  return groups
})

function executeCommand(cmd: Command) {
  if (cmd.action) {
    cmd.action()
  } else {
    emit('close')
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectedIndex.value = Math.min(selectedIndex.value + 1, filteredCommands.value.length - 1)
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const cmd = filteredCommands.value[selectedIndex.value]
    if (cmd) executeCommand(cmd)
  } else if (e.key === 'Escape') {
    emit('close')
  }
}

watch(query, () => {
  selectedIndex.value = 0
})

watch(() => props.visible, (v) => {
  if (v) {
    query.value = ''
    selectedIndex.value = 0
    setTimeout(() => inputRef.value?.focus(), 50)
  }
})

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="palette-overlay" @click.self="emit('close')">
      <div class="palette">
        <div class="palette-header">
          <span class="palette-icon">🔍</span>
          <input
            ref="inputRef"
            v-model="query"
            class="palette-input"
            placeholder="Search resources, commands, settings..."
          />
        </div>
        <div class="palette-body">
          <div v-if="filteredCommands.length === 0" class="palette-empty">
            No results found
          </div>
          <template v-for="(cmds, category) in groupedCommands" :key="category">
            <div class="palette-category">{{ category }}</div>
            <div
              v-for="(cmd, i) in cmds"
              :key="cmd.id"
              class="palette-item"
              :class="{ 'palette-item--selected': filteredCommands.indexOf(cmd) === selectedIndex }"
              @click="executeCommand(cmd)"
              @mouseenter="selectedIndex = filteredCommands.indexOf(cmd)"
            >
              <span class="palette-item-icon">{{ cmd.icon }}</span>
              <span class="palette-item-label">{{ cmd.label }}</span>
            </div>
          </template>
        </div>
        <div class="palette-footer">
          <span>↑↓ Navigate</span>
          <span>↵ Select</span>
          <span>esc Close</span>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.palette-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 20vh;
  z-index: 2000;
}

.palette {
  width: 500px;
  max-width: 90vw;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.palette-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}

.palette-icon {
  font-size: var(--text-lg);
  color: var(--text-muted);
}

.palette-input {
  flex: 1;
  background: none;
  border: none;
  color: var(--text-primary);
  font-size: var(--text-md);
  outline: none;
}

.palette-input::placeholder {
  color: var(--text-muted);
}

.palette-body {
  max-height: 400px;
  overflow-y: auto;
}

.palette-empty {
  padding: var(--space-8);
  text-align: center;
  color: var(--text-muted);
}

.palette-category {
  padding: var(--space-2) var(--space-4);
  font-size: var(--text-xs);
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  background: var(--bg-surface);
}

.palette-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-4);
  cursor: pointer;
  transition: background var(--transition);
}

.palette-item:hover,
.palette-item--selected {
  background: var(--bg-hover);
}

.palette-item-icon {
  font-size: var(--text-md);
}

.palette-item-label {
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.palette-footer {
  display: flex;
  gap: var(--space-4);
  padding: var(--space-2) var(--space-4);
  border-top: 1px solid var(--border);
  font-size: var(--text-xs);
  color: var(--text-muted);
}
</style>
