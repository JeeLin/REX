<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useEnvironmentsStore } from '@/stores/environments'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const router = useRouter()
const { t, locale } = useI18n()
const environmentsStore = useEnvironmentsStore()

const query = ref('')
const selectedIndex = ref(0)
const inputRef = ref<HTMLInputElement | null>(null)

interface Command {
  id: string
  label: string
  icon: string
  category: 'command' | 'environment' | 'setting'
  action?: () => void
}

const commands = computed<Command[]>(() => {
  const cmds: Command[] = []

  // Static commands
  cmds.push(
    { id: 'workspace', label: t('commandPalette.workspace'), icon: '🖥️', category: 'command', action: () => { router.push('/workspace'); emit('close') } },
    { id: 'new-connection', label: t('commandPalette.newConnection'), icon: '📡', category: 'command', action: () => { router.push('/resource-new'); emit('close') } },
    { id: 'dashboard', label: t('nav.dashboard'), icon: '📊', category: 'command', action: () => { router.push('/dashboard'); emit('close') } },
    { id: 'environments', label: t('nav.environments'), icon: '🌍', category: 'command', action: () => { router.push('/environments'); emit('close') } },
    { id: 'agents', label: t('nav.agents'), icon: '🤖', category: 'command', action: () => { router.push('/agents'); emit('close') } },
    { id: 'audit-log', label: t('nav.auditLog'), icon: '📋', category: 'command', action: () => { router.push('/audit-log'); emit('close') } },
    { id: 'settings', label: t('nav.settings'), icon: '⚙️', category: 'command', action: () => { router.push('/settings'); emit('close') } },
  )

  // Environments from store
  for (const env of environmentsStore.environments) {
    cmds.push({
      id: `env-${env.id}`,
      label: env.name,
      icon: '🌍',
      category: 'environment',
      action: () => { router.push(`/environments/${env.id}`); emit('close') }
    })
  }

  // Settings commands
  cmds.push(
    { id: 'theme-dark', label: t('commandPalette.themeDark'), icon: '🎨', category: 'setting', action: () => { localStorage.setItem('rex-theme', 'dark'); document.documentElement.dataset.theme = undefined; emit('close') } },
    { id: 'theme-light', label: t('commandPalette.themeLight'), icon: '🎨', category: 'setting', action: () => { localStorage.setItem('rex-theme', 'light'); document.documentElement.dataset.theme = 'light'; emit('close') } },
    { id: 'language-en', label: t('commandPalette.languageEn'), icon: '🌐', category: 'setting', action: () => { locale.value = 'en'; localStorage.setItem('rex-lang', 'en'); emit('close') } },
    { id: 'language-zh', label: t('commandPalette.languageZh'), icon: '🌐', category: 'setting', action: () => { locale.value = 'zh'; localStorage.setItem('rex-lang', 'zh'); emit('close') } },
  )

  return cmds
})

const filteredCommands = computed(() => {
  if (!query.value) return commands.value
  const q = query.value.toLowerCase()
  return commands.value.filter(cmd =>
    cmd.label.toLowerCase().includes(q) ||
    cmd.category.toLowerCase().includes(q)
  )
})

const groupedCommands = computed(() => {
  const groups: Record<string, Command[]> = {}
  const categoryLabels: Record<string, string> = {
    command: t('commandPalette.commands'),
    environment: t('commandPalette.environments'),
    setting: t('commandPalette.settings'),
  }
  for (const cmd of filteredCommands.value) {
    const category = categoryLabels[cmd.category] || cmd.category
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
  if (!props.visible) return
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
    // Fetch environments if not already loaded
    if (environmentsStore.environments.length === 0) {
      environmentsStore.fetchEnvironments()
    }
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
            :placeholder="t('commandPalette.searchPlaceholder')"
          />
        </div>
        <div class="palette-body">
          <div v-if="filteredCommands.length === 0" class="palette-empty">
            {{ t('commandPalette.noResults') }}
          </div>
          <template v-for="(cmds, category) in groupedCommands" :key="category">
            <div class="palette-category">{{ category }}</div>
            <div
              v-for="cmd in cmds"
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
